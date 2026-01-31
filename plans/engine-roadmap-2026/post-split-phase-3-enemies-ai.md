# Post-Split Phase 3: Enemies & AI -- Engine Implementation Plan

**Date**: 2026-01-31
**Source**: Agent P3 report from engine roadmap swarm (2026-01-30)
**Status**: Planning document -- ready for implementation

---

## 1. Overview

Phase 3 provides the engine-side infrastructure needed for enemies and AI in any 4D game. After the engine/game split, the boundary is clear: the engine provides **sprite/billboard rendering, spatial queries, a generic FSM framework, area damage math, and a particle system**. The game builds **specific enemy types, AI behaviors, spawn logic, and damage application** on top of these primitives.

**Total engine work**: 4.0 sessions (critical path: 1.5 sessions with full parallelism)

### Prerequisites
- Engine/game split complete (ECS migration done, `rust4d_game` crate exists)
- Foundation phase complete (fixed timestep, serialization fixes)
- Phase 1 Combat Core partially complete (raycasting needed for LOS; can stub until ready)

### What This Phase Delivers
- Sprite/billboard rendering pipeline with 4D W-distance fade
- Particle system with multiple blend modes
- Spatial query API (hypersphere queries, area effects)
- Generic finite state machine framework
- Impulse/knockback support on physics bodies
- Line-of-sight wrapper (depends on P1 raycasting)

---

## 2. Engine vs Game Boundary

### Engine Provides (Generic, Reusable)

| Feature | Crate | Purpose |
|---------|-------|---------|
| `SpriteBatch`, `SpritePipeline` | `rust4d_render` | Billboard rendering at 4D positions with W-fade |
| `SpriteAnimation` | `rust4d_render` | Frame-based animation ticker |
| `ParticleSystem`, `ParticleEmitter` | `rust4d_render` | Generic particle emission, simulation, rendering |
| `query_sphere()`, `query_area_effect()` | `rust4d_physics` | Spatial queries in 4D hyperspherical volumes |
| `line_of_sight()` | `rust4d_physics` | LOS check wrapping P1's raycasting |
| `apply_impulse()` | `rust4d_physics` | Velocity impulse on physics bodies |
| `StateMachine<S>` | `rust4d_game` | Generic FSM with state timing |

### Game Builds (Shooter-Specific)

| Feature | Description |
|---------|-------------|
| `EnemyState` enum | Idle/Chase/Attack/Pain/Dead states |
| `EnemyAI` struct | Per-enemy AI logic using FSM + spatial queries |
| `EnemyDef` data | Enemy type definitions (health, speed, sprite sheets) |
| `WBehavior` enum | W-phasing, W-flanking behaviors |
| Spawn system | Where/when enemies appear |
| Damage application | `apply_explosion()` using `query_area_effect()` |
| Particle effect presets | Blood, muzzle flash, explosion configs |
| Enemy sprite sheets | Art assets and animation frame ranges |

### Boundary Principle
Enemy types, specific AI behaviors, and enemy-specific game logic are 100% game-side. The engine provides sprite rendering, spatial queries, FSM framework, and area damage math. The engine has zero knowledge of what an "enemy" is.

---

## 3. Sub-Phase A: Sprite/Billboard Rendering Pipeline

**Session estimate: 1.5 sessions**

### The 4D-Specific Challenge

Sprites are NOT 4D geometry. They do not go through the compute-shader slicing pipeline. Instead:

1. Entities exist in 4D (`Vec4` position with XYZW coordinates)
2. The camera views a 3D hyperplane slice of 4D space
3. A sprite at a different W than the slice may be partially visible, fully visible, or invisible
4. "Camera-facing" means facing the 3D camera *within the slice space*, not in 4D

**Key insight**: Sprites are **3D billboard quads rendered at the 3D projection of a 4D position**, with visibility/opacity modulated by W-distance from the slice plane.

### Two-Pass Rendering Architecture

The current pipeline has one flow:

```
4D Tetrahedra -> [Compute: Slice] -> 3D Triangles -> [Render: Forward Pass]
```

Sprites add a second flow:

```
Sprite Entities -> [CPU: Project to Slice Space] -> 3D Billboard Quads -> [Render: Billboard Pass]
```

Both passes write to the same framebuffer and **share the depth buffer**, so sprites correctly occlude and are occluded by 4D geometry.

### Engine API: `rust4d_render::sprite`

