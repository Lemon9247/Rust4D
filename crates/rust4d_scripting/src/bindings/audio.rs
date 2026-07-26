//! Audio bindings for Lua — live `AudioEngine4D` bridge
//!
//! Reads an optional [`AudioRef`] registered into `app_data` by `ScriptSystem`.
//! When audio is disabled or unavailable (no device), every call is a no-op
//! logged at `trace` level, so scripts run identically on headless/CI hosts.
//!
//! # Audio Buses
//!
//! `"master"`, `"sfx"`, `"music"`, `"ambient"` (case-insensitive).

use mlua::prelude::*;
use rust4d_audio::{AudioBus, AudioEngine4D, SoundHandle, SpatialConfig};

use super::math::LuaVec4;
use crate::context::AudioRef;

/// Lua wrapper for a sound handle.
///
/// Wraps a `rust4d_audio::SoundHandle` (the sound data lives in the
/// `AudioEngine4D`). It is `Copy`, so scripts can store and replay handles.
#[derive(Clone, Copy, Debug)]
pub struct LuaSoundHandle {
    sound: SoundHandle,
}

impl LuaSoundHandle {
    pub fn new(sound: SoundHandle) -> Self {
        Self { sound }
    }

    pub fn id(&self) -> u64 {
        self.sound.id()
    }
}

impl LuaUserData for LuaSoundHandle {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.sound.id()));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaSoundHandle| {
            Ok(this.sound == other.sound)
        });
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| {
            Ok(format!("SoundHandle({})", this.sound.id()))
        });
    }
}

impl FromLua for LuaSoundHandle {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::UserData(ud) => ud.borrow::<LuaSoundHandle>().map(|h| *h),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "SoundHandle".to_string(),
                message: Some("expected SoundHandle userdata".to_string()),
            }),
        }
    }
}

/// Parse and validate a bus name string into an `AudioBus`.
fn parse_bus(name: &str) -> LuaResult<AudioBus> {
    match name.to_lowercase().as_str() {
        "master" => Ok(AudioBus::Master),
        "sfx" => Ok(AudioBus::Sfx),
        "music" => Ok(AudioBus::Music),
        "ambient" => Ok(AudioBus::Ambient),
        _ => Err(LuaError::RuntimeError(format!(
            "Invalid audio bus '{}'. Valid buses: master, sfx, music, ambient",
            name
        ))),
    }
}

/// Convert an `AudioError` into a Lua runtime error.
fn audio_err(e: rust4d_audio::AudioError) -> LuaError {
    LuaError::RuntimeError(format!("audio error: {}", e))
}

/// Run `f` against the live audio engine, or no-op (return `default`) when
/// audio is unavailable.
fn with_audio<F, R>(lua: &Lua, default: R, f: F) -> LuaResult<R>
where
    F: FnOnce(&mut AudioEngine4D) -> LuaResult<R>,
{
    let ptr_opt = match lua.app_data_ref::<AudioRef>() {
        Some(r) => r.0,
        None => {
            log::trace!("[audio] no AudioRef registered; no-op");
            return Ok(default);
        }
    };
    let Some(ptr) = ptr_opt else {
        log::trace!("[audio] audio disabled; no-op");
        return Ok(default);
    };
    // SAFETY: ScriptSystem registers the AudioRef for the callback's lifetime
    // and does not touch the engine while the callback runs.
    let engine = unsafe { &mut *ptr };
    f(engine)
}

