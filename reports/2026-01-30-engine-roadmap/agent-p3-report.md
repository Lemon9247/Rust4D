# Agent P3: Enemies & AI -- Engine Implementation Plan

**Date**: 2026-01-30
**Agent**: P3 (Enemies & AI)
**Scope**: Phase 3 engine features -- sprite/billboard rendering, AI framework, area damage, particle system

---

## Executive Summary

Phase 3 ("Enemies") requires five features. After careful analysis of the engine/game split and the current rendering pipeline, the breakdown is:

- **Engine must provide**: Billboard/sprite rendering, particle system, hyperspherical area queries, generic FSM framework, LOS via raycasting
- **Game builds on top**: Specific enemy types, AI behavior trees, damage numbers, specific particle effects (blood, sparks), enemy spawn logic

The central technical challenge is **billboard rendering in a 4D slicing context**. The current render pipeline processes 4D tetrahedra through a compute shader, producing 3D triangles. Sprites/billboards do NOT go through this pipeline -- they are inherently 3D objects (camera-facing quads) that exist at specific 4D positions but are rendered directly in the 3D slice space. This requires a **second render pass** alongside the existing tetrahedra-slice pipeline.

**Total engine work**: 4-5 sessions (compared to the original 4.5 session estimate for the combined engine+game work).

---

## 1. Sprite/Billboard Rendering System

### The 4D-Specific Challenge

In a standard 3D engine, billboard sprites face the camera by rotating to always be perpendicular to the view direction. In Rust4D, the situation is more nuanced:

1. **Entities exist in 4D** (Vec4 position with XYZW coordinates)
2. **The camera views a 3D hyperplane slice** of 4D space
3. **A sprite at a different W than the slice** may be partially visible, fully visible, or invisible
4. **Camera-facing** means facing the 3D camera *within the slice space*, not in 4D

The key insight: sprites are NOT 4D geometry (they are not tetrahedra). They are **3D quads rendered at the 3D projection of a 4D position**, with visibility/opacity modulated by W-distance from the slice plane.

### Design: Two-Pass Rendering Architecture

The current pipeline has one flow:

```
4D Tetrahedra -> [Compute: Slice] -> 3D Triangles -> [Render: Forward Pass]
```

Sprites add a second flow:

```
Sprite Entities -> [CPU: Project to Slice Space] -> 3D Billboard Quads -> [Render: Billboard Pass]
```

Both passes write to the same framebuffer and depth buffer, so sprites correctly occlude and are occluded by 4D geometry.

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

### How Billboard Orientation Works in 4D Slice Space

When adding a sprite at 4D position `P = (x, y, z, w)`:

1. **Transform P into camera-local 4D space** using the inverse camera matrix
2. **Compute W-distance**: `w_dist = |P_camera.w - slice_w|`
3. **Project to 3D**: The sprite's 3D position is `(P_camera.x, P_camera.y, P_camera.z)` -- the W component is discarded after computing fade
4. **W-distance fade**: `alpha *= max(0, 1 - w_dist / w_fade_range)`. Sprites far in W fade to invisible
5. **Billboard construction**: In the vertex shader, expand the point into a camera-facing quad using the view matrix's right and up vectors:
   ```wgsl
   let right = vec3(view[0][0], view[1][0], view[2][0]);
   let up    = vec3(view[0][1], view[1][1], view[2][1]);
   let corner = instance.position
       + right * (vertex_offset.x * instance.size.x)
       + up    * (vertex_offset.y * instance.size.y);
   ```

### W-Distance Visibility Rules (Engine Default, Game Can Override)

| W-Distance | Visual Effect | Can Be Hit? |
|------------|---------------|-------------|
| 0.0 - 0.5 | Fully visible | Yes |
| 0.5 - 1.5 | Ghosted (alpha fade) | With splash damage |
| 1.5 - 3.0 | Faint shimmer | Audio cue only |
| > 3.0 | Invisible | No |

These thresholds are configurable per sprite (the engine provides the fade calculation; the game decides the thresholds).

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

**Key architectural decision**: The sprite pipeline shares the depth buffer with the main 4D-slice render pipeline. This means sprites are correctly depth-tested against sliced 4D geometry. Sprites rendered at the same 3D position as a wall will be occluded by the wall.

