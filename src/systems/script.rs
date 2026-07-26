//! Scripting + audio system
//!
//! Owns the [`ScriptEngine`] and an optional [`AudioEngine4D`] and drives their
//! lifecycle callbacks from the main loop. [`ScriptSystem::update`] runs *before*
//! [`SimulationSystem`](super::simulation::SimulationSystem) so scripts can set
//! velocities/transforms before physics steps.
//!
//! # Per-call app_data pattern
//!
//! The `ScriptEngine` does not own or borrow the `hecs::World`; instead, each
//! lifecycle call registers the live `&mut World`, the current input snapshot,
//! and an optional audio handle into the Lua VM's `app_data` for the duration of
//! the call, then clears it. This keeps borrows inside the call boundary (see
//! the Wave 5 plan: "pass `&mut hecs::World` into the call, do not store it").
//! The ECS bindings read the world back via [`WorldRef`] (`bindings::ecs`).

use std::time::Instant;

use rust4d_audio::{AudioBus, AudioEngine4D};
use rust4d_core::World;
use rust4d_math::Vec4;
use rust4d_physics::PhysicsConfig;
use rust4d_scripting::{InputSnapshot, ScriptConfig, ScriptEngine, ScriptError};

use crate::config::{AudioConfig, GameConfig};

/// Fixed timestep for scripted `on_fixed_update` callbacks.
const FIXED_DT: f32 = 1.0 / 60.0;

/// Result of a script update frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScriptUpdateResult {
    /// Whether scripts mutated the world this frame (spawn/despawn/component
    /// writes) and the geometry cache should be rebuilt before rendering.
    pub geometry_dirty: bool,
}

/// Scripting + audio system driving Lua lifecycle callbacks each frame.
pub struct ScriptSystem {
    engine: Option<ScriptEngine>,
    audio: Option<AudioEngine4D>,
    physics: PhysicsConfig,
    fixed_accumulator: f32,
    last_frame: Instant,
    scripts_inited: bool,
}

impl ScriptSystem {
    /// Construct the system from configuration.
    ///
    /// Scripting is disabled (engine = `None`) when `game.game_dir` is empty or
    /// script loading fails. Audio is `None` when disabled or when no audio
    /// device is available; both degrade gracefully.
    pub fn new(game: &GameConfig, audio: &AudioConfig, physics: &PhysicsConfig) -> Self {
        let audio_engine = init_audio(game, audio);
        let engine = init_script_engine(game);

        Self {
            engine,
            audio: audio_engine,
            physics: physics.clone(),
            fixed_accumulator: 0.0,
            last_frame: Instant::now(),
            scripts_inited: false,
        }
    }

    /// Whether a script engine is active.
    pub fn is_active(&self) -> bool {
        self.engine.is_some()
    }

    /// Whether the audio engine is active.
    pub fn has_audio(&self) -> bool {
        self.audio.is_some()
    }

    /// Call `on_init` once at startup with the initial world + player position.
    pub fn init(&mut self, world: &mut World, player: Vec4) -> Result<(), ScriptError> {
        let Some(engine) = &self.engine else {
            return Ok(());
        };
        if self.scripts_inited {
            return Ok(());
        }
        self.scripts_inited = true;

        log::info!(
            "[script] on_init: world has {} entities; player at {:?}",
            world.entity_count(),
            player
        );

        if let Some(audio) = &mut self.audio {
            audio.update_listener(player);
        }

        engine.call_init()
    }

    /// Call `on_update(dt)` for the frame and drain the fixed-step accumulator
    /// into `on_fixed_update`. Runs before `SimulationSystem::update`.
    pub fn update(
        &mut self,
        world: &mut World,
        input: &InputSnapshot,
        listener_pos: Vec4,
    ) -> Result<ScriptUpdateResult, ScriptError> {
        let now = Instant::now();
        let raw_dt = (now - self.last_frame).as_secs_f32();
        // Cap dt to prevent spiral-of-death on first frame / window focus.
        let dt = raw_dt.min(0.25);
        self.last_frame = now;

        let Some(engine) = &mut self.engine else {
            // Still keep the audio listener fresh even without scripts.
            if let Some(audio) = &mut self.audio {
                audio.update_listener(listener_pos);
            }
            return Ok(ScriptUpdateResult::default());
        };

        engine.set_world(world);
        engine.set_input(input);
        engine.set_audio(self.audio.as_ref());
        engine.set_physics_config_raw(self.physics.clone());

        let update_res = engine.call_update(dt);

        // Drain the fixed-step accumulator.
        self.fixed_accumulator += dt;
        while self.fixed_accumulator >= FIXED_DT {
            self.fixed_accumulator -= FIXED_DT;
            if let Err(e) = engine.call_fixed_update(FIXED_DT) {
                log::error!("[script] on_fixed_update error: {}", e);
                break;
            }
        }

        // Hot-reload polling (no-op when disabled).
        engine.check_hot_reload();

        let geometry_dirty = engine.clear_world();
        engine.clear_input();
        engine.clear_audio();
        engine.clear_physics_config();

        if let Err(e) = update_res {
            log::error!("[script] on_update error: {}", e);
        }

        if let Some(audio) = &mut self.audio {
            audio.update_listener(listener_pos);
            audio.cleanup_finished_sounds();
        }

        Ok(ScriptUpdateResult { geometry_dirty })
    }