```rust
/// A sprite definition (texture + animation frames)
pub struct SpriteSheet {
    /// GPU texture handle
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    /// Frame layout: rows x cols in the texture atlas
    frame_cols: u32,
    frame_rows: u32,
    /// Size of each frame in pixels
    frame_width: u32,
    frame_height: u32,
}

/// A single sprite instance to render
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    /// Position in 3D slice space (already projected from 4D)
    pub position: [f32; 3],
    /// Width and height in world units
    pub size: [f32; 2],
    /// UV offset for the current animation frame
    pub uv_offset: [f32; 2],
    /// UV scale (1/cols, 1/rows)
    pub uv_scale: [f32; 2],
    /// RGBA tint/modulation (includes W-distance fade in alpha)
    pub color: [f32; 4],
}

/// Manages sprite rendering for a frame
pub struct SpriteBatch {
    instances: Vec<SpriteInstance>,
    sprite_sheet: SpriteSheet,
}

impl SpriteBatch {
    /// Add a sprite at a 4D position.
    /// Calculates 3D projection and W-distance fade automatically.
    pub fn add_sprite_4d(
        &mut self,
        position_4d: Vec4,
        slice_w: f32,
        camera_matrix: &[[f32; 4]; 4],
        size: [f32; 2],
        frame: u32,
        tint: [f32; 4],
        w_fade_range: f32,
    );

    /// Add a sprite already in 3D slice space (for HUD elements, etc.)
    pub fn add_sprite_3d(
        &mut self,
        position_3d: [f32; 3],
        size: [f32; 2],
        frame: u32,
        tint: [f32; 4],
    );
}
```

### Billboard Orientation in 4D Slice Space

When adding a sprite at 4D position `P = (x, y, z, w)`:

1. **Transform P into camera-local 4D space** using the inverse camera matrix
2. **Compute W-distance**: `w_dist = |P_camera.w - slice_w|`
3. **Project to 3D**: The sprite's 3D position is `(P_camera.x, P_camera.y, P_camera.z)` -- the W component is discarded after computing fade
4. **W-distance fade**: `alpha *= max(0, 1 - w_dist / w_fade_range)`. Sprites far in W fade to invisible
5. **Billboard construction**: In the vertex shader, expand the point into a camera-facing quad using the view matrix's right and up vectors

### Billboard WGSL Shader (`sprite_billboard.wgsl`)

```wgsl
let right = vec3(view[0][0], view[1][0], view[2][0]);
let up    = vec3(view[0][1], view[1][1], view[2][1]);
let corner = instance.position
    + right * (vertex_offset.x * instance.size.x)
    + up    * (vertex_offset.y * instance.size.y);
```

### W-Distance Visibility Rules

| W-Distance | Visual Effect | Can Be Hit? |
|------------|---------------|-------------|
| 0.0 - 0.5 | Fully visible | Yes |
| 0.5 - 1.5 | Ghosted (alpha fade) | With splash damage |
| 1.5 - 3.0 | Faint shimmer | Audio cue only |
| > 3.0 | Invisible | No |

These thresholds are configurable per sprite. The engine provides the fade calculation; the game decides the thresholds.

### Sprite Pipeline Integration

```rust
/// The sprite render pipeline (new module in rust4d_render)
pub struct SpritePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    instance_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

impl SpritePipeline {
    /// Create the sprite pipeline (shares depth buffer with main render pipeline)
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self;

    /// Render all sprite batches
    /// Called AFTER the main render pass, using the SAME depth buffer
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        batches: &[SpriteBatch],
        uniforms: &RenderUniforms,
    );
}
```

**Depth buffer sharing**: The sprite pipeline shares the depth buffer with the main 4D-slice render pipeline. This means sprites are correctly depth-tested against sliced 4D geometry. Sprites rendered at the same 3D position as a wall will be occluded by the wall.

**Integration point**: `RenderPipeline::ensure_depth_texture()` needs to expose the depth buffer for sprites and particles to share.

**Two-sub-pass approach for transparency**:
- Sub-pass 1: Opaque sprites with **alpha testing** (not alpha blending), depth-write enabled
- Sub-pass 2: Ghosted/faded sprites with **alpha blending**, depth-write disabled
- This prevents sorting issues with transparent sprites

### Sprite Animation

```rust
/// Sprite animation state (engine-provided, game drives state transitions)
pub struct SpriteAnimation {
    /// Current frame index
    pub current_frame: u32,
    /// Animation playback speed (frames per second)
    pub fps: f32,
    /// Accumulated time
    elapsed: f32,
    /// Frame sequence for this animation
    pub frames: Vec<u32>,
    /// Whether to loop
    pub looping: bool,
    /// Whether animation has finished (for non-looping)
    pub finished: bool,
}

impl SpriteAnimation {
    pub fn new(frames: Vec<u32>, fps: f32, looping: bool) -> Self;
    pub fn update(&mut self, dt: f32);
    pub fn current_frame(&self) -> u32;
    pub fn reset(&mut self);
}
```

