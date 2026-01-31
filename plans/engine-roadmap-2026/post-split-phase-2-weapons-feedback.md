# Post-Split Phase 2: Weapons & Feedback -- Engine Implementation Plan

**Source**: Agent P2 report from engine roadmap planning swarm (2026-01-30)
**Status**: Planning document -- not yet started
**Estimated Sessions**: 4.5-5.5 (engine-side only)
**Prerequisites**: Engine/game split complete, `rust4d_game` crate exists, Foundation phase done

---

## 1. Overview

Phase 2 is where combat starts to *feel* good. The cross-swarm synthesis identified four deliverables for this phase: weapon system, HUD, audio, and screen effects. After applying the engine/game split boundary, the engine work decomposes into:

- **Audio system**: New `rust4d_audio` crate wrapping kira with 4D spatial audio
- **HUD/overlay rendering**: egui-wgpu integration in `rust4d_render` via `OverlayRenderer`
- **Particle/effect system**: CPU-simulated, GPU-rendered billboard particles in `rust4d_render`
- **Screen effects**: `ScreenShake` and `TimedEffect` helpers in `rust4d_game`, depth texture exposure in `rust4d_render`

The weapon system itself is **100% game-side**. The engine provides no weapon abstractions. The game repo builds weapons on top of raycasting (P1), audio (P2-A), particles (P2-C), and the HUD (P2-B).

---

## 2. Engine vs Game Boundary

### Engine Responsibilities (this plan)

| Subsystem | Crate | What the Engine Provides |
|-----------|-------|--------------------------|
| Audio | `rust4d_audio` (NEW) | AudioEngine4D, 4D spatial audio, bus/mixer, sound loading |
| HUD/Overlay | `rust4d_render` | OverlayRenderer (egui-wgpu integration), egui context lifecycle |
| Particles | `rust4d_render` | ParticleSystem, ParticleEmitter, billboard renderer, blend modes |
| Screen Effects | `rust4d_game` + `rust4d_render` | ScreenShake, TimedEffect, depth texture getter |

### Game Responsibilities (NOT in this plan)

| Feature | What the Game Builds |
|---------|---------------------|
| Weapon system | Hitscan shotgun, projectile rocket launcher, ammo, weapon switching |
| HUD widgets | Health bar, ammo counter, crosshair, W-position indicator |
| Sound triggering | Which sounds play when (gunfire, pickup, enemy death, etc.) |
| Sound assets | .wav/.ogg files for all game sounds |
| Music | Playlists, ambient soundscapes, level music |
| Effect presets | Muzzle flash config, blood splatter config, explosion config |
| Damage flash | Game-side egui overlay (semi-transparent red rect) |
| Muzzle flash lighting | Brief ambient_strength boost in RenderUniforms |

Key decisions from Agent P2:
- **Weapon system is 100% game-side.** The engine has no concept of weapons.
- **Screen shake goes in `rust4d_game`** -- it is a camera offset, not post-processing.
- **Damage flash via egui overlay** -- no post-processing pipeline needed for Phase 2.
- **Particles are 3D, not 4D** -- they exist in sliced output space and bypass the compute shader.

---

## 3. Sub-Phase A: Audio System (`rust4d_audio` crate)

### Session Estimate: 1.5-2 sessions

- **Session 1**: AudioEngine4D core (init, load, play, play_oneshot, bus routing), basic tests with mock sounds
- **Session 2 (partial)**: 4D spatial audio (listener updates, W-distance attenuation, W-filtering), spatial tests

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

### Game-Side Usage Example

```rust
// Game-side example usage
struct GameAudio {
    engine: AudioEngine4D,
    shotgun_fire: SoundHandle,
    rocket_fire: SoundHandle,
    rocket_explode: SoundHandle,
    enemy_pain: SoundHandle,
    pickup_health: SoundHandle,
    music_level1: SoundHandle,
}

impl GameAudio {
    fn on_weapon_fire(&mut self, weapon: WeaponType, position: Vec4) {
        let sound = match weapon {
            WeaponType::Shotgun => &self.shotgun_fire,
            WeaponType::RocketLauncher => &self.rocket_fire,
        };
        self.engine.play_oneshot_spatial(
            sound,
            position,
            AudioBus::Sfx,
            SpatialConfig::default(),
        );
    }
}
```