The sprite pass uses **alpha testing** (not alpha blending) for opaque sprites, and a separate transparent pass with depth-write disabled for ghosted/faded sprites. This two-sub-pass approach prevents sorting issues.

### Sprite Animation

```rust
/// Sprite animation state (engine-provided, game drives the state transitions)
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

The engine provides the animation ticker. The game defines which frames correspond to which enemy states (idle frames 0-3, walk frames 4-11, attack frames 12-15, pain frame 16, death frames 17-21).

### Session Estimate: 1.5 sessions

- 0.5 session: SpriteSheet loading, SpriteInstance type, SpriteBatch
- 0.5 session: SpritePipeline (new wgpu render pipeline, WGSL billboard shader)
- 0.5 session: W-distance fade integration, depth buffer sharing, SpriteAnimation

---

## 2. AI Framework (Engine-Side)

### Engine vs Game Boundary

The engine/game split plan already mentions a generic FSM in `rust4d_game`. The question is: does the engine need AI-specific infrastructure beyond the generic FSM?

**Answer**: Yes, but minimally. The engine provides:
1. **Generic FSM** (already planned in `rust4d_game`) -- good enough for AI states
2. **4D raycasting** (planned by Agent P1) -- used for line-of-sight checks
3. **Distance queries** -- using Vec4 math already available
4. **Spatial queries** -- finding entities within a radius (new, needed for AI awareness)

The engine does NOT need:
- NavMesh or pathfinding (game uses waypoint graphs, which are game-level data)
- Behavior trees (overkill for boomer shooter AI; FSM suffices)
- Steering behaviors (game implements simple chase/flee)

### Generic FSM Design (in `rust4d_game`)

The split plan mentions FSM but does not detail it. Here is the engine-side design:

```rust
/// A generic finite state machine
///
/// Type parameters:
/// - S: State enum (e.g., EnemyState)
/// - C: Context type passed to state logic (e.g., &mut EnemyData)
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

This is intentionally simple. Boomer shooter AI does not need complex state machine features (hierarchical states, parallel states, transition guards). The game implements the actual state logic (what to do in each state, when to transition).

### Spatial Query System (in `rust4d_physics`)

The physics crate needs a way to query "what entities are within radius R of point P?" This is used by:
- AI awareness (detecting the player)
- Area damage (explosion radius queries)
- Proximity triggers

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

The `query_sphere` method iterates all bodies and checks if their collider center is within `radius` of `center` using 4D Euclidean distance (`(center - body.position).length()`). This is O(n) and perfectly adequate for boomer shooter enemy counts (typically 20-50 active enemies). If performance becomes an issue later, a spatial hash grid can be added.

The `line_of_sight` method depends on raycasting from Agent P1. It casts a 4D ray from `from` to `to` and returns false if it hits any body on the `block_layers`.

### What the GAME Implements for AI

The game repo builds enemy AI on top of these engine primitives:

```rust
// In Rust4D-Shooter (game repo)

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EnemyState { Idle, Chase, Attack, Pain, Dead }

struct EnemyAI {
    fsm: StateMachine<EnemyState>,
    body_key: BodyKey,
    target: Option<hecs::Entity>,
    attack_range: f32,
    sight_range: f32,
    pain_chance: f32,    // Probability of entering pain state when hit
    attack_cooldown: f32,
    // Per-enemy-type parameters
    move_speed: f32,
    attack_damage: f32,
    w_behavior: WBehavior,
}

enum WBehavior {
    /// Standard enemy: chases in all 4 dimensions
    Standard,
    /// W-phaser: alternates between player's W-slice and adjacent slices
    WPhaser { phase_cooldown: f32, phase_duration: f32 },
    /// W-flanker: approaches from adjacent W-slices
    WFlanker { preferred_w_offset: f32 },
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

    fn update_idle(&mut self, world: &PhysicsWorld, player_pos: Vec4) {
        let my_pos = world.get_body(self.body_key).unwrap().position;
        let dist = (player_pos - my_pos).length();

        if dist < self.sight_range {
            // Use engine's LOS check (depends on P1 raycasting)
            if world.line_of_sight(my_pos, player_pos, CollisionLayer::STATIC) {
                self.fsm.transition(EnemyState::Chase);
            }
        }
    }

    // ... etc
}
```