The engine provides the animation ticker. The game defines which frames correspond to which states (e.g., idle frames 0-3, walk frames 4-11, attack frames 12-15, pain frame 16, death frames 17-21).

### Sub-Phase A Session Breakdown

| Task | Sessions | Details |
|------|----------|---------|
| SpriteSheet loading, SpriteInstance type, SpriteBatch | 0.5 | Data types and CPU-side logic |
| SpritePipeline (wgpu render pipeline, WGSL billboard shader) | 0.5 | New GPU pipeline alongside slice pipeline |
| W-distance fade integration, depth buffer sharing, SpriteAnimation | 0.5 | Integration with existing render pipeline |

### New Files

| File | Crate | Description |
|------|-------|-------------|
| `sprite.rs` | `rust4d_render` | SpriteSheet, SpriteInstance, SpriteBatch |
| `sprite_pipeline.rs` | `rust4d_render` | SpritePipeline (wgpu render pipeline) |
| `sprite_billboard.wgsl` | `rust4d_render` | Billboard vertex/fragment shader |
| `sprite_animation.rs` | `rust4d_render` | SpriteAnimation state |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_render/src/lib.rs` | Export new sprite modules |
| `rust4d_render/src/pipeline/mod.rs` | Export SpritePipeline |
| `rust4d_render/src/pipeline/render_pipeline.rs` | Expose depth texture via `ensure_depth_texture()` for shared depth buffer |

---

## 4. Sub-Phase B: Spatial Queries

**Session estimate: 0.5 sessions** (combined with Sub-Phase D)

### Design

`PhysicsWorld` needs spatial query methods for finding entities within a radius. These are used by AI awareness, area damage, and proximity triggers.

### API: `rust4d_physics::spatial_query`

```rust
/// Result of a spatial query
pub struct SpatialQueryResult {
    /// The body key that was found
    pub body_key: BodyKey,
    /// 4D distance from the query origin
    pub distance: f32,
    /// Position of the found body
    pub position: Vec4,
}

impl PhysicsWorld {
    /// Query all bodies within a 4D hypersphere of the given center and radius.
    /// Optionally filter by collision layer.
    pub fn query_sphere(
        &self,
        center: Vec4,
        radius: f32,
        layer_filter: Option<CollisionLayer>,
    ) -> Vec<SpatialQueryResult>;

    /// Query all bodies within radius, sorted by distance (nearest first).
    pub fn query_sphere_sorted(
        &self,
        center: Vec4,
        radius: f32,
        layer_filter: Option<CollisionLayer>,
    ) -> Vec<SpatialQueryResult>;

    /// Check line-of-sight between two points.
    /// Returns true if no STATIC geometry blocks the 4D ray.
    /// (Requires raycasting from P1 -- this is the LOS API the game uses.)
    pub fn line_of_sight(
        &self,
        from: Vec4,
        to: Vec4,
        block_layers: CollisionLayer,
    ) -> bool;
}
```

### Implementation Approach

- `query_sphere()` iterates all bodies and checks `(center - body.position).length() < radius` using 4D Euclidean distance
- **O(n) complexity** -- perfectly adequate for boomer shooter enemy counts (20-50 active enemies)
- If performance becomes an issue later, a 4D spatial hash grid can be added without API changes
- `line_of_sight()` depends on P1's raycasting (`PhysicsWorld::raycast()`); casts a 4D ray from `from` to `to` and returns false if it hits any body on `block_layers`
- **Stub strategy**: Until P1 delivers raycasting, LOS returns `true` (all enemies always see the player)

### Layer Mask Filtering

Per P1's coordination note: `CollisionLayer::STATIC | CollisionLayer::PLAYER` for LOS checks that see through enemies. The layer mask parameter provides full flexibility.

### New Files

| File | Crate | Description |
|------|-------|-------------|
| `spatial_query.rs` | `rust4d_physics` | SpatialQueryResult, query_sphere, query_area_effect, apply_impulse |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_physics/src/world.rs` | Add query_sphere, query_area_effect, line_of_sight, apply_impulse methods |
| `rust4d_physics/src/lib.rs` | Export new spatial query types |

---

## 5. Sub-Phase C: FSM Framework

**Session estimate: included in Sub-Phase B's 0.5 sessions** (FSM is ~30 lines)

**Location**: `rust4d_game` crate (not engine-core, since it is a game framework pattern)

### Design

Intentionally minimal. Boomer shooter AI does not need hierarchical states, parallel states, or transition guards. The game implements the actual state logic.