/// Register audio bindings with the Lua VM.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let audio_table = lua.create_table()?;

    audio_table.set(
        "load_sound",
        lua.create_function(|lua, path: String| {
            with_audio(lua, Option::<LuaSoundHandle>::None, |engine| {
                let sound = engine.load_sound(&path).map_err(audio_err)?;
                Ok(Some(LuaSoundHandle::new(sound)))
            })
        })?,
    )?;

    audio_table.set(
        "play",
        lua.create_function(|lua, (handle, bus): (LuaSoundHandle, String)| {
            let bus = parse_bus(&bus)?;
            with_audio(lua, (), |engine| {
                engine.play(&handle.sound, bus).map_err(audio_err)
            })
        })?,
    )?;

    audio_table.set(
        "play_oneshot",
        lua.create_function(|lua, (handle, bus): (LuaSoundHandle, String)| {
            let bus = parse_bus(&bus)?;
            with_audio(lua, (), |engine| {
                engine.play_oneshot(&handle.sound, bus).map_err(audio_err)
            })
        })?,
    )?;

    audio_table.set(
        "play_spatial",
        lua.create_function(
            |lua, (handle, pos, bus): (LuaSoundHandle, LuaVec4, String)| {
                let bus = parse_bus(&bus)?;
                with_audio(lua, (), |engine| {
                    let config = SpatialConfig::new(pos.0);
                    engine
                        .play_spatial(&handle.sound, config, bus)
                        .map_err(audio_err)
                })
            },
        )?,
    )?;

    audio_table.set(
        "play_oneshot_spatial",
        lua.create_function(
            |lua,
             (handle, pos, min_dist, max_dist, bus): (
                LuaSoundHandle,
                LuaVec4,
                f32,
                f32,
                String,
            )| {
                let bus = parse_bus(&bus)?;
                if min_dist < 0.0 {
                    return Err(LuaError::RuntimeError("min_dist must be >= 0".into()));
                }
                if max_dist <= min_dist {
                    return Err(LuaError::RuntimeError("max_dist must be > min_dist".into()));
                }
                with_audio(lua, (), |engine| {
                    let config = SpatialConfig::new(pos.0)
                        .with_min_distance(min_dist)
                        .with_max_distance(max_dist);
                    engine
                        .play_oneshot_spatial(&handle.sound, config, bus)
                        .map_err(audio_err)
                })
            },
        )?,
    )?;

    audio_table.set(
        "set_volume",
        lua.create_function(|lua, (bus, volume): (String, f32)| {
            let bus = parse_bus(&bus)?;
            let clamped = volume.clamp(0.0, 1.0);
            if clamped != volume {
                log::warn!("[audio] Volume {} clamped to {}", volume, clamped);
            }
            with_audio(lua, (), |engine| {
                engine.set_bus_volume(bus, clamped);
                Ok(())
            })
        })?,
    )?;

    audio_table.set(
        "stop_all",
        lua.create_function(|lua, ()| {
            with_audio(lua, (), |engine| {
                engine.stop_all();
                Ok(())
            })
        })?,
    )?;

    audio_table.set(
        "stop_bus",
        lua.create_function(|lua, bus: String| {
            let bus = parse_bus(&bus)?;
            with_audio(lua, (), |engine| {
                engine.stop_bus(bus);
                Ok(())
            })
        })?,
    )?;

    audio_table.set(
        "update_listener",
        lua.create_function(|lua, pos: LuaVec4| {
            with_audio(lua, (), |engine| {
                engine.update_listener(pos.0);
                Ok(())
            })
        })?,
    )?;

    audio_table.set(
        "get_listener_position",
        lua.create_function(|lua, ()| {
            with_audio(lua, LuaVec4(rust4d_math::Vec4::ZERO), |engine| {
                Ok(LuaVec4(engine.listener_position()))
            })
        })?,
    )?;

    lua.globals().set("audio", audio_table)?;

    log::debug!("[audio] Audio bindings registered (live AudioEngine4D bridge)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::math;

    fn create_lua_with_audio() -> Lua {
        let lua = Lua::new();
        math::register(&lua).expect("Failed to register math bindings");
        register(&lua).expect("Failed to register audio bindings");
        lua
    }

    #[test]
    fn test_audio_table_exists() {
        let lua = create_lua_with_audio();
        let audio: LuaTable = lua
            .globals()
            .get("audio")
            .expect("audio table should exist");
        for key in [
            "load_sound",
            "play",
            "play_oneshot",
            "play_spatial",
            "play_oneshot_spatial",
            "set_volume",
            "stop_all",
            "stop_bus",
            "update_listener",
            "get_listener_position",
        ] {
            assert!(audio.contains_key(key).unwrap());
        }
    }

    #[test]
    fn test_noop_without_engine() {
        // No AudioRef registered -> load_sound returns nil; other calls no-op.
        let lua = create_lua_with_audio();
        let val: LuaValue = lua
            .load(r#"return audio.load_sound("test.ogg")"#)
            .eval()
            .unwrap();
        assert!(val.is_nil(), "load_sound returns nil without an engine");
        lua.load(r#"audio.stop_all()"#).exec().unwrap();
        lua.load(r#"audio.update_listener(Vec4.new(1,2,3,4))"#)
            .exec()
            .unwrap();
        let pos: LuaVec4 = lua
            .load("return audio.get_listener_position()")
            .eval()
            .unwrap();
        assert_eq!(pos.0, rust4d_math::Vec4::ZERO);
    }

    #[test]
    fn test_invalid_bus_errors() {
        let lua = create_lua_with_audio();
        assert!(lua.load("audio.stop_bus('nope')").eval::<()>().is_err());
        assert!(lua
            .load("audio.set_volume('nope', 0.5)")
            .eval::<()>()
            .is_err());
    }

    #[test]
    fn test_set_volume_clamps_without_error() {
        let lua = create_lua_with_audio();
        lua.load(r#"audio.set_volume("sfx", 2.0)"#).exec().unwrap();
        lua.load(r#"audio.set_volume("sfx", -1.0)"#).exec().unwrap();
    }

    #[test]
    fn test_spatial_distance_validation() {
        let lua = create_lua_with_audio();
        // nil handle + invalid distances still error before reaching the engine.
        assert!(
            lua.load(r#"audio.play_oneshot_spatial(nil, Vec4.new(0,0,0,0), -1.0, 50.0, "sfx")"#)
                .eval::<()>()
                .is_err()
                || lua
                    .load(
                        r#"audio.play_oneshot_spatial(nil, Vec4.new(0,0,0,0), -1.0, 50.0, "sfx")"#
                    )
                    .eval::<()>()
                    .is_err()
        );
        assert!(lua
            .load(r#"audio.play_oneshot_spatial(nil, Vec4.new(0,0,0,0), 10.0, 5.0, "sfx")"#)
            .eval::<()>()
            .is_err());
    }
}