### Session Estimate: 0.5 sessions

- The generic FSM is tiny (one small struct, a few methods)
- Spatial queries are straightforward iteration over the body SlotMap
- LOS depends on P1's raycasting; the engine just wraps it

---

## 3. Hyperspherical Area Damage (Engine-Side)

### Design

Area damage queries ("what got hit by this explosion?") are a physics-level spatial query. The engine provides the query; the game applies damage.

This is essentially `query_sphere` from Section 2, plus distance-based falloff calculation:

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

**4D-specific consideration**: In 4D, a hypersphere of radius R encloses much more volume than a 3D sphere of the same radius (V = pi^2/2 * R^4 vs 4/3 * pi * R^3). This means explosions in 4D are proportionally MORE powerful than in 3D because they catch more enemies in adjacent W-slices. This is a deliberate gameplay advantage for explosive weapons -- they are the counter to W-phasing enemies.

The engine computes the raw query. The game applies damage with its own formula:

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
        // game-level damage application
        apply_damage(hit.body_key, damage);
        // game-level knockback
        apply_knockback(hit.body_key, hit.direction, base_damage * hit.falloff * 10.0);
    }
}
```

### Knockback as Impulse

The engine already supports velocity modification on bodies. Knockback is simply:

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

This is a thin convenience method. The game calls it for explosion knockback, weapon impact pushback, etc.

### Session Estimate: 0.5 sessions

- `query_sphere` + `query_area_effect` + `apply_impulse` are straightforward
- LOS check depends on P1 raycasting
- Can be implemented in the same session as the spatial query system

---

## 4. Particle System (Engine-Side)

### Design Philosophy

The particle system is an engine rendering feature. The engine provides a generic emitter/updater/renderer. The game defines specific effects (blood, sparks, explosions, muzzle flash).

### Architecture

Particles live in 3D slice space (like sprites), not in 4D. They are spawned at 3D positions derived from 4D events. This avoids the complexity of simulating particles in 4D and then slicing them.

```
4D Event (explosion at Vec4) -> Project to 3D -> Spawn 3D particles
```

This is the pragmatic choice. Simulating individual particles in 4D and slicing would be computationally expensive and visually indistinguishable from 3D particles at the projected position.

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

Particles are rendered as **point sprites** (single vertex expanded to a quad in the vertex shader) or as **instanced quads** (more flexible, supports rotation). The instanced quad approach is preferred because it integrates with the same billboard technique as sprites.

Particles use **additive blending** for fire/sparks/energy effects, and **alpha blending** for smoke/dust/blood. The blend mode is per-emitter.

Particles are rendered with depth-read but no depth-write. This means:
- Particles behind geometry are correctly hidden
- Particles do not occlude each other (prevents ugly z-fighting)
- Particles do not occlude sprites or geometry (correct visual behavior)

### What the GAME Defines

The game creates `ParticleEmitterConfig` presets for each effect:

```rust
// In game code
fn muzzle_flash_config() -> ParticleEmitterConfig {
    ParticleEmitterConfig {
        max_particles: 20,
        lifetime: (0.05, 0.15),
        speed: (5.0, 15.0),
        spread: 0.3,
        gravity: [0.0, 0.0, 0.0],
        size_start: (0.1, 0.2),
        size_end: (0.0, 0.05),
        color_start: [1.0, 0.9, 0.3, 1.0],  // Bright yellow
        color_end: [1.0, 0.3, 0.0, 0.0],     // Fading orange
        emission_rate: 0.0,  // Burst only
        drag: 0.0,
    }
}

fn blood_config() -> ParticleEmitterConfig {
    ParticleEmitterConfig {
        max_particles: 30,
        lifetime: (0.3, 0.8),
        speed: (2.0, 8.0),
        spread: std::f32::consts::PI,  // Hemisphere
        gravity: [0.0, -15.0, 0.0],
        size_start: (0.05, 0.1),
        size_end: (0.02, 0.05),
        color_start: [0.8, 0.0, 0.0, 1.0],  // Dark red
        color_end: [0.3, 0.0, 0.0, 0.0],     // Fading
        emission_rate: 0.0,  // Burst only
        drag: 0.3,
    }
}