```rust
/// A generic finite state machine
///
/// Type parameters:
/// - S: State enum (e.g., EnemyState)
pub struct StateMachine<S: Copy + Eq + std::hash::Hash> {
    current_state: S,
    previous_state: S,
    time_in_state: f32,
}

impl<S: Copy + Eq + std::hash::Hash> StateMachine<S> {
    pub fn new(initial_state: S) -> Self;

    /// Transition to a new state. Returns true if state actually changed.
    pub fn transition(&mut self, new_state: S) -> bool;

    /// Update the timer. Called each frame.
    pub fn update(&mut self, dt: f32);

    /// Get current state
    pub fn current(&self) -> S;

    /// Get previous state (useful for transition logic)
    pub fn previous(&self) -> S;

    /// How long we've been in the current state
    pub fn time_in_state(&self) -> f32;

    /// Check if we just entered this state (time_in_state < dt threshold)
    pub fn just_entered(&self) -> bool;
}
```

### Why This Is Enough

The game uses the FSM like this:

```rust
// Game-side usage
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EnemyState { Idle, Chase, Attack, Pain, Dead }

struct EnemyAI {
    fsm: StateMachine<EnemyState>,
    // ... game-specific fields
}

impl EnemyAI {
    fn update(&mut self, dt: f32, world: &PhysicsWorld, player_pos: Vec4) {
        self.fsm.update(dt);
        match self.fsm.current() {
            EnemyState::Idle => self.update_idle(world, player_pos),
            EnemyState::Chase => self.update_chase(dt, world, player_pos),
            EnemyState::Attack => self.update_attack(dt, world, player_pos),
            EnemyState::Pain => self.update_pain(dt),
            EnemyState::Dead => { /* despawn after death animation */ },
        }
    }
}
```

The `match` on `fsm.current()` IS the state logic. No engine framework needed beyond the timer and transition tracking.

### New Files

| File | Crate | Description |
|------|-------|-------------|
| `fsm.rs` | `rust4d_game` | StateMachine\<S\> |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_game/src/lib.rs` | Export FSM module |

---

## 6. Sub-Phase D: Area Damage / Hyperspherical Explosions

**Session estimate: 0.5 sessions** (combined with Sub-Phase B spatial queries)

### Design

Area damage queries ("what got hit by this explosion?") are a physics-level spatial query. The engine provides the query; the game applies damage.

```rust
/// Result of an area effect query, with distance falloff
pub struct AreaEffectHit {
    /// The body key that was hit
    pub body_key: BodyKey,
    /// 4D distance from the explosion center
    pub distance: f32,
    /// Falloff multiplier (1.0 at center, 0.0 at edge of radius)
    pub falloff: f32,
    /// Position of the hit body
    pub position: Vec4,
    /// Direction from center to hit body (normalized)
    pub direction: Vec4,
}