### Dependencies

- **Foundation**: Audio config needs serialization (serde) for saving volume settings
- **Phase 1 (P1)**: Event system -- game uses events to trigger audio (damage dealt -> play hit sound). This is a SOFT dependency; game can trigger audio directly without events.
- **ECS split**: AudioEngine4D lives as a resource, not a component. No ECS dependency.

### Kira Threading Model

Kira uses a separate audio thread. Commands are sent from the game thread. The `AudioEngine4D` wrapper must be designed to handle this cleanly -- all methods should be non-blocking.

---

## 4. Sub-Phase B: HUD/egui Overlay (`rust4d_render`)

### Session Estimate: 1 session

- Integrate egui-wgpu into the render pipeline
- Create OverlayRenderer with begin/end frame pattern
- Forward winit events to egui
- Test with a simple "Hello World" overlay
- Document the game-side API pattern

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

The engine exposes `OverlayRenderer` and `egui::Context`. The game's render system calls it:

```rust
// Game-side render loop (simplified)
fn render_frame(&mut self) {
    // ... existing 4D slice + 3D render passes ...

    // Begin egui frame
    self.overlay.begin_frame(&self.window);

    // Game builds its HUD using egui context
    let ctx = self.overlay.ctx();
    self.hud.draw(ctx, &self.game_state);
    self.menu.draw(ctx, &self.game_state);

    // End frame and render overlay
    self.overlay.render(&device, &queue, &mut encoder, &view, screen_desc);
}
```

### Game-Side HUD Example

```rust
struct GameHud;

impl GameHud {
    fn draw(&self, ctx: &egui::Context, state: &GameState) {
        // Bottom-left: Health
        egui::Area::new("health")
            .fixed_pos(egui::pos2(20.0, screen_height - 60.0))
            .show(ctx, |ui| {
                ui.colored_label(egui::Color32::RED,
                    format!("HP: {}", state.player_health));
            });

        // Bottom-right: Ammo
        egui::Area::new("ammo")
            .fixed_pos(egui::pos2(screen_width - 120.0, screen_height - 60.0))
            .show(ctx, |ui| {
                ui.label(format!("AMMO: {}", state.current_ammo));
            });

        // Center: Crosshair
        egui::Area::new("crosshair")
            .fixed_pos(egui::pos2(screen_width / 2.0 - 8.0, screen_height / 2.0 - 8.0))
            .show(ctx, |ui| {
                ui.label("+");  // Simple crosshair (later: custom painter)
            });

        // Top-right: W-position indicator
        egui::Area::new("w_indicator")
            .fixed_pos(egui::pos2(screen_width - 100.0, 20.0))
            .show(ctx, |ui| {
                ui.label(format!("W: {:.1}", state.player_w));
            });
    }
}
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

### Session Estimate: 1.5-2 sessions

- **Session 1**: ParticleEmitter, ParticleSystem update logic, billboard shader, additive pipeline
- **Session 2 (partial)**: Alpha blending pipeline, depth integration, burst spawning, cleanup/testing

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

### Game-Side Effect Preset Examples

```rust
fn muzzle_flash_config() -> ParticleEmitterConfig {
    ParticleEmitterConfig {
        max_particles: 20,
        spawn_rate: 0.0,        // Burst mode
        burst_count: 15,
        initial_velocity: [0.0, 2.0, 0.0],
        velocity_randomness: 0.8,
        initial_size: 0.3,
        initial_color: [1.0, 0.9, 0.5, 1.0],  // Bright yellow-white
        end_size: 0.05,
        end_color: [1.0, 0.3, 0.0, 0.0],       // Fade to transparent orange
        lifetime: 0.1,          // Very short -- a flash
        gravity: 0.0,
        drag: 5.0,
        blend_mode: ParticleBlendMode::Additive,
        ..Default::default()
    }
}