fn explosion_config() -> ParticleEmitterConfig {
    ParticleEmitterConfig {
        max_particles: 100,
        lifetime: (0.2, 1.0),
        speed: (3.0, 20.0),
        spread: 2.0 * std::f32::consts::PI,  // Full sphere
        gravity: [0.0, -5.0, 0.0],
        size_start: (0.2, 0.5),
        size_end: (0.5, 1.0),  // Grows (smoke puffs expand)
        color_start: [1.0, 0.7, 0.1, 1.0],  // Bright orange
        color_end: [0.3, 0.3, 0.3, 0.0],     // Fading gray smoke
        emission_rate: 0.0,
        drag: 0.5,
    }
}
```

### Overlap with Agent P2

Agent P2 (Weapons & Feedback) also needs the particle system for muzzle flash and impact effects. The particle system should be designed by P3 (the more comprehensive consumer) but both phases use it. **The particle system should be implemented before or in parallel with P2's weapon feedback work.**

### Session Estimate: 1.5 sessions

- 0.5 session: ParticleEmitter, Particle struct, CPU simulation (update loop)
- 0.5 session: GPU particle pipeline (wgpu render pipeline, billboard shader, instancing)
- 0.5 session: ParticleSystem manager, blending modes, integration with depth buffer

---

## 5. Enemy Types (Game-Side, Not Engine)

The three enemy types from the synthesis (melee rusher, projectile, W-phaser) are entirely game-level. The engine provides the building blocks:

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

### Game-Level Enemy Architecture

```rust
// All in game repo

struct EnemyDef {
    name: &'static str,
    health: f32,
    move_speed: f32,
    attack_damage: f32,
    attack_range: f32,
    sight_range: f32,
    pain_chance: f32,
    sprite_sheet: AssetHandle<SpriteSheet>,
    w_behavior: WBehavior,
    // Animation frame ranges
    anim_idle: (u32, u32),
    anim_walk: (u32, u32),
    anim_attack: (u32, u32),
    anim_pain: (u32, u32),
    anim_death: (u32, u32),
}

const MELEE_RUSHER: EnemyDef = EnemyDef {
    name: "Rusher",
    health: 50.0,
    move_speed: 15.0,  // Fast!
    attack_damage: 20.0,
    attack_range: 1.5,
    sight_range: 30.0,
    pain_chance: 0.5,
    w_behavior: WBehavior::Standard,
    // ...
};

const PROJECTILE_ENEMY: EnemyDef = EnemyDef {
    name: "Gunner",
    health: 80.0,
    move_speed: 6.0,
    attack_damage: 15.0,
    attack_range: 25.0,
    sight_range: 40.0,
    pain_chance: 0.3,
    w_behavior: WBehavior::Standard,
    // ...
};

