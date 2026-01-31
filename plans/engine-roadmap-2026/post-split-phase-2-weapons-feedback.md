# Post-Split Phase 2: Weapons & Feedback -- Engine Implementation Plan

**Source**: Agent P2 report from engine roadmap planning swarm (2026-01-30)
**Status**: Planning document -- not yet started
**Estimated Sessions**: 5.5-7.0 (engine-side, includes Lua bindings)
**Prerequisites**: Engine/game split complete, `rust4d_game` crate exists, Foundation phase done, `rust4d_scripting` crate exists (mlua + Lua 5.4)

*Updated 2026-01-31: Integrated Lua scripting amendments. This plan now includes all Lua binding work for Phase 2 APIs. The core Rust implementations are unchanged; new work consists of Lua API wrappers for audio, particles, screen effects, and a new HUD drawing API that bridges Lua to egui.*

---

## 1. Overview

Phase 2 is where combat starts to *feel* good. The cross-swarm synthesis identified four deliverables for this phase: weapon system, HUD, audio, and screen effects. After applying the engine/game split boundary, the engine work decomposes into:

- **Audio system**: New `rust4d_audio` crate wrapping kira with 4D spatial audio
- **HUD/overlay rendering**: egui-wgpu integration in `rust4d_render` via `OverlayRenderer`
- **Particle/effect system**: CPU-simulated, GPU-rendered billboard particles in `rust4d_render`
- **Screen effects**: `ScreenShake` and `TimedEffect` helpers in `rust4d_game`, depth texture exposure in `rust4d_render`
- **Lua bindings**: Wrappers exposing audio, particles, screen effects, and a new HUD drawing API to Lua scripts

The weapon system itself is **100% game-side** (now Lua scripts rather than Rust game code). The engine provides no weapon abstractions. The game scripts build weapons on top of raycasting (P1), audio (P2-A), particles (P2-C), and the HUD (P2-B).

---

## 2. Engine vs Game Boundary

### Engine Responsibilities (this plan)

| Subsystem | Crate | What the Engine Provides |
|-----------|-------|--------------------------|
| Audio | `rust4d_audio` (NEW) | AudioEngine4D, 4D spatial audio, bus/mixer, sound loading |
| HUD/Overlay | `rust4d_render` | OverlayRenderer (egui-wgpu integration), egui context lifecycle |
| Particles | `rust4d_render` | ParticleSystem, ParticleEmitter, billboard renderer, blend modes |
| Screen Effects | `rust4d_game` + `rust4d_render` | ScreenShake, TimedEffect, depth texture getter |
| Lua Audio API | `rust4d_scripting` | String-keyed sound loading/playback, bus control wrappers |
| Lua Particle API | `rust4d_scripting` | Named presets, burst/emit/stop wrappers |
| Lua Screen Effects API | `rust4d_scripting` | Screen shake and flash wrappers |
| Lua HUD Drawing API | `rust4d_scripting` + `rust4d_render` | Simplified draw commands (text, bar, rect, image, crosshair) bridging Lua to egui |

### Game Responsibilities (NOT in this plan -- now Lua scripts)

| Feature | What the Game Builds (in Lua) |
|---------|-------------------------------|
| Weapon system | Hitscan shotgun, projectile rocket launcher, ammo, weapon switching |
| HUD layout | Health bar, ammo counter, crosshair, W-position indicator via `hud:draw_*()` calls |
| Sound triggering | `audio:play_oneshot("shotgun_fire", "sfx")` -- one-line calls, no struct needed |
| Sound assets | .wav/.ogg files for all game sounds |
| Music | Playlists, ambient soundscapes, level music via `audio:play()` |
| Effect presets | Muzzle flash, blood splatter, explosion configs as Lua data tables |
| Damage flash | `effects:flash(255, 0, 0, 128, 0.2)` or `hud:draw_rect()` |
| Muzzle flash lighting | Brief ambient_strength boost in RenderUniforms |

### Engine vs Game Boundary Shift (Lua Migration)

With the shift from "game in Rust" to "game in Lua scripts," several patterns that were previously game-side Rust now require Lua bindings in the engine:

| Was (Rust game code) | Becomes (Engine Lua binding) |
|---------------------|------------------------------|
| `audio_engine.play(&sound, AudioBus::Sfx)` | `audio:play("shotgun_fire", "sfx")` |
| `audio_engine.play_spatial(&sound, pos, bus, spatial)` | `audio:play_spatial("explosion", position, "sfx", { min_dist=1, max_dist=50 })` |
| `audio_engine.play_oneshot(&sound, bus)` | `audio:play_oneshot("pickup", "sfx")` |
| `audio_engine.play_oneshot_spatial(&sound, pos, bus, spatial)` | `audio:play_oneshot_spatial("bullet_impact", pos, "sfx")` |
| `audio_engine.set_bus_volume(AudioBus::Music, 0.5)` | `audio:set_volume("music", 0.5)` |
| `audio_engine.stop_bus(AudioBus::Sfx)` | `audio:stop("sfx")` |
| `particle_system.spawn_burst(pos, &config)` | `particles:burst(position, "muzzle_flash")` with named presets |
| `particle_system.spawn_emitter(pos, &config)` | `particles:emit(position, config_table)` |
| `particle_system.stop_emitter(id)` | `particles:stop(emitter_id)` |
| `screen_shake.add_trauma(0.5)` | `effects:screen_shake(0.5)` |
| Game builds HUD via `overlay.ctx()` (egui Context) | **New HUD drawing API** -- `hud:draw_text()`, `hud:draw_bar()`, etc. |
| `overlay.begin_frame() / render()` cycle | Engine manages internally, Lua submits draw commands |

### What Gets Simpler with Lua

- **Sound triggering**: In the Rust approach, the game maintained a `GameAudio` struct with loaded sound handles and matched on event types. In Lua, it is just `audio:play_oneshot("shotgun_fire", "sfx")` -- one line. No struct, no handle management.
- **Particle effect presets**: In Rust, these were `ParticleEmitterConfig` structs defined in compiled code. In Lua, they are data tables that can be defined in config files and hot-reloaded. Easier to tweak without recompilation.
- **Screen shake**: `effects:screen_shake(0.5)` is simpler than creating a `ScreenShake` struct and calling methods on it from Rust.

### What Gets Removed from Engine Scope

- **Game-side `GameAudio` struct pattern**: The Rust example showed a struct holding `AudioEngine4D` + named `SoundHandle` fields. With Lua, the engine manages handles by string name internally. No game-side Rust struct needed.
- **Game-side `GameHud` struct**: The egui-based HUD drawing shown in the original plan was Rust code. This is now Lua code using the HUD drawing API. The Rust `GameHud` struct is gone.
- **`ScreenShake` and `TimedEffect` game-side usage pattern**: The Rust types remain internally (engine uses them), but they get Lua-facing wrappers. The game-side Rust usage pattern is replaced by Lua calls.

Key decisions from Agent P2 (unchanged):
- **Weapon system is 100% game-side.** The engine has no concept of weapons.
- **Screen shake goes in `rust4d_game`** -- it is a camera offset, not post-processing.
- **Damage flash via egui overlay** -- no post-processing pipeline needed for Phase 2.
- **Particles are 3D, not 4D** -- they exist in sliced output space and bypass the compute shader.