fn blood_splatter_config() -> ParticleEmitterConfig {
    ParticleEmitterConfig {
        max_particles: 30,
        burst_count: 20,
        initial_velocity: [0.0, 3.0, 0.0],
        velocity_randomness: 0.9,
        initial_color: [0.8, 0.0, 0.0, 1.0],  // Dark red
        end_color: [0.5, 0.0, 0.0, 0.0],
        lifetime: 0.5,
        gravity: 15.0,          // Falls quickly
        blend_mode: ParticleBlendMode::Alpha,
        ..Default::default()
    }
}
```

### Dependencies

- **Existing render pipeline**: Needs access to the depth texture view from `RenderPipeline::ensure_depth_texture()`. Currently `depth_texture` is private -- needs a getter (see Sub-Phase D).
- **Foundation**: Fixed timestep for consistent particle simulation (dt must be consistent). **Blocking.**
- **Phase 1 (P1)**: Event system for game to trigger particles on events (damage dealt -> blood). **Soft dependency** -- game can call `spawn_burst()` directly.

### Particle Count Performance

Expected budget: 200-500 simultaneous particles max for a boomer shooter. The billboard approach with instanced rendering should handle thousands easily, but worth performance testing during implementation.

---

## 6. Sub-Phase D: Screen Effects

### Session Estimate: 0.5 session

- Screen shake struct in `rust4d_game` (trivial)
- TimedEffect helper in `rust4d_game` (trivial)
- Depth texture getter on RenderPipeline (one-liner)
- All visual effects (damage flash, muzzle flash) are game-side using egui overlay + existing RenderUniforms

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

### Damage Flash (Game-Side via egui)

No engine work needed. The game draws a semi-transparent colored rectangle:

```rust
// Game-side: draw a damage flash as an egui area
fn draw_damage_flash(ctx: &egui::Context, intensity: f32) {
    if intensity > 0.0 {
        egui::Area::new("damage_flash")
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let color = egui::Color32::from_rgba_unmultiplied(
                    255, 0, 0, (intensity * 128.0) as u8
                );
                let size = ui.available_size();
                ui.painter().rect_filled(
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size),
                    0.0,
                    color,
                );
            });
    }
}
```

A proper post-processing pipeline (render to intermediate texture, apply fullscreen shader) can be added in Phase 5 when bloom and other effects are needed.

### Muzzle Flash Lighting (Game-Side)

Two components:
1. **Particle effect**: Handled by the particle system (Sub-Phase C)
2. **Light flash**: Brief ambient light boost -- no engine changes needed:

```rust
// Game-side: temporarily increase ambient strength in RenderUniforms
fn apply_muzzle_flash(&mut self, intensity: f32) {
    self.render_uniforms.ambient_strength = 0.3 + intensity * 0.5;
}
```

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
  |    Game builds HUD via egui context
  |    Game draws damage flash / effects via egui
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
| B | `OverlayRenderer` (egui-wgpu integration) | 1 | egui render pass, input forwarding |
| C | `ParticleSystem` (emitters, billboard shader, GPU pipeline) | 1.5-2 | CPU sim + GPU billboards + two blend modes |
| D | Screen effects (ScreenShake, TimedEffect, depth getter) | 0.5 | Small structs in rust4d_game |
| | **Total Engine Work** | **4.5-5.5** | |

### Game-Side Work (NOT in these estimates, for reference)

| Task | Sessions | Notes |
|------|----------|-------|
| Weapon system (hitscan + projectile, ammo, switching) | 2 | Entirely game-side |
| HUD widgets (health, ammo, crosshair, W-indicator) | 0.5 | Uses OverlayRenderer's egui context |
| Sound assets and trigger logic | 0.5 | Game-side audio integration |
| Effect presets (muzzle flash, blood, explosion configs) | 0.5 | ParticleEmitterConfig definitions |
| Damage flash, muzzle flash lighting | 0.25 | Game-side egui overlay + ambient boost |
| **Total Game Work** | **~3.75** | |

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

### What Other Phases Need From Phase 2

| Consumer Phase | What They Need | Sub-Phase |
|---------------|----------------|-----------|
| P3 (Enemies) | `ParticleSystem::spawn_burst()` for blood/explosion effects | C |
| P3 (Enemies) | Depth texture getter for sprite rendering | D |
| P4 (Level Design) | Audio for door/elevator sounds | A |
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

Wave P2-2 (Sequential -- after P2-1 completes):
  +-- Agent D: Integration                                        [0.5 session]
      - ScreenShake in rust4d_game
      - TimedEffect in rust4d_game
      - Depth texture getter on RenderPipeline
      - Wire everything into the render loop
      - Integration testing
```