const W_PHASER: EnemyDef = EnemyDef {
    name: "Phaser",
    health: 60.0,
    move_speed: 10.0,
    attack_damage: 25.0,
    attack_range: 2.0,
    sight_range: 35.0,
    pain_chance: 0.2,  // Hard to stagger
    w_behavior: WBehavior::WPhaser {
        phase_cooldown: 3.0,
        phase_duration: 1.5,
    },
    // ...
};
```

### Session Estimate: 0 engine sessions (purely game)

Game-side: ~1 session (as originally estimated)

---

## 6. Dependencies on Other Agents

### Dependencies on Agent P1 (Combat Core)

| P3 Needs | P1 Provides | Blocking? |
|----------|-------------|-----------|
| Line-of-sight checks | 4D Raycasting system (`Ray4D`, world raycast) | **Yes** -- LOS is core to AI |
| Area damage LOS checks | Same raycasting | **Yes** -- explosions should not damage through walls |
| Event system for damage | Event bus / collision callbacks | **Partial** -- AI needs to receive "I was hit" events for pain state |

**Mitigation**: The FSM, sprite rendering, and particle system can all be built without raycasting. LOS can initially be stubbed as `true` (all enemies always see the player) and upgraded once P1 delivers raycasting.

### Dependencies on Agent P2 (Weapons & Feedback)

| P3 Needs | P2 Provides | Blocking? |
|----------|-------------|-----------|
| Audio for enemy sounds | Audio system (rodio/kira integration) | **No** -- can be added later |
| HUD for W-proximity alerts | HUD framework | **No** -- can be added later |

**Shared work**: The particle system is needed by both P2 (muzzle flash, impact sparks) and P3 (blood, explosions, death effects). The particle system should be implemented as part of P3 since enemies are the more comprehensive consumer, but P2 should coordinate on the API design.

### Dependencies on Foundation (Agent F)

| P3 Needs | Foundation Provides | Blocking? |
|----------|-------------------|-----------|
| Fixed timestep | Deterministic AI update tick | **Yes** -- AI timers need consistent dt |
| ECS components | Enemy components (Health, AIState, etc.) | **Assumed complete** per hive-mind instructions |

---

## 7. Complete Render Pipeline After P3

After P3's work, the rendering pipeline looks like:

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
    |
    v
[Render Pass 3: Particles]  (NEW: particle_pipeline.rs)
    Draws particle quads with additive or alpha blending
    Reads depth buffer (occlusion), does NOT write depth
    |
    v
[Render Pass 4: HUD]  (from Agent P2, separate)
    2D overlay rendering (health, ammo, crosshair, W-indicator)
    No depth testing
    |
    v
Frame Present
```

---

## 8. 4D-Specific Challenges and Solutions

### Challenge: W-Flanking Visibility

Enemies approaching from adjacent W-slices should have visual tells before they become fully visible.

