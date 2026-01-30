# Agent P2: Weapons & Feedback -- Engine Implementation Plan
**Agent**: P2 (Weapons & Feedback)
**Date**: 2026-01-30
**Scope**: Audio system, HUD/overlay rendering, particle/effect system, screen effects framework

---

## Executive Summary

Phase 2 of the cross-swarm synthesis ("Weapons & Feedback") is where combat starts to *feel* good. The four items listed -- weapon system, HUD, audio, and screen effects -- decompose cleanly along the engine/game boundary:

- **Weapon system**: Entirely game-side. The engine provides no weapon abstractions.
- **HUD/overlay rendering**: Engine provides a 2D overlay rendering layer via egui-wgpu integration. Game builds specific HUD widgets.
- **Audio system**: Engine provides a new `rust4d_audio` crate wrapping kira with a 4D spatial audio abstraction. Game triggers sounds and manages audio assets.
- **Particle/effect system**: Engine provides a generic particle emitter system in `rust4d_render`. Game configures specific effects (muzzle flash, blood, sparks).
- **Screen effects**: Engine provides a post-processing effect stack in `rust4d_render`. Game triggers specific effects (screen shake, damage flash, color tint).

The engine work totals **4-5.5 sessions** across 3 new subsystems and 1 significant extension to the render pipeline.

---

## 1. Audio System -- New `rust4d_audio` Crate

### Engine vs Game Boundary

| Responsibility | Owner |
|---|---|
| Audio device initialization, mixer, playback | **Engine** (`rust4d_audio`) |
| 4D spatial audio (listener + emitters with 4D distance attenuation) | **Engine** (`rust4d_audio`) |
| Sound asset loading (WAV, OGG, MP3, FLAC) | **Engine** (`rust4d_audio`) |
| Audio bus/channel organization (SFX, Music, Ambient) | **Engine** (`rust4d_audio`) |
| Which sounds to play when (gunfire, pickup, enemy death) | **Game** |
| Sound asset files (.wav, .ogg) | **Game** |
| Music playlists and ambient configuration | **Game** |

### Crate Choice: Kira