impl PhysicsWorld {
    /// Query a hyperspherical area effect.
    /// Returns all bodies within `radius` of `center`, with linear distance falloff.
    /// Optionally checks LOS from center to each body (requires raycasting from P1).
    pub fn query_area_effect(
        &self,
        center: Vec4,
        radius: f32,
        layer_filter: Option<CollisionLayer>,
        require_los: bool,
    ) -> Vec<AreaEffectHit>;
}
```

### 4D-Specific: Hypersphere Volume Scaling

In 4D, a hypersphere of radius R encloses volume V = (pi^2 / 2) * R^4, compared to a 3D sphere's V = (4/3) * pi * R^3. This means:

- **4D explosions cover MORE volume** than equivalently-sized 3D explosions
- Explosions catch more enemies in adjacent W-slices
- This is a **deliberate gameplay advantage** for explosive weapons -- they are the counter to W-phasing enemies
- A W-phaser that shifts to an adjacent slice to avoid hitscan is still within a hyperspherical explosion radius

### Knockback as Impulse

The engine already supports velocity modification on bodies. Knockback is a thin convenience method:

```rust
impl PhysicsWorld {
    /// Apply an impulse to a body (velocity change = force / mass)
    pub fn apply_impulse(&mut self, key: BodyKey, impulse: Vec4) {
        if let Some(body) = self.get_body_mut(key) {
            if !body.is_static() {
                body.velocity += impulse * (1.0 / body.mass);
            }
        }
    }
}
```

### Game-Side Usage

```rust
// In game code
fn apply_explosion(world: &PhysicsWorld, center: Vec4, radius: f32, base_damage: f32) {
    let hits = world.query_area_effect(
        center, radius,
        Some(CollisionLayer::ENEMY | CollisionLayer::PLAYER),
        true, // require LOS (no damage through walls)
    );

    for hit in &hits {
        let damage = base_damage * hit.falloff;
        apply_damage(hit.body_key, damage);
        apply_knockback(hit.body_key, hit.direction, base_damage * hit.falloff * 10.0);
    }
}
```

---

## 7. Particle System

**Session estimate: 1.5 sessions**

### Design Philosophy

The particle system is an engine rendering feature. The engine provides a generic emitter/updater/renderer. The game defines specific effects (blood, sparks, explosions, muzzle flash).

### Key Decision: Particles Live in 3D, Not 4D

Particles exist in 3D slice space (like sprites), not in 4D. They are spawned at 3D positions derived from 4D events:

```
4D Event (explosion at Vec4) -> Project to 3D -> Spawn 3D particles
```

This is the pragmatic choice. Simulating individual particles in 4D and slicing would be computationally expensive and visually indistinguishable from 3D particles at the projected position.

### Shared with Phase 2 (Weapons)

The particle system is needed by both P2 (muzzle flash, impact sparks) and P3 (blood, explosions, death effects). P2 confirmed in the hive-mind: single shared particle system in `rust4d_render`, coordinated API. Per P2: use `ParticleSystem::spawn_burst()` for one-shot effects.

### Engine API: `rust4d_render::particles`

```rust
/// Configuration for a particle emitter
#[derive(Clone, Debug)]
pub struct ParticleEmitterConfig {
    /// Maximum number of live particles
    pub max_particles: u32,
    /// Particle lifetime range (seconds)
    pub lifetime: (f32, f32),
    /// Initial speed range
    pub speed: (f32, f32),
    /// Spread angle (radians, 0 = focused beam, PI = hemisphere, 2*PI = sphere)
    pub spread: f32,
    /// Gravity applied to particles (typically (0, -9.8, 0) or (0,0,0))
    pub gravity: [f32; 3],
    /// Start size range
    pub size_start: (f32, f32),
    /// End size range (particles shrink/grow over lifetime)
    pub size_end: (f32, f32),
    /// Start color (RGBA)
    pub color_start: [f32; 4],
    /// End color (RGBA, alpha fades out)
    pub color_end: [f32; 4],
    /// Emission rate (particles per second, 0 = burst mode only)
    pub emission_rate: f32,
    /// Drag coefficient (0 = no drag, 1 = instant stop)
    pub drag: f32,
}

/// A single particle (internal state)
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    position: [f32; 3],
    velocity: [f32; 3],
    color: [f32; 4],
    size: f32,
    lifetime: f32,       // Total lifetime
    age: f32,            // Current age
    _padding: f32,
}

/// A particle emitter instance
pub struct ParticleEmitter {
    config: ParticleEmitterConfig,
    particles: Vec<Particle>,
    /// Position in 3D slice space
    pub position: [f32; 3],
    /// Direction for directional emissions
    pub direction: [f32; 3],
    /// Whether this emitter is actively spawning
    pub active: bool,
    /// Accumulated time for emission rate
    emit_accumulator: f32,
}

impl ParticleEmitter {
    pub fn new(config: ParticleEmitterConfig) -> Self;

    /// Burst-emit a number of particles at once
    pub fn burst(&mut self, count: u32);

    /// Update all particles (age, move, apply gravity, remove dead)
    pub fn update(&mut self, dt: f32);

    /// Get the current particle instances for rendering
    pub fn instances(&self) -> &[Particle];

    /// Whether all particles are dead and emitter is inactive
    pub fn is_finished(&self) -> bool;
}

/// Manages all particle emitters and renders them
pub struct ParticleSystem {
    emitters: Vec<ParticleEmitter>,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

impl ParticleSystem {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self;

    /// Add an emitter and return its index
    pub fn add_emitter(&mut self, emitter: ParticleEmitter) -> usize;

    /// Remove an emitter by index
    pub fn remove_emitter(&mut self, index: usize);

    /// Update all emitters
    pub fn update(&mut self, dt: f32);

    /// Remove all finished emitters
    pub fn cleanup_finished(&mut self);