**Critical path**: 1.5-2 sessions (Wave P2-1 longest item) + 0.5 session (Wave P2-2) = **2-2.5 sessions** if fully parallelized.

### External Parallelism

Phase 2 sub-phases can run in parallel with work from other phases:

- **P2-A (Audio)** has zero dependency on P1, P3, P4, P5. Can start as soon as Foundation is done and `rust4d_math` provides Vec4.
- **P2-B (egui)** has zero dependency on other phases. Can start independently.
- **P2-C (Particles)** depends on Foundation (fixed timestep). Can run parallel with P1.
- **P2-D (Screen Effects)** depends on `rust4d_game` crate existing (from split plan).

---

## 12. Verification Criteria

### Sub-Phase A: Audio System
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

### Sub-Phase B: HUD/egui Overlay
- [ ] `OverlayRenderer::new()` creates egui context and wgpu renderer
- [ ] `handle_event()` forwards winit events to egui
- [ ] `begin_frame()` / `render()` cycle produces visible output
- [ ] egui draws ON TOP of 3D scene (LoadOp::Load verified)
- [ ] Simple "Hello World" overlay renders correctly
- [ ] Mouse clicks are captured by egui when over UI elements
- [ ] Integration test: overlay + 3D scene renders without artifacts

### Sub-Phase C: Particle System
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

### Sub-Phase D: Screen Effects
- [ ] `ScreenShake::add_trauma()` stacks and clamps to 1.0
- [ ] `ScreenShake::update()` decays trauma over time
- [ ] `ScreenShake::get_offset()` returns position + rotation offsets
- [ ] `TimedEffect::trigger()` starts the effect
- [ ] `TimedEffect::intensity()` returns 1.0 at start, 0.0 at end
- [ ] `TimedEffect::is_active()` returns false after duration
- [ ] `RenderPipeline::depth_texture_view()` returns the depth texture

---

## 13. Open Questions and Risks

### Open Questions

1. **Kira version**: API surface changed between kira 0.8 and 0.9. Need to verify the latest stable version when implementing. The spatial track API is what we need.

2. **egui version alignment**: egui, egui-wgpu, and egui-winit versions must match exactly. Need to check compatibility with the winit version already in the workspace (`winit = "0.30"`).

3. **Particle count limits**: How many particles can we sustain at 60fps? The billboard approach with instanced rendering should handle thousands easily, but worth testing. For a boomer shooter, we probably need 200-500 simultaneous particles max.

4. **4D audio W-filtering**: Should W-distance filtering be a low-pass filter or just volume attenuation? Low-pass filtering requires kira's effect system per spatial track. Volume-only is simpler and might be sufficient for an initial implementation. Recommendation: start with volume-only, add low-pass later if it improves the feel.

### Risks

1. **egui-winit compatibility** (MEDIUM): The engine uses `winit = "0.30"`. Need to verify `egui-winit` supports this version. winit has frequent breaking changes. Mitigation: check version compatibility before starting Sub-Phase B.

2. **kira threading model** (LOW): Kira uses a separate audio thread. Commands are sent from the game thread. The `AudioEngine4D` wrapper must be designed to handle this cleanly (all methods non-blocking). Mitigation: kira's API is designed for this; just follow their patterns.

3. **Particle depth interaction** (LOW): Particles need to read the depth buffer. The depth texture already has `TEXTURE_BINDING` usage (verified in render_pipeline.rs line 238). No changes needed to the depth texture creation.

4. **Performance budget** (LOW): Adding three new render passes (sprites from P3, particles, egui) to every frame. Each pass has overhead. Mitigation: the egui pass only draws when there is content; the particle pass should be skipped when no emitters are active.

5. **Post-processing deferral** (RISK NOTED): Phase 2 deliberately avoids building a post-processing pipeline. Damage flash and muzzle flash use simpler approaches (egui overlay, ambient boost). This means Phase 5 will need to build post-processing from scratch when bloom/etc. are needed. This is an accepted tradeoff -- the simpler approach is correct for now.

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