**Recommendation: Use [kira](https://crates.io/crates/kira)** over rodio.

Rationale:
1. **Built-in spatial audio**: Kira has a listener/emitter model with spatial tracks, distance attenuation, and panning. Rodio's `Spatial` source is basic by comparison.
2. **Tweens and transitions**: Kira provides built-in tweens for smoothly adjusting volume, pitch, and panning. Essential for "game feel" -- sounds should fade in/out, not cut abruptly.
3. **Mixer/track system**: Kira's flexible mixer allows creating sub-tracks (SFX, Music, Ambient) with independent volume control and effects. This is the audio bus abstraction games need.
4. **Clock system**: Precise audio event timing for synchronized effects (e.g., muzzle flash + sound must be frame-accurate).
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
- Sounds within a configurable `w_audible_range` (e.g., 5.0 units) are audible
- Volume attenuates by 4D distance (not just 3D)
- Sounds far in W get low-pass filtered (like hearing through walls)
- Sounds outside `w_audible_range` are silent (pragmatic cutoff)

This maps naturally onto kira's spatial track API: the engine computes the effective 3D position for kira's listener system by projecting the 4D distance onto a 3D representation:

```rust
// Project 4D emitter to kira's 3D spatial system
// X,Z = direction in the XZ plane (for left/right/front/back panning)
// Y = height (for vertical panning)
// Distance = 4D euclidean distance (for volume falloff)
```

The W-filtering (low-pass for distant W) uses kira's per-track effects.

### Crate Organization

```
crates/rust4d_audio/
  Cargo.toml          # depends on: kira, rust4d_math, log
  src/
    lib.rs            # Public API re-exports
    audio_engine.rs   # AudioEngine4D wrapper around kira::AudioManager
    listener.rs       # Listener4D (position + orientation in 4D)
    emitter.rs        # SoundEmitter4D (position in 4D + spatial config)
    bus.rs            # AudioBus enum (SFX, Music, Ambient) + volume control
    sound.rs          # SoundHandle, SoundId, sound loading
    config.rs         # Audio configuration (master volume, bus volumes, spatial settings)
```

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

### What the GAME Builds On Top

The game repo creates sound asset management and event-driven sound triggering:

```rust
// Game-side example usage
struct GameAudio {
    engine: AudioEngine4D,
    // Sound handles loaded at startup
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
- **Phase 1 (P1)**: Event system -- game uses events to trigger audio (damage dealt -> play hit sound)
- **ECS split**: AudioEngine4D lives as a resource, not a component. No ECS dependency.

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

### Session Estimate: 1.5-2 sessions

- Session 1: AudioEngine4D core (init, load, play, play_oneshot, bus routing), basic tests with mock sounds
- Session 2 (partial): 4D spatial audio (listener updates, W-distance attenuation, W-filtering), spatial tests

---

## 2. HUD / Overlay Rendering -- egui Integration in `rust4d_render`

### Engine vs Game Boundary

| Responsibility | Owner |
|---|---|
| egui-wgpu integration (render pass, input forwarding) | **Engine** (`rust4d_render`) |
| Overlay render pass that draws egui on top of 3D scene | **Engine** (`rust4d_render`) |
| egui context lifetime management | **Engine** (`rust4d_render`) |
| Specific HUD widgets (health bar, ammo counter, crosshair) | **Game** |
| W-position indicator design and placement | **Game** |
| Menu screens (pause, options, title) | **Game** |
| Debug overlays (FPS counter, physics debug) | **Engine** (optional, via feature flag) |

### Design: egui-wgpu Overlay

**Approach**: Integrate `egui-wgpu` (the official egui wgpu bindings) as an overlay render pass that executes AFTER the 4D cross-section render pass.

Why egui over custom wgpu text rendering:
1. **Immediate mode**: Perfect for HUD that changes every frame (ammo counts, health bars)
2. **Rich widget set**: Text, sliders, progress bars, windows, panels -- all available immediately
3. **Input handling**: Mouse/keyboard input routing built-in (needed for menus)
4. **Debug UI**: Free debug panels for development (entity inspector, physics debug)
5. **Mature ecosystem**: Widely used in the Rust gamedev community
6. **wgpu integration**: `egui-wgpu` is maintained by the egui team, direct wgpu render pass

The tradeoff is dependency weight, but egui is already in the project's future plans (Phase 5 editor) so this front-loads the integration.

### Where It Fits in the Render Pipeline

Current pipeline:
```
[Compute: 4D Slice] -> [Render: 3D Cross-Section] -> Surface Present
```

New pipeline:
```
[Compute: 4D Slice] -> [Render: 3D Cross-Section] -> [Render: egui Overlay] -> Surface Present
```

Key implementation detail: The egui render pass uses `LoadOp::Load` (not `Clear`) for the color attachment, so it draws ON TOP of the existing 3D scene. No depth buffer needed for 2D overlay.

The current `RenderPipeline::render()` method takes `view: &wgpu::TextureView` as a parameter (line 253 of render_pipeline.rs). The egui pass takes the same `view` and renders after the 3D pass.

### Integration Point

A new `OverlayRenderer` struct in `rust4d_render` manages the egui integration:

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

### API Usage Pattern (Engine Side)

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

### What the GAME Builds On Top

```rust
// Game-side HUD implementation
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

### New Dependencies

Add to `rust4d_render/Cargo.toml`:
```toml
egui = "0.31"
egui-wgpu = "0.31"
egui-winit = "0.31"
```

These versions should align. The specific version should be whatever is current when implementation starts.

### Session Estimate: 1 session

- Integrate egui-wgpu into the render pipeline
- Create OverlayRenderer with begin/end frame pattern
- Forward winit events to egui
- Test with a simple "Hello World" overlay
- Document the game-side API pattern

---

## 3. Particle / Effect System -- Extension to `rust4d_render`

### Engine vs Game Boundary

| Responsibility | Owner |
|---|---|
| GPU billboard particle renderer | **Engine** (`rust4d_render`) |
| Particle emitter configuration (lifetime, velocity, color, size curves) | **Engine** (`rust4d_render`) |
| Particle system update (CPU-side simulation) | **Engine** (`rust4d_render`) |
| Specific effect presets (muzzle flash, blood, sparks, explosion) | **Game** |
| Triggering effects at positions and times | **Game** |

### Design: CPU-Simulated, GPU-Rendered Billboard Particles

For a boomer shooter, we need fast, cheap particles that look good. The approach:

1. **CPU simulation**: Particles are simple structs updated on the CPU. At the expected particle counts (hundreds, not millions), CPU simulation is more than fast enough and far simpler than GPU particle compute shaders.

2. **GPU rendering**: Particles are rendered as camera-facing billboards (quads that always face the camera). This requires a separate render pass with a different pipeline (no depth write, additive blending for fire/flash effects, alpha blending for smoke/blood).

3. **3D particles in the sliced space**: Particles exist in 3D (the sliced output space), NOT in 4D pre-slice space. This is a deliberate simplification -- particles are visual effects, not physical 4D objects. They spawn at 3D positions in the cross-section and simulate in 3D. This avoids the enormous complexity of 4D particle slicing while looking correct (a muzzle flash appears where the gun is in the 3D slice).

### Particle System Architecture

```rust
// In rust4d_render/src/particles/

/// A single particle
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

/// A particle emitter instance
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

### Render Pipeline Position

```
[Compute: 4D Slice]
    -> [Render: 3D Cross-Section] (depth write ON)
    -> [Render: Particles] (depth test ON, depth write OFF, blending ON)
    -> [Render: egui Overlay] (no depth)
    -> Surface Present
```

Particles use the depth buffer from the 3D render pass for occlusion (particles behind geometry are hidden), but do NOT write to it (particles don't occlude each other or geometry).

### Billboard Shader

A new WGSL shader for particle billboards:

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

### What the GAME Builds On Top

```rust
// Game-side effect definitions
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

- **Existing render pipeline**: Needs access to the depth texture view from `RenderPipeline::ensure_depth_texture()`. Currently `depth_texture` is private -- will need a getter method.
- **Foundation**: Fixed timestep for consistent particle simulation (dt must be consistent)
- **Phase 1 (P1)**: Event system for game to trigger particles on events (damage dealt -> blood)

### Session Estimate: 1.5-2 sessions

- Session 1: ParticleEmitter, ParticleSystem update logic, billboard shader, additive pipeline
- Session 2 (partial): Alpha blending pipeline, depth integration, burst spawning, cleanup/testing

---

## 4. Screen Effects / Post-Processing Framework -- Extension to `rust4d_render`

### Engine vs Game Boundary

| Responsibility | Owner |
|---|---|
| Post-processing effect stack (render-to-intermediate, fullscreen quad) | **Engine** (`rust4d_render`) |
| Screen shake (camera offset) | **Engine** (`rust4d_render` or `rust4d_game`) |
| Damage flash (color overlay) | **Engine** (`rust4d_render` via effect stack) |
| Specific effect triggers and parameters | **Game** |

### Design: Lightweight Effect Layer

For Phase 2, we do NOT need a full post-processing pipeline. The synthesis specifies "muzzle flash, screen shake, damage flash" at 0.5 session. These are achievable with a minimal approach:

#### Screen Shake

Screen shake is NOT a post-processing effect. It is a camera offset applied before the view matrix is computed. This belongs in the game's camera system or in `rust4d_game`:

```rust
/// Screen shake state (lives in game-side camera wrapper or rust4d_game)
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

#### Damage Flash / Color Overlay

A full-screen color tint can be achieved through the egui overlay (draw a semi-transparent colored rectangle over the entire screen) OR through a minimal post-process pass.

**Simpler approach (egui overlay)**:
```rust
// Game-side: draw a damage flash as an egui area
fn draw_damage_flash(ctx: &egui::Context, intensity: f32) {
    if intensity > 0.0 {
        egui::Area::new("damage_flash")
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let color = egui::Color32::from_rgba_unmultiplied(255, 0, 0, (intensity * 128.0) as u8);
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

This is adequate for Phase 2 and avoids building post-processing infrastructure. A proper post-processing pipeline (render to intermediate texture, apply fullscreen shader) can be added in Phase 5 when bloom and other effects are needed.

**Engine-side support needed**: None beyond the egui overlay already provided. The game handles damage flash, pickup flash, and similar effects entirely through egui painting.

#### Muzzle Flash Lighting

Muzzle flash has two components:
1. **Particle effect**: Handled by the particle system (see section 3)
2. **Light flash**: A brief increase in ambient lighting or a point light at the muzzle position

For Phase 2, the simplest approach is a brief ambient light boost:

```rust
// Game-side: temporarily increase ambient strength in RenderUniforms
fn apply_muzzle_flash(&mut self, intensity: f32) {
    // Boost ambient_strength in RenderUniforms for 1-2 frames
    self.render_uniforms.ambient_strength = 0.3 + intensity * 0.5;
}
```

This doesn't require engine changes -- the game already controls `RenderUniforms`. Proper point lights come in Phase 5.

### What the Engine DOES Need (Minimal)

1. **Expose depth texture**: Add a `pub fn depth_texture_view(&self) -> Option<&wgpu::TextureView>` method to `RenderPipeline` for the particle system to use.

2. **Screen shake in `rust4d_game`**: The `ScreenShake` struct described above.

3. **Effect timing helpers in `rust4d_game`**: A simple `TimedEffect` helper:

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

### Session Estimate: 0.5 session

- Screen shake struct in `rust4d_game` (trivial)
- TimedEffect helper in `rust4d_game` (trivial)
- Depth texture getter on RenderPipeline (one-liner)
- All visual effects (damage flash, muzzle flash) are game-side using egui overlay + existing RenderUniforms

---

## 5. Complete Pipeline After Phase 2

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
  +--> [Render: Particles]                              [NEW]
  |    (reads depth buffer, no depth write)
  |
  +--> OverlayRenderer.begin_frame()                    [NEW]
  |    Game builds HUD via egui context
  |    Game draws damage flash / effects via egui
  |    OverlayRenderer.render()
  |
  +--> Surface Present
```

---

## 6. Crate Organization Summary

### New Crate

| Crate | Purpose | Dependencies |
|---|---|---|
| `rust4d_audio` | 4D spatial audio engine wrapping kira | `rust4d_math`, `kira`, `log`, `serde` |

### Modified Crates

| Crate | Changes |
|---|---|
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

## 7. Dependency Map

```
Foundation (fixed timestep, serialization)
  |
  +--> P1: Event System (engine events for triggering sounds/particles)
  |      |
  |      +--> P2: Audio System (game triggers sounds via events)
  |      +--> P2: Particle System (game spawns particles via events)
  |
  +--> ECS Split (rust4d_game crate exists)
         |
         +--> P2: ScreenShake (in rust4d_game)
         +--> P2: TimedEffect (in rust4d_game)

No dependency:
  - HUD/egui overlay can be built independently (no Foundation or P1 dependency)
  - Audio core (non-spatial) can be built independently
```

### Parallelism Opportunities

These P2 tasks are independent of each other:

```
Wave P2-1 (Parallel):
  ├── Agent A: rust4d_audio crate (audio engine + spatial)
  ├── Agent B: OverlayRenderer (egui-wgpu integration)
  └── Agent C: ParticleSystem (emitters + billboard renderer)

Wave P2-2 (Sequential, after P2-1):
  └── Agent D: Integration (ScreenShake, TimedEffect, wire everything into render loop)
```

### Dependencies on Other Phases

| Dependency | Phase | Blocking? | Notes |
|---|---|---|---|
| Fixed timestep | Foundation | **Yes** for particles | Particle simulation needs consistent dt |
| Serialization (serde) | Foundation | No (nice-to-have) | Audio/particle configs could use serialization but not blocking |
| Event system | P1 | **Soft** | Game can trigger audio/particles directly without events; events make it cleaner |
| Raycasting | P1 | No | Audio/HUD/particles don't need raycasting |
| ECS / rust4d_game crate | Split Plan | **Yes** for ScreenShake/TimedEffect | These go in rust4d_game which must exist |
| Sprite rendering | P3 | No | Particles are billboards, not sprites |

---

## 8. Session Estimates Summary

| Task | Sessions | Notes |
|---|---|---|
| `rust4d_audio` core (init, load, play, bus routing) | 1 | Kira wrapper, basic playback |
| `rust4d_audio` spatial (listener, 4D attenuation, W-filtering) | 0.5-1 | 4D spatial projection onto kira's 3D system |
| `OverlayRenderer` (egui-wgpu integration) | 1 | egui render pass, input forwarding |
| `ParticleSystem` (emitters, billboard shader, GPU pipeline) | 1.5-2 | CPU sim + GPU billboards + two blend modes |
| Screen effects (ScreenShake, TimedEffect, depth getter) | 0.5 | Small structs in rust4d_game |
| **Total Engine Work** | **4.5-5.5** | |

### What the Game Builds (NOT in these estimates)

| Task | Sessions | Notes |
|---|---|---|
| Weapon system (hitscan + projectile, ammo, switching) | 2 | Entirely game-side |
| HUD widgets (health, ammo, crosshair, W-indicator) | 0.5 | Uses OverlayRenderer's egui context |
| Sound assets and trigger logic | 0.5 | Game-side audio integration |
| Effect presets (muzzle flash, blood, explosion configs) | 0.5 | ParticleEmitterConfig definitions |
| Damage flash, muzzle flash lighting | 0.25 | Game-side egui overlay + ambient boost |
| **Total Game Work** | **~3.75** | |

---

## 9. Open Questions / Risks

### Open Questions

1. **Kira version**: The API surface changed between kira 0.8 and 0.9. Need to verify the latest stable version when implementing. The spatial track API is what we need.

2. **egui version alignment**: egui, egui-wgpu, and egui-winit versions must match exactly. Need to check compatibility with the winit version already in the workspace (`winit = "0.30"`).

3. **Particle count limits**: How many particles can we sustain at 60fps? The billboard approach with instanced rendering should handle thousands easily, but worth testing. For a boomer shooter, we probably need 200-500 simultaneous particles max.

4. **4D audio W-filtering**: Should W-distance filtering be a low-pass filter or just volume attenuation? Low-pass filtering requires kira's effect system per spatial track. Volume-only is simpler and might be sufficient for an initial implementation.

### Risks

1. **egui-winit compatibility**: The engine uses `winit = "0.30"`. Need to verify `egui-winit` supports this version. winit has frequent breaking changes.

2. **kira threading model**: Kira uses a separate audio thread. Commands are sent from the game thread. This is generally fine but the `AudioEngine4D` wrapper must be designed to handle this cleanly (all methods should be non-blocking).

3. **Particle depth interaction**: Particles need to read the depth buffer for occlusion against scene geometry. This requires the depth texture to have `TEXTURE_BINDING` usage -- which it already does (render_pipeline.rs line 238: `usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING`). Good.

4. **Performance budget**: Adding three new render passes (particles, egui, potential post-process) to every frame. Each pass has overhead. The egui pass only draws when there is content. The particle pass should be skipped when no emitters are active. Need to be mindful of GPU overhead.

---

## 10. Key Design Decisions and Rationale

### Why NOT a `rust4d_particles` crate?

Particles are tightly coupled to the rendering pipeline (they need the wgpu device, the depth texture, the view/projection matrices, and a render pass slot in the frame). Putting them in `rust4d_render` keeps all GPU code together and avoids cross-crate GPU resource sharing complexity.

### Why NOT custom wgpu text rendering for HUD?

Custom text rendering requires: font loading, glyph rasterization, text layout, texture atlas management, and a text render pipeline. This is weeks of work for a worse result than egui provides out of the box. egui also gives us debug UI and menu systems "for free" which will be used extensively.

### Why CPU-simulated particles instead of GPU compute?

At the particle counts a boomer shooter needs (hundreds), CPU simulation is simpler, debuggable, and fast enough. GPU compute particles shine at millions of particles (smoke simulations, fluid effects) which we don't need. CPU simulation also makes it trivial to query particle state (e.g., "is this emitter done?") without GPU readback.

### Why 3D particles instead of 4D?

Particles are visual effects that enhance the 3D cross-section the player sees. A muzzle flash doesn't exist in 4D space -- it's a visual feedback element in the player's 3D viewport. Making particles 4D objects that get sliced would:
- Require running them through the compute slice pipeline (expensive)
- Make them subject to W-distance visibility (a muzzle flash could disappear if the player's slice shifts slightly)
- Add complexity for zero gameplay benefit

3D particles in the sliced output space is the correct abstraction for visual effects.

### Why kira over rodio?

See section 1. Summary: kira has spatial audio, tweens, mixer/tracks, and precise timing -- all game audio essentials. rodio is simpler but would require building these features ourselves.