    /// Render all particles
    /// Shares depth buffer with main render pipeline (read-only, no depth write)
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        uniforms: &RenderUniforms,
    );
}
```

### Rendering Approach

- Rendered as **instanced quads** (camera-facing billboards, same technique as sprites)
- **Additive blending** for fire/sparks/energy effects
- **Alpha blending** for smoke/dust/blood
- Blend mode is **per-emitter**
- **Depth-read, no depth-write**: particles behind geometry are correctly hidden, but particles do not occlude each other (prevents z-fighting) or occlude sprites/geometry
- CPU-simulated (hundreds of particles, not millions) with GPU-rendered billboards

### Particle WGSL Shader (`particle.wgsl`)

Uses the same billboard expansion technique as sprites. Per-particle color and size are passed via instance buffer. Blend mode controlled by pipeline state (additive vs alpha).

### Game-Side Effect Presets (Examples)

```rust
// Muzzle flash: 20 particles, bright yellow -> fading orange, 0.05-0.15s lifetime
// Blood: 30 particles, dark red -> fading, hemisphere spread, gravity, 0.3-0.8s
// Explosion: 100 particles, bright orange -> gray smoke, full sphere, 0.2-1.0s
```

### Sub-Phase Session Breakdown

| Task | Sessions | Details |
|------|----------|---------|
| ParticleEmitter, Particle struct, CPU simulation (update loop) | 0.5 | Data types, emission, aging, gravity, drag |
| GPU particle pipeline (wgpu render pipeline, billboard shader, instancing) | 0.5 | New render pipeline with blend modes |
| ParticleSystem manager, blending modes, depth buffer integration | 0.5 | Integration with existing depth buffer |

### New Files

| File | Crate | Description |
|------|-------|-------------|
| `particles.rs` | `rust4d_render` | ParticleEmitter, ParticleEmitterConfig, Particle |
| `particle_pipeline.rs` | `rust4d_render` | ParticleSystem (wgpu render pipeline) |
| `particle.wgsl` | `rust4d_render` | Particle vertex/fragment shader |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_render/src/lib.rs` | Export particle modules |
| `rust4d_render/src/pipeline/mod.rs` | Export ParticleSystem |

---

## 8. Session Estimates Summary

| Sub-Phase | Engine Sessions | Notes |
|-----------|----------------|-------|
| A: Sprite/Billboard rendering | 1.5 | New render pipeline, WGSL shader, W-distance fade |
| B: Spatial queries | 0.5 | query_sphere + query_sphere_sorted + line_of_sight (combined with D) |
| C: FSM framework | -- | Included in B (FSM is ~30 lines) |
| D: Area damage / hyperspherical explosions | 0.5 | query_area_effect + AreaEffectHit + apply_impulse (combined with B) |
| Particle system | 1.5 | Emitter, renderer, pipeline, blending modes |
| **Total Engine** | **4.0** | |

### Game-Side Work (NOT Engine, for reference)

| Feature | Game Sessions | Engine Dependency |
|---------|---------------|-------------------|
| Enemy AI state machine logic | 0.5 | FSM (engine), raycasting (P1) |
| 3 enemy types (rusher, gunner, phaser) | 1.0 | Sprites, FSM, physics body, spatial queries |
| Enemy spawn system | 0.5 | ECS (foundation), trigger system (P1) |
| Specific particle effects (blood, explosions) | 0.5 | Particle system (engine) |
| W-phasing enemy behavior | 0.5 | Sprite W-fade (engine), physics (engine) |
| **Total Game** | **3.0** | |

---

## 9. Dependencies

### On Phase 1: Combat Core (Agent P1)

| P3 Needs | P1 Provides | Blocking? |
|----------|-------------|-----------|
| Line-of-sight checks | 4D Raycasting system (`Ray4D`, world raycast) | **Yes** -- LOS is core to AI |
| Area damage LOS checks | Same raycasting | **Yes** -- explosions should not damage through walls |
| Event system for damage | Event bus / collision callbacks | **Partial** -- AI needs "I was hit" events for pain state |

**Mitigation**: FSM, sprite rendering, and particle system can all be built without raycasting. LOS is stubbed as `true` (all enemies always see the player) until P1 delivers raycasting.

### On Phase 2: Weapons & Feedback (Agent P2)

| P3 Needs | P2 Provides | Blocking? |
|----------|-------------|-----------|
| Audio for enemy sounds | Audio system (kira integration) | **No** -- can be added later |
| HUD for W-proximity alerts | HUD framework | **No** -- can be added later |

**Shared work**: Particle system is needed by both P2 and P3. P3 designs it as the more comprehensive consumer; P2 coordinates on API. The particle system should be implemented before or in parallel with P2's weapon feedback work.

### On Foundation (Agent F)

| P3 Needs | Foundation Provides | Blocking? |
|----------|-------------------|-----------|
| Fixed timestep | Deterministic AI update tick | **Yes** -- AI timers need consistent dt |
| ECS components | Enemy components (Health, AIState, etc.) | **Assumed complete** per split plan |

---

## 10. Parallelization

All Wave 2 engine items can run in parallel. The total critical path is **1.5 sessions** (the longest of the three parallel streams).