---

## 3. Sub-Phase A: Audio System (`rust4d_audio` crate)

### Session Estimate: 1.5-2 sessions (Rust) + 0.25 session (Lua bindings) = 1.75-2.25 sessions

- **Session 1**: AudioEngine4D core (init, load, play, play_oneshot, bus routing), basic tests with mock sounds
- **Session 2 (partial)**: 4D spatial audio (listener updates, W-distance attenuation, W-filtering), spatial tests
- **Lua binding work (0.25 session)**: Audio API wrappers for Lua -- string-keyed sound management, bus control

### Why Kira Over Rodio

**Recommendation: Use [kira](https://crates.io/crates/kira)** (not rodio).

Rationale:
1. **Built-in spatial audio**: Kira has a listener/emitter model with spatial tracks, distance attenuation, and panning. Rodio's `Spatial` source is basic by comparison.
2. **Tweens and transitions**: Built-in tweens for smoothly adjusting volume, pitch, and panning. Essential for game feel -- sounds should fade in/out, not cut abruptly.
3. **Mixer/track system**: Flexible mixer allows creating sub-tracks (SFX, Music, Ambient) with independent volume control and effects. This is the audio bus abstraction games need.
4. **Clock system**: Precise audio event timing for synchronized effects (muzzle flash + sound must be frame-accurate).
5. **Game-focused design**: Kira is explicitly designed for game audio, whereas rodio is general-purpose.
6. **Format support**: MP3, OGG, FLAC, WAV via Symphonia.

The tradeoff is slightly higher complexity, but this is abstracted away by the `rust4d_audio` crate's API.

### 4D Spatial Audio Design

Standard 3D spatial audio uses Euclidean distance in XYZ for attenuation. In 4D, we need to account for the W dimension.

**Approach**: Use 4D Euclidean distance for volume attenuation, plus a W-distance filter.

```
// 4D distance = sqrt(dx^2 + dy^2 + dz^2 + dw^2)
let distance_4d = (listener_pos - emitter_pos).length();

// W-distance for filtering
let w_distance = (listener_pos.w - emitter_pos.w).abs();
```

**W-distance behavior**:
- Sounds within a configurable `w_audible_range` (default 5.0 units) are audible
- Volume attenuates by 4D distance (not just 3D)
- Sounds far in W get low-pass filtered (like hearing through walls/dimensions)
- Sounds outside `w_audible_range` are silent (pragmatic cutoff)

**Mapping onto kira**: The engine computes the effective 3D position for kira's listener system by projecting the 4D distance onto a 3D representation:

```rust
// Project 4D emitter to kira's 3D spatial system
// X,Z = direction in the XZ plane (for left/right/front/back panning)
// Y = height (for vertical panning)
// Distance = 4D euclidean distance (for volume falloff)
```

The W-filtering (low-pass for distant W) uses kira's per-track effects.

### Open question: Should W-distance filtering be a low-pass filter or just volume attenuation? Low-pass filtering requires kira's effect system per spatial track. Volume-only is simpler and might be sufficient for an initial implementation.

### Crate Organization

```
crates/rust4d_audio/
  Cargo.toml          # depends on: kira, rust4d_math, log, serde
  src/
    lib.rs            # Public API re-exports
    audio_engine.rs   # AudioEngine4D wrapper around kira::AudioManager
    listener.rs       # Listener4D (position + orientation in 4D)
    emitter.rs        # SoundEmitter4D (position in 4D + spatial config)
    bus.rs            # AudioBus enum (SFX, Music, Ambient) + volume control
    sound.rs          # SoundHandle, SoundId, sound loading
    config.rs         # Audio configuration (master volume, bus volumes, spatial settings)
```

### Cargo.toml

```toml
[package]
name = "rust4d_audio"
version = "0.1.0"
edition = "2021"

[dependencies]
rust4d_math = { path = "../rust4d_math" }
kira = { version = "0.9", features = ["mp3", "ogg", "flac", "wav"] }
log = "0.4"
serde = { version = "1.0", features = ["derive"] }
```

**Note**: Kira API changed between 0.8 and 0.9. Need to verify the latest stable version when implementing. The spatial track API is what we need.

### API Surface

```rust
// === Core types ===

/// The 4D audio engine, wraps kira::AudioManager
pub struct AudioEngine4D { ... }

/// Configuration for the audio engine
pub struct AudioConfig {
    pub master_volume: f32,          // 0.0 - 1.0
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub ambient_volume: f32,
    pub w_audible_range: f32,        // Max W-distance for audible sounds (default: 5.0)
    pub w_filter_start: f32,         // W-distance where low-pass filtering starts (default: 2.0)
    pub spatial_max_distance: f32,   // Distance at which sounds become inaudible (default: 50.0)
}

/// Handle to a loaded sound asset
pub struct SoundHandle { ... }

/// Handle to a playing sound instance
pub struct PlayingSound { ... }

/// Audio bus categories
pub enum AudioBus {
    Sfx,
    Music,
    Ambient,
}

/// 4D listener (typically attached to the player camera)
pub struct Listener4D {
    pub position: Vec4,
    pub forward: Vec4,     // Camera forward direction
    pub up: Vec4,          // Camera up direction
}

/// Spatial emitter configuration
pub struct SpatialConfig {
    pub min_distance: f32,   // Distance at which sound is at full volume
    pub max_distance: f32,   // Distance at which sound becomes inaudible
    pub w_audible_range: Option<f32>,  // Override global w_audible_range
}

// === AudioEngine4D methods ===

impl AudioEngine4D {
    /// Create a new audio engine with the given config
    pub fn new(config: AudioConfig) -> Result<Self, AudioError>;

    /// Load a sound from a file path
    pub fn load_sound(&mut self, path: &str) -> Result<SoundHandle, AudioError>;

    /// Play a non-spatial sound on a bus (UI sounds, music)
    pub fn play(&mut self, sound: &SoundHandle, bus: AudioBus) -> PlayingSound;

    /// Play a spatial sound at a 4D position
    pub fn play_spatial(
        &mut self,
        sound: &SoundHandle,
        position: Vec4,
        bus: AudioBus,
        spatial: SpatialConfig,
    ) -> PlayingSound;

    /// Play a one-shot sound (fire and forget, no handle returned)
    pub fn play_oneshot(&mut self, sound: &SoundHandle, bus: AudioBus);

    /// Play a one-shot spatial sound
    pub fn play_oneshot_spatial(
        &mut self,
        sound: &SoundHandle,
        position: Vec4,
        bus: AudioBus,
        spatial: SpatialConfig,
    );

    /// Update the listener position (call once per frame)
    pub fn update_listener(&mut self, listener: &Listener4D);

    /// Update a playing sound's position (for moving emitters)
    pub fn update_emitter_position(&mut self, sound: &PlayingSound, position: Vec4);

    /// Set bus volume (with smooth tween)
    pub fn set_bus_volume(&mut self, bus: AudioBus, volume: f32);

    /// Set master volume
    pub fn set_master_volume(&mut self, volume: f32);

    /// Stop all sounds on a bus
    pub fn stop_bus(&mut self, bus: AudioBus);

    /// Stop all sounds
    pub fn stop_all(&mut self);
}
```

### Lua Audio API

The engine exposes audio to Lua via string-keyed sound management. The engine maintains an internal `name -> SoundHandle` mapping, so Lua scripts never deal with Rust handles.

```
audio:load(name, path)                           -- load and register a sound by name
audio:play(name, bus)                            -- play non-spatial
audio:play_spatial(name, position, bus, config?) -- play spatial at 4D position
audio:play_oneshot(name, bus)                    -- fire-and-forget non-spatial
audio:play_oneshot_spatial(name, pos, bus)        -- fire-and-forget spatial
audio:set_volume(bus, volume)                    -- set bus volume (0.0-1.0)
audio:stop(bus?)                                 -- stop sounds on a bus, or all
```

**Lua usage example** (replaces the Rust `GameAudio` struct):

```lua
-- Game script: load sounds at startup
function init()
    audio:load("shotgun_fire", "assets/sounds/shotgun.ogg")
    audio:load("rocket_fire", "assets/sounds/rocket.ogg")
    audio:load("explosion", "assets/sounds/explosion.ogg")
    audio:load("pickup_health", "assets/sounds/pickup.ogg")
end

-- Game script: weapon fire (replaces Rust GameAudio::on_weapon_fire)
function on_weapon_fire(weapon_type, position)
    if weapon_type == "shotgun" then
        audio:play_oneshot_spatial("shotgun_fire", position, "sfx")
    elseif weapon_type == "rocket" then
        audio:play_oneshot_spatial("rocket_fire", position, "sfx")
    end
end
```

### Dependencies

- **Foundation**: Audio config needs serialization (serde) for saving volume settings
- **Phase 1 (P1)**: Event system -- game uses events to trigger audio (damage dealt -> play hit sound). This is a SOFT dependency; game can trigger audio directly without events.
- **ECS split**: AudioEngine4D lives as a resource, not a component. No ECS dependency.
- **`rust4d_scripting`**: Lua bindings require the scripting crate to exist and provide the Lua state for function registration.

### Kira Threading Model

Kira uses a separate audio thread. Commands are sent from the game thread. The `AudioEngine4D` wrapper must be designed to handle this cleanly -- all methods should be non-blocking.

---

## 4. Sub-Phase B: HUD/egui Overlay (`rust4d_render`)

### Session Estimate: 1 session (Rust egui integration) + 0.5-1.0 session (Lua HUD API) = 1.5-2.0 sessions

- **Rust work (1 session)**: Integrate egui-wgpu into the render pipeline, create OverlayRenderer with begin/end frame pattern, forward winit events to egui, test with a simple "Hello World" overlay, document the API pattern
- **Lua HUD API (0.5-1.0 session)**: New HUD drawing abstraction that bridges Lua draw commands to egui -- this is the biggest new item from the Lua migration

### Why egui

1. **Immediate mode**: Perfect for HUD that changes every frame (ammo counts, health bars)
2. **Rich widget set**: Text, sliders, progress bars, windows, panels -- all available immediately
3. **Input handling**: Mouse/keyboard input routing built-in (needed for menus)
4. **Debug UI**: Free debug panels for development (entity inspector, physics debug)
5. **Mature ecosystem**: Widely used in Rust gamedev community
6. **wgpu integration**: `egui-wgpu` is maintained by the egui team, direct wgpu render pass

The tradeoff is dependency weight, but **egui is already in the project's future plans (Phase 5 editor)** so this front-loads the integration. Per Agent P5: the editor is a new `rust4d_editor` crate that uses egui; integrating egui here avoids duplicate work.

### Render Pipeline Integration

Current pipeline:
```
[Compute: 4D Slice] -> [Render: 3D Cross-Section] -> Surface Present
```

New pipeline after egui integration:
```
[Compute: 4D Slice] -> [Render: 3D Cross-Section] -> [Render: egui Overlay] -> Surface Present
```

Key implementation detail: The egui render pass uses `LoadOp::Load` (not `Clear`) for the color attachment, so it draws ON TOP of the existing 3D scene. No depth buffer needed for 2D overlay.

The current `RenderPipeline::render()` method takes `view: &wgpu::TextureView` as a parameter (line 253 of render_pipeline.rs). The egui pass takes the same `view` and renders after the 3D pass.

### Full render pass ordering (coordinated with P3 and P5):

```
(1) 4D slice compute
(2) 3D cross-section geometry render (depth write ON)
(3) Sprites/billboards [P3] (depth test ON, depth write OFF)
(4) Particles [P2-C] (depth test ON, depth write OFF, blending ON)
(5) egui HUD overlay [P2-B] (no depth)
(6) egui editor overlay [P5] (last, on top of everything)
```

### OverlayRenderer Design

```rust
// In rust4d_render/src/overlay.rs

pub struct OverlayRenderer {
    egui_ctx: egui::Context,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
}

impl OverlayRenderer {
    /// Create the overlay renderer
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        window: &winit::window::Window,
    ) -> Self;

    /// Forward a winit event to egui (for input handling)
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool;

    /// Begin a new frame (call at start of frame)
    pub fn begin_frame(&mut self, window: &winit::window::Window);

    /// Get the egui context for the game to build UI
    pub fn ctx(&self) -> &egui::Context;

    /// End the frame and render egui (call after game has built UI)
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
    );
}
```

### API Usage Pattern (Engine-Side)

The engine manages the overlay lifecycle internally. In the Lua-integrated approach, the engine:
1. Calls `begin_frame()` at frame start
2. Processes Lua HUD draw commands into egui calls
3. Calls `render()` to produce the overlay

```rust
// Engine-internal render loop (simplified)
fn render_frame(&mut self) {
    // ... existing 4D slice + 3D render passes ...

    // Begin egui frame
    self.overlay.begin_frame(&self.window);

    // Engine translates queued Lua HUD commands into egui calls
    let ctx = self.overlay.ctx();
    self.hud_bridge.flush_lua_commands(ctx);

    // End frame and render overlay
    self.overlay.render(&device, &queue, &mut encoder, &view, screen_desc);
}
```

### Lua HUD Drawing API (NEW -- Lua Migration)

egui's immediate-mode Rust API does not translate directly to Lua. Exposing the full egui `Context` to Lua would be extremely complex and fragile. Instead, the engine provides a simplified HUD drawing API.

**Recommended approach**: A set of draw commands that Lua calls each frame. The engine collects these commands and translates them into egui calls internally.

```
hud:draw_text(x, y, text, options?)        -- options: { color, size, font, anchor }
hud:draw_bar(x, y, w, h, fill, options?)   -- health/ammo bar, options: { color, bg_color }
hud:draw_rect(x, y, w, h, options?)        -- filled rectangle (for damage flash, etc.)
hud:draw_image(x, y, w, h, image_name)     -- sprite/image on screen
hud:draw_crosshair(style?)                 -- built-in crosshair styles
```

**Rationale for not exposing raw egui to Lua**: egui's API is deeply Rust-idiomatic (closures, builders, trait objects). Binding it to Lua would require wrapping hundreds of types and methods, creating a massive maintenance surface. The simplified HUD API covers 95% of boomer-shooter HUD needs in 5 functions.

**Design**: Immediate-mode (redraw every frame from Lua). This matches egui's model and is simpler than retained-mode. Performance is fine for HUD-level complexity.

**Alternative considered**: Expose egui directly to Lua via mlua userdata. More powerful but much more work to bind and maintain. Recommend starting with the simplified API and adding egui passthrough later if needed.

**Lua HUD usage example** (replaces the Rust `GameHud` struct):

```lua
-- Game script: draw HUD every frame
function draw_hud(state)
    local sw, sh = screen_width(), screen_height()

    -- Bottom-left: Health bar
    hud:draw_bar(20, sh - 60, 200, 20, state.player_health / 100, {
        color = {1, 0, 0, 1},
        bg_color = {0.3, 0, 0, 1}
    })
    hud:draw_text(20, sh - 80, "HP: " .. state.player_health, {
        color = {1, 0.2, 0.2, 1}, size = 18
    })

    -- Bottom-right: Ammo
    hud:draw_text(sw - 120, sh - 60, "AMMO: " .. state.current_ammo, {
        color = {1, 1, 1, 1}, size = 18
    })

    -- Center: Crosshair
    hud:draw_crosshair("dot")

    -- Top-right: W-position indicator
    hud:draw_text(sw - 100, 20, string.format("W: %.1f", state.player_w), {
        color = {0.7, 0.7, 1, 1}, size = 16
    })
end
```

### New Dependencies for `rust4d_render`

Add to `rust4d_render/Cargo.toml`:
```toml
egui = "0.31"
egui-wgpu = "0.31"
egui-winit = "0.31"
```

These versions must align exactly. The specific version should be whatever is current when implementation starts.

### Workspace Cargo.toml additions:
```toml
[workspace.dependencies]
egui = "0.31"
egui-wgpu = "0.31"
egui-winit = "0.31"
```

### Risk: winit Version Compatibility

The engine currently uses `winit = "0.30"`. `egui-winit` must support this version. winit has frequent breaking changes. This needs verification before implementation begins. If incompatible, options are:
1. Upgrade winit in the engine to match egui-winit's requirement
2. Pin to an older egui-winit version that supports winit 0.30
3. Write a thin winit-to-egui input adapter (undesirable)

### Debug Overlays (Optional, via Feature Flag)

The engine can provide optional debug overlays behind a feature flag:
- FPS counter
- Physics debug visualization
- Entity count

These use the same `OverlayRenderer` and are drawn after the game's HUD.

---

## 5. Sub-Phase C: Particle System (`rust4d_render`)

### Session Estimate: 1.5-2 sessions (Rust) + 0.25 session (Lua bindings) = 1.75-2.25 sessions

- **Session 1**: ParticleEmitter, ParticleSystem update logic, billboard shader, additive pipeline
- **Session 2 (partial)**: Alpha blending pipeline, depth integration, burst spawning, cleanup/testing
- **Lua binding work (0.25 session)**: Particle API wrappers -- named presets, burst/emit/stop

### Design: CPU-Simulated, GPU-Rendered Billboard Particles

For a boomer shooter, we need fast, cheap particles that look good. The approach:

1. **CPU simulation**: Particles are simple structs updated on the CPU. At expected particle counts (hundreds, not millions), CPU simulation is more than fast enough and far simpler than GPU particle compute shaders.

2. **GPU rendering**: Particles are rendered as camera-facing billboards (quads that always face the camera). This requires a separate render pass with a different pipeline (no depth write, additive blending for fire/flash effects, alpha blending for smoke/blood).

3. **3D particles in the sliced space**: Particles exist in 3D (the sliced output space), NOT in 4D pre-slice space. This is a deliberate simplification:
   - Particles are visual effects, not physical 4D objects
   - They spawn at 3D positions in the cross-section and simulate in 3D
   - Avoids the enormous complexity of 4D particle slicing
   - Looks correct: a muzzle flash appears where the gun is in the 3D slice
   - A muzzle flash does not "exist in 4D" -- it is a visual feedback element

### Why 3D and Not 4D

Making particles 4D objects that get sliced would:
- Require running them through the compute slice pipeline (expensive)
- Make them subject to W-distance visibility (a muzzle flash could disappear if the player's slice shifts slightly)
- Add complexity for zero gameplay benefit

3D particles in the sliced output space is the correct abstraction for visual effects.

### Why CPU Not GPU Compute

At the particle counts a boomer shooter needs (hundreds), CPU simulation is simpler, debuggable, and fast enough. GPU compute particles shine at millions of particles (smoke simulations, fluid effects) which we don't need. CPU simulation also makes it trivial to query particle state (e.g., "is this emitter done?") without GPU readback.

### Particle System Architecture

```rust
// In rust4d_render/src/particles/

/// A single particle (internal, not pub)
struct Particle {
    position: [f32; 3],       // 3D position in world space
    velocity: [f32; 3],       // 3D velocity
    color: [f32; 4],          // Current RGBA color
    size: f32,                // Current billboard size
    age: f32,                 // Time since spawn (seconds)
    lifetime: f32,            // Total lifetime (seconds)
}

/// Configuration for a particle emitter
pub struct ParticleEmitterConfig {
    pub max_particles: u32,            // Max live particles for this emitter
    pub spawn_rate: f32,               // Particles per second (0 = burst mode)
    pub burst_count: u32,              // Particles to spawn in burst mode

    // Initial particle properties (with randomization ranges)
    pub initial_velocity: [f32; 3],    // Base velocity
    pub velocity_randomness: f32,      // Random spread (0.0-1.0)
    pub initial_size: f32,             // Starting size
    pub initial_color: [f32; 4],       // Starting color

    // Over-lifetime curves (simplified: start -> end lerp)
    pub end_size: f32,                 // Size at end of lifetime
    pub end_color: [f32; 4],           // Color at end of lifetime (alpha -> 0 for fadeout)

    pub lifetime: f32,                 // Particle lifetime in seconds
    pub lifetime_randomness: f32,      // Random variation (0.0-1.0)

    pub gravity: f32,                  // Downward acceleration (for sparks, blood)
    pub drag: f32,                     // Velocity damping per second

    pub blend_mode: ParticleBlendMode, // Additive (fire) or Alpha (smoke/blood)
}

pub enum ParticleBlendMode {
    Additive,   // For fire, flash, energy effects
    Alpha,      // For smoke, blood, dust
}

/// A particle emitter instance (internal management)
pub struct ParticleEmitter {
    config: ParticleEmitterConfig,
    particles: Vec<Particle>,
    position: [f32; 3],
    active: bool,
}

/// Manages all particle emitters and renders them
pub struct ParticleSystem {
    emitters: Vec<ParticleEmitter>,
    pipeline_additive: wgpu::RenderPipeline,
    pipeline_alpha: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    // ... GPU resources
}

impl ParticleSystem {
    /// Create the particle system
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self;

    /// Spawn a burst of particles at a position
    pub fn spawn_burst(
        &mut self,
        position: [f32; 3],
        config: &ParticleEmitterConfig,
    ) -> EmitterId;

    /// Spawn a continuous emitter at a position
    pub fn spawn_emitter(
        &mut self,
        position: [f32; 3],
        config: &ParticleEmitterConfig,
    ) -> EmitterId;

    /// Update an emitter's position (for attached effects)
    pub fn update_emitter_position(&mut self, id: EmitterId, position: [f32; 3]);

    /// Stop an emitter (existing particles continue, no new spawns)
    pub fn stop_emitter(&mut self, id: EmitterId);

    /// Kill an emitter immediately (remove all particles)
    pub fn kill_emitter(&mut self, id: EmitterId);

    /// Update all particles (call once per frame with delta time)
    pub fn update(&mut self, dt: f32);

    /// Render all particles
    /// Must be called AFTER the 3D cross-section render pass, BEFORE egui overlay
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        view_matrix: [[f32; 4]; 4],
        projection_matrix: [[f32; 4]; 4],
    );
}
```

**Note**: `ParticleSystem::spawn_burst()` is the shared API for both weapons (muzzle flash, impact sparks) and enemies (blood, explosions). Confirmed with Agent P3 -- single shared particle system, not two separate ones.

### Lua Particle API

The engine exposes particles to Lua via named presets. The engine maintains an internal `name -> ParticleEmitterConfig` mapping.

```
particles:define(name, config_table)   -- define a named particle preset from a Lua table
particles:burst(position, preset_name) -- one-shot burst effect
particles:emit(position, preset_name)  -- start continuous emitter, returns emitter ID
particles:stop(emitter_id)             -- stop an emitter (existing particles finish)
particles:kill(emitter_id)             -- immediately remove all particles
particles:move(emitter_id, position)   -- update emitter position
```

`ParticleEmitterConfig` is constructable from a Lua table: `{ max_particles=20, lifetime=0.1, ... }`

**Lua usage example** (replaces Rust `muzzle_flash_config()` / `blood_splatter_config()`):

```lua
-- Game script: define particle presets at startup (hot-reloadable!)
function init_effects()
    particles:define("muzzle_flash", {
        max_particles = 20,
        burst_count = 15,
        initial_velocity = {0, 2, 0},
        velocity_randomness = 0.8,
        initial_size = 0.3,
        initial_color = {1.0, 0.9, 0.5, 1.0},   -- bright yellow-white
        end_size = 0.05,
        end_color = {1.0, 0.3, 0.0, 0.0},        -- fade to transparent orange
        lifetime = 0.1,                            -- very short flash
        gravity = 0,
        drag = 5.0,
        blend_mode = "additive",
    })

    particles:define("blood_splatter", {
        max_particles = 30,
        burst_count = 20,
        initial_velocity = {0, 3, 0},
        velocity_randomness = 0.9,
        initial_color = {0.8, 0.0, 0.0, 1.0},    -- dark red
        end_color = {0.5, 0.0, 0.0, 0.0},
        lifetime = 0.5,
        gravity = 15.0,                            -- falls quickly
        blend_mode = "alpha",
    })
end

-- Game script: trigger effects
function on_weapon_hit(position)
    particles:burst(position, "muzzle_flash")
end

function on_enemy_hit(position)
    particles:burst(position, "blood_splatter")
end
```

### Render Pipeline Position

```
[Compute: 4D Slice]
    -> [Render: 3D Cross-Section] (depth write ON)
    -> [Render: Sprites/Billboards] [P3] (depth test ON, depth write OFF)
    -> [Render: Particles] (depth test ON, depth write OFF, blending ON)
    -> [Render: egui Overlay] (no depth)
    -> Surface Present
```

Particles use the depth buffer from the 3D render pass for occlusion (particles behind geometry are hidden), but do NOT write to it (particles don't occlude each other or geometry).

### Depth Buffer Sharing

Particles need to read the depth buffer for occlusion against scene geometry. This requires the depth texture to have `TEXTURE_BINDING` usage. Agent P2 verified this is already the case:

> render_pipeline.rs line 238: `usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING`

This is the key integration point identified by Agent P3 as well. `RenderPipeline::ensure_depth_texture()` needs to expose the depth buffer view. See Sub-Phase D for the getter method.

### Billboard Shader (WGSL)

New shader file at `crates/rust4d_render/src/shaders/particles.wgsl`:

```wgsl
// particles.wgsl
struct ParticleInstance {
    @location(4) world_position: vec3<f32>,
    @location(5) size: f32,
    @location(6) color: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: ParticleInstance,
) -> VertexOutput {
    // Generate billboard quad from vertex_index (0-5, two triangles)
    // Orient quad to face camera using inverse view matrix
    // Scale by instance.size
    // Position at instance.world_position
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Simple circular particle shape (discard outside circle)
    // Apply instance color
}
```

### File Organization

```
crates/rust4d_render/src/
  particles/
    mod.rs              # Re-exports
    system.rs           # ParticleSystem (update + render management)
    emitter.rs          # ParticleEmitter, ParticleEmitterConfig
    particle.rs         # Particle struct
    gpu.rs              # GPU pipeline, buffers, instance data
  shaders/
    particles.wgsl      # Billboard particle shader
```

### Dependencies

- **Existing render pipeline**: Needs access to the depth texture view from `RenderPipeline::ensure_depth_texture()`. Currently `depth_texture` is private -- needs a getter (see Sub-Phase D).
- **Foundation**: Fixed timestep for consistent particle simulation (dt must be consistent). **Blocking.**
- **Phase 1 (P1)**: Event system for game to trigger particles on events (damage dealt -> blood). **Soft dependency** -- game can call `spawn_burst()` directly (or via Lua `particles:burst()`).
- **`rust4d_scripting`**: Lua bindings require the scripting crate for function registration.

### Particle Count Performance

Expected budget: 200-500 simultaneous particles max for a boomer shooter. The billboard approach with instanced rendering should handle thousands easily, but worth performance testing during implementation.

---

## 6. Sub-Phase D: Screen Effects

### Session Estimate: 0.5 session (Rust) + 0.1 session (Lua bindings) = 0.6 session

- Screen shake struct in `rust4d_game` (trivial)
- TimedEffect helper in `rust4d_game` (trivial)
- Depth texture getter on RenderPipeline (one-liner)
- Lua screen effects wrappers (thin)
- All visual effects (damage flash, muzzle flash) are game-side using Lua HUD API + existing RenderUniforms

### Design: Lightweight Effect Layer

For Phase 2, we do NOT need a full post-processing pipeline. The synthesis specifies "muzzle flash, screen shake, damage flash" at 0.5 session. These are achievable with a minimal approach:

### ScreenShake (in `rust4d_game`)

Screen shake is NOT a post-processing effect. It is a camera offset applied before the view matrix is computed.

```rust
/// Screen shake state (lives in rust4d_game)
pub struct ScreenShake {
    trauma: f32,           // Current trauma level (0.0-1.0)
    decay_rate: f32,       // Trauma decay per second
    max_offset: f32,       // Maximum positional offset
    max_rotation: f32,     // Maximum rotational offset (radians)
    frequency: f32,        // Shake frequency (noise sampling rate)
}

impl ScreenShake {
    pub fn new() -> Self;

    /// Add trauma (stacks, clamped to 1.0)
    pub fn add_trauma(&mut self, amount: f32);

    /// Update shake (call per frame)
    pub fn update(&mut self, dt: f32);

    /// Get current camera offset
    /// Returns (position_offset, rotation_offset)
    pub fn get_offset(&self) -> ([f32; 3], f32);
}
```

**Placement**: `rust4d_game` crate. Screen shake is a common game pattern (not rendering). The game applies the offset to the Camera4D before computing view matrices.

### TimedEffect (in `rust4d_game`)

```rust
/// A timed visual effect (generic fade-out pattern)
pub struct TimedEffect {
    remaining: f32,
    duration: f32,
}

impl TimedEffect {
    pub fn new(duration: f32) -> Self;
    pub fn trigger(&mut self);
    pub fn update(&mut self, dt: f32);
    pub fn intensity(&self) -> f32;  // 1.0 -> 0.0 over duration
    pub fn is_active(&self) -> bool;
}
```

### Depth Texture Getter (in `rust4d_render`)

Add to `RenderPipeline`:
```rust
pub fn depth_texture_view(&self) -> Option<&wgpu::TextureView>
```

One-liner that exposes the existing private depth texture for the particle system (and later, sprite system from P3).

### Lua Screen Effects API

```
effects:screen_shake(intensity, duration?)    -- add screen shake trauma
effects:flash(r, g, b, a, duration)           -- full-screen color flash (replaces egui damage flash)
```

**Lua usage example:**

```lua
-- Game script: weapon feedback
function on_weapon_fire(weapon_type)
    if weapon_type == "shotgun" then
        effects:screen_shake(0.4)
    elseif weapon_type == "rocket" then
        effects:screen_shake(0.7)
    end
end

-- Game script: damage feedback
function on_player_damaged(amount)
    effects:screen_shake(amount / 100)
    effects:flash(255, 0, 0, 128, 0.2)   -- red damage flash
end
```

### Damage Flash (Engine-Side via Lua HUD API)

The `effects:flash()` command is implemented engine-side by drawing a semi-transparent colored rectangle via egui. Alternatively, Lua scripts can use `hud:draw_rect()` directly for custom flash effects.

A proper post-processing pipeline (render to intermediate texture, fullscreen quad shader) can be added in Phase 5 when bloom and other effects are needed.

### Muzzle Flash Lighting (Game-Side -- Lua)

Two components:
1. **Particle effect**: Handled by the particle system (Sub-Phase C) via `particles:burst(pos, "muzzle_flash")`
2. **Light flash**: Brief ambient light boost -- no engine changes needed. Lua scripts adjust RenderUniforms directly or via an engine helper.

Proper point lights come in Phase 5.

---

## 7. Complete Render Pipeline After Phase 2

```
Frame Start
  |
  +--> AudioEngine4D.update_listener(camera_position)  [Audio]
  |
  +--> ParticleSystem.update(dt)                        [Particles]
  |
  +--> ScreenShake.update(dt)                           [Game Feel]
  |    Camera4D + ScreenShake.get_offset() -> view_matrix
  |
  +--> [Compute: 4D Slice]                              [Existing]
  |
  +--> [Render: 3D Cross-Section]                       [Existing]
  |    (writes depth buffer)
  |
  +--> [Render: Sprites/Billboards]                     [P3 - NEW]
  |    (reads depth buffer, no depth write)
  |
  +--> [Render: Particles]                              [P2 - NEW]
  |    (reads depth buffer, no depth write)
  |
  +--> OverlayRenderer.begin_frame()                    [P2 - NEW]
  |    Engine processes Lua HUD draw commands via egui
  |    Engine draws screen flash effects via egui
  |    OverlayRenderer.render()
  |
  +--> Surface Present
```

---

## 8. Crate Organization Summary

### New Crate

| Crate | Purpose | Dependencies |
|-------|---------|--------------|
| `rust4d_audio` | 4D spatial audio engine wrapping kira | `rust4d_math`, `kira`, `log`, `serde` |

### Modified Crates

| Crate | Changes |
|-------|---------|
| `rust4d_render` | Add `OverlayRenderer` (egui-wgpu), `ParticleSystem`, expose depth texture; add `egui`, `egui-wgpu`, `egui-winit` dependencies |
| `rust4d_game` | Add `ScreenShake`, `TimedEffect` helpers |
| `rust4d_scripting` | Add Lua bindings for audio, particles, screen effects, HUD drawing API |

### Workspace Cargo.toml Changes

```toml
# New workspace member
members = [
    # ... existing ...
    "crates/rust4d_audio",
]

# New workspace dependencies
[workspace.dependencies]
kira = { version = "0.9", features = ["mp3", "ogg", "flac", "wav"] }
egui = "0.31"
egui-wgpu = "0.31"
egui-winit = "0.31"
```

---

## 9. Session Estimates

### Engine-Side Work (This Plan)

| Sub-Phase | Task | Sessions | Notes |
|-----------|------|----------|-------|
| A | `rust4d_audio` core (init, load, play, bus routing) | 1 | Kira wrapper, basic playback |
| A | `rust4d_audio` spatial (listener, 4D attenuation, W-filtering) | 0.5-1 | 4D spatial projection onto kira's 3D system |
| A | Lua audio API bindings | 0.25 | String-keyed name-to-handle mapping, bus control |
| B | `OverlayRenderer` (egui-wgpu integration) | 1 | egui render pass, input forwarding |
| B | HUD drawing API for Lua | 0.5-1.0 | New abstraction bridging Lua draw commands to egui |
| C | `ParticleSystem` (emitters, billboard shader, GPU pipeline) | 1.5-2 | CPU sim + GPU billboards + two blend modes |
| C | Lua particle API bindings | 0.25 | Named presets, burst/emit/stop wrappers |
| D | Screen effects (ScreenShake, TimedEffect, depth getter) | 0.5 | Small structs in rust4d_game |
| D | Lua screen effects bindings | 0.1 | Thin wrappers for shake and flash |
| | **Total Engine Work** | **5.6-7.1** | |

The HUD drawing API is the significant new item from the Lua migration. The other Lua bindings are thin wrappers over existing Rust APIs.

### Game-Side Work (NOT in these estimates -- now Lua scripts, for reference)

| Task | Sessions | Notes |
|------|----------|-------|
| Weapon system (hitscan + projectile, ammo, switching) | 2 | Entirely game-side Lua scripts |
| HUD layout (health, ammo, crosshair, W-indicator) | 0.5 | Uses `hud:draw_*()` API -- simpler than Rust egui code |
| Sound asset loading and trigger logic | 0.25 | Lua one-liners via `audio:play_oneshot()` -- simpler than Rust |
| Effect presets (muzzle flash, blood, explosion configs) | 0.25 | Lua data tables, hot-reloadable -- simpler than Rust |
| Damage flash, muzzle flash lighting | 0.1 | `effects:flash()` + ambient boost -- simpler than Rust |
| **Total Game Work** | **~3.1** | |

---

## 10. Dependencies

### External Dependencies (Phase 2 Depends On)

| Dependency | Source | Blocking? | Notes |
|------------|--------|-----------|-------|
| Fixed timestep | Foundation | **Yes** for particles | Particle simulation needs consistent dt |
| Serialization (serde) | Foundation | No (nice-to-have) | Audio/particle configs could use serialization but not blocking |
| Event system | P1 | **Soft** | Game can trigger audio/particles directly without events; events make it cleaner |
| Raycasting | P1 | No | Audio/HUD/particles don't need raycasting |
| ECS / `rust4d_game` crate | Split Plan | **Yes** for ScreenShake/TimedEffect | These go in rust4d_game which must exist |
| Sprite rendering | P3 | No | Particles are billboards, not sprites. Separate pipeline. |
| `rust4d_scripting` crate | Scripting Phase | **Yes** for Lua bindings | Lua runtime, script lifecycle, hot-reload must be available |

### What Other Phases Need From Phase 2

| Consumer Phase | What They Need | Sub-Phase |
|---------------|----------------|-----------|
| P3 (Enemies) | `ParticleSystem::spawn_burst()` (Rust) or `particles:burst()` (Lua) for blood/explosion effects | C |
| P3 (Enemies) | Depth texture getter for sprite rendering | D |
| P4 (Level Design) | Audio for door/elevator sounds (Rust) or `audio:play_spatial()` (Lua) | A |
| P5 (Editor) | egui integration (OverlayRenderer) as foundation for editor UI | B |
| P5 (Editor) | Point lights add bind group 1 to main render pipeline -- HUD/sprite passes use separate pipelines, no conflict | B |

---

## 11. Parallelization

### Internal Parallelism

Phase 2 sub-tasks are largely independent of each other:

```
Wave P2-1 (Parallel -- can all run simultaneously):
  +-- Agent A: rust4d_audio crate (audio engine + spatial)       [1.5-2 sessions]
  +-- Agent B: OverlayRenderer (egui-wgpu integration)           [1 session]
  +-- Agent C: ParticleSystem (emitters + billboard renderer)     [1.5-2 sessions]

Wave P2-2 (Parallel -- after P2-1 completes, requires rust4d_scripting):
  +-- Agent D: Integration + Lua bindings                        [1.1-1.6 sessions]
      - ScreenShake in rust4d_game
      - TimedEffect in rust4d_game
      - Depth texture getter on RenderPipeline
      - Wire everything into the render loop
      - Lua audio API wrappers (0.25 session)
      - Lua particle API wrappers (0.25 session)
      - Lua screen effects API (0.1 session)
      - Integration testing (Rust + Lua)
  +-- Agent E: HUD drawing API for Lua                           [0.5-1.0 session]
      - Design and implement the Lua-to-egui bridge
      - draw_text, draw_bar, draw_rect, draw_image, draw_crosshair
      - Lua integration tests for HUD rendering
```

**Critical path**: 1.5-2 sessions (Wave P2-1 longest item) + 1.1-1.6 sessions (Wave P2-2 longest item) = **2.6-3.6 sessions** if fully parallelized.

### External Parallelism

Phase 2 sub-phases can run in parallel with work from other phases:

- **P2-A (Audio)** has zero dependency on P1, P3, P4, P5. Can start as soon as Foundation is done and `rust4d_math` provides Vec4.
- **P2-B (egui)** has zero dependency on other phases. Can start independently.
- **P2-C (Particles)** depends on Foundation (fixed timestep). Can run parallel with P1.
- **P2-D (Screen Effects)** depends on `rust4d_game` crate existing (from split plan).
- **Lua bindings (all)** depend on `rust4d_scripting` crate existing. Can run in parallel with each other once the scripting crate is ready.

---

## 12. Verification Criteria

### Sub-Phase A: Audio System (Rust)
- [ ] `AudioEngine4D::new()` initializes kira backend without errors
- [ ] `load_sound()` loads WAV and OGG files
- [ ] `play()` plays non-spatial sounds on correct bus
- [ ] `play_spatial()` correctly attenuates by 4D distance
- [ ] W-distance filtering silences sounds outside `w_audible_range`
- [ ] `set_bus_volume()` smoothly transitions volume
- [ ] `stop_all()` / `stop_bus()` silence all sounds
- [ ] All methods are non-blocking (kira audio thread)
- [ ] Unit tests for 4D distance attenuation math
- [ ] Unit tests for W-distance filtering logic

### Sub-Phase A: Audio System (Lua Integration)
- [ ] Lua script calls `audio:load(name, path)` and sound is registered
- [ ] Lua script calls `audio:play("name", "sfx")` and sound plays
- [ ] Lua script calls `audio:play_spatial()` at a 4D position and spatial attenuation works
- [ ] Lua script calls `audio:set_volume("music", 0.5)` and bus volume changes
- [ ] Lua script calls `audio:stop("sfx")` and all SFX sounds stop
- [ ] Invalid sound name produces a Lua error (not a crash)

### Sub-Phase B: HUD/egui Overlay (Rust)
- [ ] `OverlayRenderer::new()` creates egui context and wgpu renderer
- [ ] `handle_event()` forwards winit events to egui
- [ ] `begin_frame()` / `render()` cycle produces visible output
- [ ] egui draws ON TOP of 3D scene (LoadOp::Load verified)
- [ ] Simple "Hello World" overlay renders correctly
- [ ] Mouse clicks are captured by egui when over UI elements
- [ ] Integration test: overlay + 3D scene renders without artifacts

### Sub-Phase B: HUD Drawing API (Lua Integration)
- [ ] `hud:draw_text(x, y, "Hello")` renders visible text at correct position
- [ ] `hud:draw_text()` with options (color, size) renders correctly
- [ ] `hud:draw_bar(x, y, w, h, fill)` renders a bar with correct fill percentage
- [ ] `hud:draw_rect(x, y, w, h, options)` draws a filled rectangle
- [ ] `hud:draw_crosshair("dot")` renders a centered crosshair
- [ ] HUD draws correctly on top of 3D scene
- [ ] Hot-reload of Lua script updates HUD layout without restart
- [ ] Multiple draw commands per frame compose correctly

### Sub-Phase C: Particle System (Rust)
- [ ] `ParticleEmitterConfig` with `Default` implementation
- [ ] `spawn_burst()` creates correct number of particles
- [ ] `spawn_emitter()` continuously spawns at configured rate
- [ ] `update()` correctly simulates position, velocity, gravity, drag
- [ ] Particles fade/shrink according to start/end color/size
- [ ] Dead particles (age > lifetime) are removed
- [ ] Additive blend mode works (bright, glowing effects)
- [ ] Alpha blend mode works (opaque-to-transparent fadeout)
- [ ] Particles are occluded by scene geometry (depth test against 3D pass depth buffer)
- [ ] Particles do NOT occlude each other or scene geometry (no depth write)
- [ ] Billboard orientation is correct (always faces camera)
- [ ] `kill_emitter()` immediately removes all particles
- [ ] `stop_emitter()` allows existing particles to finish
- [ ] Unit tests for particle simulation math
- [ ] Performance: 500 simultaneous particles at 60fps

### Sub-Phase C: Particle System (Lua Integration)
- [ ] Lua script calls `particles:define(name, table)` and preset is registered
- [ ] Lua script calls `particles:burst(position, "muzzle_flash")` and particles appear
- [ ] Lua script creates a continuous emitter via `particles:emit()` and receives an ID
- [ ] Lua script stops an emitter via `particles:stop(id)` and existing particles finish
- [ ] Lua script kills an emitter via `particles:kill(id)` and particles vanish immediately
- [ ] Lua script moves an emitter via `particles:move(id, pos)` and it tracks
- [ ] Invalid preset name produces a Lua error (not a crash)

### Sub-Phase D: Screen Effects (Rust)
- [ ] `ScreenShake::add_trauma()` stacks and clamps to 1.0
- [ ] `ScreenShake::update()` decays trauma over time
- [ ] `ScreenShake::get_offset()` returns position + rotation offsets
- [ ] `TimedEffect::trigger()` starts the effect
- [ ] `TimedEffect::intensity()` returns 1.0 at start, 0.0 at end
- [ ] `TimedEffect::is_active()` returns false after duration
- [ ] `RenderPipeline::depth_texture_view()` returns the depth texture

### Sub-Phase D: Screen Effects (Lua Integration)
- [ ] Lua script calls `effects:screen_shake(0.5)` and camera shakes
- [ ] Lua script calls `effects:flash(255, 0, 0, 128, 0.2)` and screen flashes red
- [ ] Screen shake decays naturally after Lua trigger

---

## 13. Open Questions and Risks

### Open Questions

1. **Kira version**: API surface changed between kira 0.8 and 0.9. Need to verify the latest stable version when implementing. The spatial track API is what we need.

2. **egui version alignment**: egui, egui-wgpu, and egui-winit versions must match exactly. Need to check compatibility with the winit version already in the workspace (`winit = "0.30"`).

3. **Particle count limits**: How many particles can we sustain at 60fps? The billboard approach with instanced rendering should handle thousands easily, but worth testing. For a boomer shooter, we probably need 200-500 simultaneous particles max.

4. **4D audio W-filtering**: Should W-distance filtering be a low-pass filter or just volume attenuation? Low-pass filtering requires kira's effect system per spatial track. Volume-only is simpler and might be sufficient for an initial implementation. Recommendation: start with volume-only, add low-pass later if it improves the feel.

5. **HUD API scope (Lua)**: Should the HUD drawing API be immediate-mode (redraw every frame from Lua) or retained-mode (define layout once, update values)? Immediate-mode is simpler and matches egui's model. Performance should be fine for HUD-level complexity. Recommendation: immediate-mode.

6. **HUD image support**: `hud:draw_image()` requires loading textures into egui's texture system. This needs integration with the TextureManager or a separate HUD texture registry. May be deferred if not needed for the initial HUD.

### Risks

1. **egui-winit compatibility** (MEDIUM): The engine uses `winit = "0.30"`. Need to verify `egui-winit` supports this version. winit has frequent breaking changes. Mitigation: check version compatibility before starting Sub-Phase B.

2. **kira threading model** (LOW): Kira uses a separate audio thread. Commands are sent from the game thread. The `AudioEngine4D` wrapper must be designed to handle this cleanly (all methods non-blocking). Mitigation: kira's API is designed for this; just follow their patterns.

3. **Particle depth interaction** (LOW): Particles need to read the depth buffer. The depth texture already has `TEXTURE_BINDING` usage (verified in render_pipeline.rs line 238). No changes needed to the depth texture creation.

4. **Performance budget** (LOW): Adding three new render passes (sprites from P3, particles, egui) to every frame. Each pass has overhead. Mitigation: the egui pass only draws when there is content; the particle pass should be skipped when no emitters are active.

5. **Post-processing deferral** (RISK NOTED): Phase 2 deliberately avoids building a post-processing pipeline. Damage flash and muzzle flash use simpler approaches (Lua `effects:flash()`, ambient boost). This means Phase 5 will need to build post-processing from scratch when bloom/etc. are needed. This is an accepted tradeoff -- the simpler approach is correct for now.

6. **Lua-to-egui bridge performance** (LOW): The HUD drawing API marshals Lua draw commands into egui calls each frame. For a boomer shooter HUD (10-20 draw commands per frame), this overhead is negligible. If more complex UIs are needed, the bridge can be optimized with command batching.

7. **Lua binding maintenance** (ONGOING): Each Lua binding is a maintenance surface -- when the Rust API changes, the Lua wrapper must be updated. Mitigation: keep bindings thin (direct pass-through), use mlua's derive macros where possible, automated tests catch mismatches.

---

## 14. Key Design Decisions and Rationale

### Why NOT a `rust4d_particles` crate?
Particles are tightly coupled to the rendering pipeline (they need the wgpu device, the depth texture, the view/projection matrices, and a render pass slot in the frame). Putting them in `rust4d_render` keeps all GPU code together and avoids cross-crate GPU resource sharing complexity.

### Why NOT custom wgpu text rendering for HUD?
Custom text rendering requires: font loading, glyph rasterization, text layout, texture atlas management, and a text render pipeline. This is weeks of work for a worse result than egui provides out of the box. egui also gives us debug UI and menu systems "for free" which will be used extensively.

### Why front-load egui in Phase 2?
Phase 5 needs egui for the editor. Integrating it in Phase 2 means:
- The dependency is already resolved by the time the editor starts
- The game has HUD capabilities earlier
- Debug overlays become available immediately
- Any winit compatibility issues surface early

### Why screen shake in `rust4d_game` and not `rust4d_render`?
Screen shake modifies camera position/rotation before the view matrix is computed. It is a game-feel utility, not a rendering effect. Placing it in `rust4d_game` keeps the rendering crate focused on GPU work.

### Why damage flash via egui instead of post-processing?
A post-processing pipeline (render to intermediate texture, fullscreen quad shader) is significant infrastructure. For a semi-transparent color overlay, egui's painter API is adequate and avoids the engineering cost. Post-processing can be added in Phase 5 when bloom and other shader effects are needed.

### Why a simplified HUD API instead of exposing egui to Lua?
egui's immediate-mode API is deeply Rust-idiomatic: closures, builders, trait objects. Binding the full `egui::Context` to Lua would require wrapping hundreds of types and methods, creating a massive maintenance surface that breaks with every egui version update. The simplified HUD API (`draw_text`, `draw_bar`, `draw_rect`, `draw_image`, `draw_crosshair`) covers the needs of a boomer shooter HUD in 5 functions. If more complex UI is needed later, egui passthrough can be added incrementally.

### Why string-keyed sound names instead of Lua userdata handles?
Lua scripts reference sounds by name (`"shotgun_fire"`) rather than by opaque handle. This is more natural for scripting (no handle lifecycle management), enables hot-reload (re-load a sound asset without invalidating references), and makes scripts self-documenting. The engine maintains the name-to-handle mapping internally.