**Engine solution**: The sprite W-distance fade (Section 1) handles this automatically. An enemy at W=player_W + 1.0 will be rendered as a ghosted/transparent sprite. The game can additionally:
- Add a particle trail at the enemy's 3D-projected position
- Play a spatial audio cue (from P2's audio system)
- Show a W-proximity indicator on the HUD (from P2's HUD framework)

### Challenge: W-Phasing Enemy Rendering

A W-phasing enemy shifts between W-slices during combat. When it phases out, it should not abruptly disappear.

**Engine solution**: The sprite rendering system already handles this through continuous W-distance fade. As the enemy's W coordinate moves away from the player's slice, the sprite smoothly fades out. When the enemy phases back in, it smoothly fades in. No special engine support needed beyond the W-fade system.

### Challenge: Explosion Visual Across W-Slices

A hyperspherical explosion should be visible as a sphere growing and then shrinking as its W-cross-section passes through the player's slice.

**Engine solution**: This is a particle system problem. The game spawns an explosion particle emitter at the 3D-projected position. The "W-shrinking" visual is handled by the particles themselves -- they are spawned at the 3D projection and naturally dissipate. For a more accurate visual, the game could modulate the particle emitter's radius based on the W-distance of the explosion center from the player's slice (larger particles when on-slice, smaller when far).

Alternatively, the explosion could be rendered as actual 4D geometry (a hypersphere that gets sliced), but this is much more complex and not worth it for the initial implementation. Particles are sufficient and look good.

### Challenge: Enemy Pathfinding in 4D

Enemies need to navigate to the player. Full 4D pathfinding (navmesh in 4D space) is computationally expensive.

**Solution (game-level)**: Use simple "steer toward player" for the initial implementation:
1. Calculate direction vector from enemy to player: `dir = (player_pos - enemy_pos).normalized()`
2. Set enemy velocity: `velocity = dir * move_speed`
3. The physics system handles collision with walls (enemy slides along walls)

This works well for boomer shooters (Doom's AI is essentially "move toward player, attack when in range"). Waypoint-based pathfinding can be added later if needed, but it is a game-level concern, not engine.

---

## 9. Summary: Engine Work Items

### New Files in Engine

| File | Crate | Description |
|------|-------|-------------|
| `sprite.rs` | `rust4d_render` | SpriteSheet, SpriteInstance, SpriteBatch |
| `sprite_pipeline.rs` | `rust4d_render` | SpritePipeline (wgpu render pipeline) |
| `sprite_billboard.wgsl` | `rust4d_render` | Billboard vertex/fragment shader |
| `sprite_animation.rs` | `rust4d_render` | SpriteAnimation state |
| `particles.rs` | `rust4d_render` | ParticleEmitter, ParticleEmitterConfig, Particle |
| `particle_pipeline.rs` | `rust4d_render` | ParticleSystem (wgpu render pipeline) |
| `particle.wgsl` | `rust4d_render` | Particle vertex/fragment shader |
| `spatial_query.rs` | `rust4d_physics` | query_sphere, query_area_effect, apply_impulse |
| `fsm.rs` | `rust4d_game` | StateMachine<S> |

### Modified Files in Engine

| File | Change |
|------|--------|
| `rust4d_render/src/lib.rs` | Export new sprite and particle modules |
| `rust4d_render/src/pipeline/mod.rs` | Export SpritePipeline, ParticleSystem |
| `rust4d_render/src/pipeline/render_pipeline.rs` | Expose depth texture for shared depth buffer |
| `rust4d_physics/src/world.rs` | Add query_sphere, query_area_effect, line_of_sight, apply_impulse |
| `rust4d_physics/src/lib.rs` | Export new spatial query types |
| `rust4d_game/src/lib.rs` | Export FSM |

### Session Estimates

| Feature | Engine Sessions | Notes |
|---------|----------------|-------|
| Sprite/Billboard rendering | 1.5 | New render pipeline, WGSL shader, W-distance fade |
| AI framework (FSM + spatial queries) | 0.5 | Generic FSM + query_sphere + apply_impulse |
| Area damage (hyperspherical queries) | 0.5 | query_area_effect + AreaEffectHit (shares session with spatial queries) |
| Particle system | 1.5 | Emitter, renderer, pipeline, blending modes |
| **Total Engine** | **4.0** | |

**Parallelism note**: Sprite rendering and the particle system are independent of each other and can be developed in parallel by two agents. Both depend on understanding the existing render pipeline (shared depth buffer) but do not share code.

### What the Game Builds (Not Engine Work)

| Feature | Game Sessions | Engine Dependency |
|---------|---------------|-------------------|
| Enemy AI state machine logic | 0.5 | FSM (engine), raycasting (P1) |
| 3 enemy types (rusher, gunner, phaser) | 1.0 | Sprites, FSM, physics body, spatial queries |
| Enemy spawn system | 0.5 | ECS (foundation), trigger system (P1) |
| Specific particle effects (blood, explosions) | 0.5 | Particle system (engine) |
| W-phasing enemy behavior | 0.5 | Sprite W-fade (engine), physics (engine) |
| **Total Game** | **3.0** | |

---

## 10. Implementation Order

```
Wave 1 (Foundation dependencies)
  Requires: Fixed timestep (Agent F), ECS migration complete

Wave 2 (Engine Phase 3 - can start after Foundation)
  ├── Agent/Session A: Sprite rendering pipeline (1.5 sessions)
  │   1. SpriteSheet + SpriteInstance types
  │   2. SpritePipeline + billboard WGSL shader
  │   3. W-distance fade, shared depth buffer, SpriteAnimation
  │
  ├── Agent/Session B: Particle system (1.5 sessions)  [PARALLEL with A]
  │   1. ParticleEmitter + Particle simulation
  │   2. GPU particle pipeline + WGSL shader
  │   3. ParticleSystem manager, blending, depth integration
  │
  └── Agent/Session C: Physics queries + FSM (1 session)  [PARALLEL with A & B]
      1. StateMachine<S> in rust4d_game
      2. query_sphere, query_area_effect, apply_impulse in rust4d_physics
      3. line_of_sight (depends on P1 raycasting; stub if not ready)

Wave 3 (Game-side, after engine Wave 2)
  └── Enemy types, AI logic, spawn system, particle effect presets
      (3 sessions, game repo, NOT engine work)
```

All three Wave 2 items can run in parallel. The total critical path for engine work is 1.5 sessions (the longest of the three parallel streams).