```
Wave 1 (Foundation dependencies -- must complete first)
  Requires: Fixed timestep (Agent F), ECS migration complete

Wave 2 (Engine Phase 3 -- can start after Foundation)
  |
  +-- Agent/Session A: Sprite rendering pipeline (1.5 sessions)
  |   1. SpriteSheet + SpriteInstance types
  |   2. SpritePipeline + billboard WGSL shader
  |   3. W-distance fade, shared depth buffer, SpriteAnimation
  |
  +-- Agent/Session B: Particle system (1.5 sessions)  [PARALLEL with A]
  |   1. ParticleEmitter + Particle simulation
  |   2. GPU particle pipeline + WGSL shader
  |   3. ParticleSystem manager, blending, depth integration
  |
  +-- Agent/Session C: Physics queries + FSM (1 session)  [PARALLEL with A & B]
      1. StateMachine<S> in rust4d_game
      2. query_sphere, query_area_effect, apply_impulse in rust4d_physics
      3. line_of_sight (depends on P1 raycasting; stub if not ready)

Wave 3 (Game-side, after engine Wave 2)
  Enemy types, AI logic, spawn system, particle effect presets
  (3 sessions, game repo, NOT engine work)
```

---

## 11. Render Pass Ordering

After Phase 3 work, the complete rendering pipeline:

```
Frame Start
    |
    v
[Compute Pass: 4D Slice]
    4D tetrahedra -> 3D triangles (existing pipeline)
    |
    v
[Render Pass 1: 3D Geometry]  (existing render_pipeline.rs)
    Draws sliced 3D triangles with lighting, W-depth coloring
    Writes to color buffer + depth buffer
    |
    v
[Render Pass 2: Sprites/Billboards]  (NEW: sprite_pipeline.rs)
    Draws camera-facing quads for entities at 4D positions
    Reads depth buffer (occlusion), writes depth (opaque sprites)
    Uses W-distance fade for cross-slice visibility
    Two sub-passes: opaque (alpha test, depth write) then transparent (alpha blend, no depth write)
    |
    v
[Render Pass 3: Particles]  (NEW: particle_pipeline.rs)
    Draws particle quads with additive or alpha blending
    Reads depth buffer (occlusion), does NOT write depth
    |
    v
[Render Pass 4: egui Overlay / HUD]  (from Agent P2)
    2D overlay rendering (health, ammo, crosshair, W-indicator)
    No depth testing
    |
    v
[Render Pass 5: egui Editor]  (from Agent P5, if editor active)
    Editor UI overlay (last, on top of everything)
    |
    v
Frame Present
```

Confirmed by P2 and P5 in hive-mind coordination.

---

## 12. Verification Criteria

### Sub-Phase A: Sprites
- [ ] Billboard quads face the camera correctly in 3D slice space
- [ ] Sprites at the same W as the slice are fully opaque
- [ ] Sprites at increasing W-distance fade smoothly to invisible
- [ ] Sprites are correctly occluded by 4D geometry (shared depth buffer works)
- [ ] 4D geometry is correctly occluded by opaque sprites
- [ ] SpriteAnimation advances frames at the correct FPS
- [ ] Non-looping animations stop at the last frame

### Sub-Phase B: Spatial Queries
- [ ] `query_sphere()` returns all bodies within the specified 4D radius
- [ ] `query_sphere_sorted()` returns results nearest-first
- [ ] Layer filtering correctly excludes non-matching bodies
- [ ] `line_of_sight()` returns true when no geometry blocks the ray (or always true when stubbed)
- [ ] `line_of_sight()` returns false when static geometry blocks the ray (after P1 raycasting)

### Sub-Phase C: FSM
- [ ] `StateMachine::transition()` changes state and resets timer
- [ ] `StateMachine::time_in_state()` accumulates correctly
- [ ] `StateMachine::just_entered()` returns true only on first frame after transition
- [ ] `StateMachine::previous()` returns the state before the last transition

### Sub-Phase D: Area Damage
- [ ] `query_area_effect()` returns correct falloff (1.0 at center, 0.0 at edge)
- [ ] `query_area_effect()` respects layer filters
- [ ] `query_area_effect()` with `require_los: true` excludes bodies behind walls (after P1 raycasting)
- [ ] `apply_impulse()` correctly modifies body velocity based on mass
- [ ] `apply_impulse()` has no effect on static bodies

### Particle System
- [ ] Particles spawn at the correct 3D position
- [ ] Particles age and die after their lifetime expires
- [ ] Gravity, drag, and velocity work correctly
- [ ] Size and color interpolate over particle lifetime
- [ ] Burst mode emits the correct number of particles at once
- [ ] Continuous emission rate spawns particles at the correct rate
- [ ] Additive blending looks correct for fire/sparks
- [ ] Alpha blending looks correct for smoke/dust
- [ ] Particles are hidden behind geometry (depth-read works)
- [ ] Particles do not occlude each other or geometry (no depth-write)
- [ ] Finished emitters are cleaned up

---

## 13. 4D-Specific Challenges and Solutions