    /// Call `on_shutdown` before the engine exits.
    pub fn shutdown(&mut self) -> Result<(), ScriptError> {
        let Some(engine) = &self.engine else {
            return Ok(());
        };
        if !self.scripts_inited {
            return Ok(());
        }
        engine.call_shutdown()
    }

    /// Update the audio listener position without running scripts (used when
    /// scripting is disabled but audio is active).
    pub fn update_listener(&mut self, pos: Vec4) {
        if let Some(audio) = &mut self.audio {
            audio.update_listener(pos);
        }
    }
}

fn init_audio(game: &GameConfig, audio: &AudioConfig) -> Option<AudioEngine4D> {
    if !game.audio_enabled {
        return None;
    }
    match AudioEngine4D::new() {
        Ok(mut eng) => {
            eng.set_bus_volume(AudioBus::Master, audio.master_volume);
            eng.set_bus_volume(AudioBus::Sfx, audio.sfx_volume);
            eng.set_bus_volume(AudioBus::Music, audio.music_volume);
            eng.set_bus_volume(AudioBus::Ambient, audio.ambient_volume);
            log::info!("[audio] AudioEngine4D initialized");
            Some(eng)
        }
        Err(e) => {
            log::warn!("[audio] Audio init failed; degrading to silent: {}", e);
            None
        }
    }
}

fn init_script_engine(game: &GameConfig) -> Option<ScriptEngine> {
    if game.game_dir.is_empty() {
        log::info!("[script] No game_dir configured; scripting disabled.");
        return None;
    }
    let scripts_dir = resolve_scripts_dir(&game.game_dir, &game.scripts_dir);
    let config = ScriptConfig {
        scripts_dir,
        hot_reload: game.hot_reload,
        ..Default::default()
    };
    let mut engine = match ScriptEngine::new(config) {
        Ok(engine) => engine,
        Err(e) => {
            log::error!("[script] ScriptEngine init failed: {}", e);
            return None;
        }
    };
    if let Err(e) = engine.load_game() {
        log::error!(
            "[script] Failed to load game scripts from '{}': {}",
            game.game_dir,
            e
        );
        return None;
    }
    log::info!("[script] Game scripts loaded from '{}'", game.game_dir);
    Some(engine)
}

/// Resolve the effective scripts directory.
///
/// - `scripts_dir` empty -> `game_dir`
/// - `scripts_dir` relative -> `game_dir` joined with `scripts_dir`
/// - `scripts_dir` absolute -> `scripts_dir` as-is
pub fn resolve_scripts_dir(game_dir: &str, scripts_dir: &str) -> String {
    if scripts_dir.is_empty() {
        return game_dir.to_string();
    }
    let scripts_path = std::path::Path::new(scripts_dir);
    if scripts_path.is_absolute() {
        return scripts_dir.to_string();
    }
    if game_dir.is_empty() {
        return scripts_dir.to_string();
    }
    let mut path = std::path::PathBuf::from(game_dir);
    path.push(scripts_dir);
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_scripts_dir_empty() {
        assert_eq!(resolve_scripts_dir("games/trivial", ""), "games/trivial");
    }

    #[test]
    fn test_resolve_scripts_dir_relative() {
        assert_eq!(
            resolve_scripts_dir("games/demo", "scripts"),
            "games/demo/scripts"
        );
    }

    #[test]
    fn test_resolve_scripts_dir_absolute() {
        assert_eq!(
            resolve_scripts_dir("games/demo", "/absolute/scripts"),
            "/absolute/scripts"
        );
    }

    #[test]
    fn test_resolve_scripts_dir_no_game() {
        assert_eq!(resolve_scripts_dir("", "scripts"), "scripts");
    }

    #[test]
    fn test_script_system_disabled_without_game_dir() {
        // No game_dir + no audio device in CI -> both disabled, must not panic.
        let game = GameConfig {
            game_dir: String::new(),
            audio_enabled: false,
            ..Default::default()
        };
        let audio = AudioConfig::default();
        let physics = PhysicsConfig::default();
        let sys = ScriptSystem::new(&game, &audio, &physics);
        assert!(!sys.is_active());
    }
}