### W-Flanking Visibility
Enemies approaching from adjacent W-slices should have visual tells before they become fully visible.

**Engine solution**: The sprite W-distance fade handles this automatically. An enemy at W=player_W + 1.0 renders as a ghosted/transparent sprite. The game can additionally add particle trails, spatial audio cues, or HUD indicators.

### W-Phasing Enemy Rendering
A W-phasing enemy shifts between W-slices during combat. When it phases out, it should not abruptly disappear.

**Engine solution**: The sprite rendering system handles this through continuous W-distance fade. As the enemy's W coordinate moves away from the player's slice, the sprite smoothly fades out. When the enemy phases back in, it smoothly fades in. No special engine support needed beyond the W-fade system.

### Explosion Visual Across W-Slices
A hyperspherical explosion should be visible as a sphere growing and then shrinking as its W-cross-section passes through the player's slice.

**Engine solution**: The game spawns an explosion particle emitter at the 3D-projected position. The "W-shrinking" visual is handled by the particles themselves -- they dissipate naturally. For more accuracy, the game could modulate particle radius based on W-distance from the explosion center. Full 4D geometry slicing of the explosion is deferred (particles are sufficient and look good).

### Enemy Pathfinding in 4D
Full 4D pathfinding (navmesh in 4D) is computationally expensive.

**Solution (game-level)**: Simple "steer toward player" for initial implementation:
1. Calculate direction: `dir = (player_pos - enemy_pos).normalized()`
2. Set velocity: `velocity = dir * move_speed`
3. Physics handles wall collision (enemy slides along walls)

This works for boomer shooters (Doom's AI is essentially this). Waypoint pathfinding is a game-level concern if needed later.

---

## 14. Complete File Inventory

### New Files

| File | Crate | Description |
|------|-------|-------------|
| `sprite.rs` | `rust4d_render` | SpriteSheet, SpriteInstance, SpriteBatch |
| `sprite_pipeline.rs` | `rust4d_render` | SpritePipeline (wgpu render pipeline) |
| `sprite_billboard.wgsl` | `rust4d_render` | Billboard vertex/fragment shader |
| `sprite_animation.rs` | `rust4d_render` | SpriteAnimation state |
| `particles.rs` | `rust4d_render` | ParticleEmitter, ParticleEmitterConfig, Particle |
| `particle_pipeline.rs` | `rust4d_render` | ParticleSystem (wgpu render pipeline) |
| `particle.wgsl` | `rust4d_render` | Particle vertex/fragment shader |
| `spatial_query.rs` | `rust4d_physics` | SpatialQueryResult, AreaEffectHit, query methods |
| `fsm.rs` | `rust4d_game` | StateMachine\<S\> |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_render/src/lib.rs` | Export new sprite and particle modules |
| `rust4d_render/src/pipeline/mod.rs` | Export SpritePipeline, ParticleSystem |
| `rust4d_render/src/pipeline/render_pipeline.rs` | Expose depth texture for shared depth buffer |
| `rust4d_physics/src/world.rs` | Add query_sphere, query_area_effect, line_of_sight, apply_impulse |
| `rust4d_physics/src/lib.rs` | Export new spatial query types |
| `rust4d_game/src/lib.rs` | Export FSM |

---

## 15. Game-Side Reference: Enemy Types

For completeness, here is the game-side enemy architecture that builds on these engine primitives. This is NOT engine work, but shows how the engine APIs are consumed.

### Engine-to-Game Mapping

| Engine Provides | Game Builds |
|----------------|-------------|
| FSM (`StateMachine<S>`) | `EnemyState` enum + transition logic |
| Spatial queries (`query_sphere`) | Enemy awareness ("is player nearby?") |
| LOS check (`line_of_sight`) | Enemy sight ("can I see the player?") |
| Physics body (`RigidBody4D`) | Enemy movement (set velocity toward player) |
| Sprite rendering (`SpriteBatch`) | Enemy visuals (sprite sheets, animations) |
| Particles (`ParticleSystem`) | Enemy death effects, projectile trails |
| Area damage (`query_area_effect`) | Enemy explosion attacks |
| Collision filters (`CollisionFilter::enemy()`) | Enemy-player, enemy-projectile interactions |

### Three Planned Enemy Types

1. **Melee Rusher**: Fast (15.0 speed), low HP (50), standard W behavior, high pain chance (0.5)
2. **Projectile Gunner**: Slow (6.0 speed), medium HP (80), long range (25.0), low pain chance (0.3)
3. **W-Phaser**: Medium speed (10.0), medium HP (60), phases between W-slices (3s cooldown, 1.5s duration), very low pain chance (0.2) -- hard to stagger, countered by hyperspherical explosions
