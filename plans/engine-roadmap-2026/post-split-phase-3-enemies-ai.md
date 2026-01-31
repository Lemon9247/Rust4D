# Post-Split Phase 3: Enemies & AI -- Engine Implementation Plan

**Date**: 2026-01-31
**Source**: Agent P3 report from engine roadmap swarm (2026-01-30)
**Status**: Planning document -- ready for implementation
**Updated 2026-01-31**: Integrated Lua scripting amendments. FSM framework removed (Lua handles state machines natively). Lua binding sub-sections added for sprites, spatial queries, and animation. Session estimates updated.

---

## 1. Overview

Phase 3 provides the engine-side infrastructure needed for enemies and AI in any 4D game. After the engine/game split, the boundary is clear: the engine provides **sprite/billboard rendering, spatial queries, area damage math, and a particle system** with **Lua bindings** so game scripts can drive enemy behavior. ~~The generic FSM framework originally planned has been removed~~ -- Lua tables, functions, and coroutines handle state machines natively, making a Rust FSM unnecessary.

**Total engine work**: 3.75-4.5 sessions (original was 4.0; FSM removal saves ~0.25, Lua bindings add ~0.5)
**Critical path**: 1.5 sessions with full parallelism (unchanged from original)

### Prerequisites
- Engine/game split complete (ECS migration done, `rust4d_game` crate exists)
- Foundation phase complete (fixed timestep, serialization fixes)
- Phase 1 Combat Core partially complete (raycasting needed for LOS; can stub until ready)
- `rust4d_scripting` crate exists with mlua integration, Lua 5.4 runtime, hot-reload support, and error handling (see scripting phase plan)

### What This Phase Delivers
- Sprite/billboard rendering pipeline with 4D W-distance fade
- Particle system with multiple blend modes
- Spatial query API (hypersphere queries, area effects)
- Impulse/knockback support on physics bodies
- Line-of-sight wrapper (depends on P1 raycasting)
- **Lua bindings** for sprites, spatial queries, animation, and impulse APIs
- **NOT delivered**: FSM framework (removed -- Lua provides this natively)

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
| Lua sprite API wrappers | `rust4d_scripting` | `sprites:load_sheet()`, `sprites:add()`, `sprites:animate()` |
| Lua spatial query wrappers | `rust4d_scripting` | `world:query_sphere()`, `world:area_effect()`, `world:line_of_sight()`, `world:impulse()` |
| Lua animation control | `rust4d_scripting` | `animation.new()`, `anim:update()`, `anim:frame()`, `entity:set_animation()` |

### Game Builds (in Lua Scripts)

| Feature | Description |
|---------|-------------|
| Enemy state logic | Idle/Chase/Attack/Pain/Dead as Lua table fields and functions (no Rust enum needed) |
| Enemy AI behavior | Per-enemy AI logic using spatial queries and state transitions in Lua |
| Enemy definitions | Enemy type data tables (health, speed, sprite sheets) in Lua files -- hot-reloadable |
| W-phasing behavior | W-phasing, W-flanking behaviors as Lua scripts |
| Spawn system | Where/when enemies appear (Lua-driven) |
| Damage application | `world:area_effect()` + Lua damage logic |
| Particle effect presets | Blood, muzzle flash, explosion configs as Lua data tables |
| Enemy sprite sheets | Art assets and animation frame ranges |

### REMOVED: `StateMachine<S>` in `rust4d_game`

The original plan included a generic `StateMachine<S>` (~30 lines of Rust) in `rust4d_game`. This has been **removed** from engine scope because Lua provides native FSM capability via tables, functions, and coroutines. The entire point of the Rust FSM was to give compiled game code a state management pattern; with Lua, the language itself provides this.

**Session savings**: ~0.25 sessions

**Lua FSM example** (replaces the Rust `StateMachine<S>`):

```lua
local enemy = {
  state = "idle",
  time_in_state = 0,

  update = function(self, dt)
    self.time_in_state = self.time_in_state + dt
    if self.state == "idle" then self:update_idle(dt)
    elseif self.state == "chase" then self:update_chase(dt)
    elseif self.state == "attack" then self:update_attack(dt)
    elseif self.state == "pain" then self:update_pain(dt)
    elseif self.state == "dead" then -- despawn after death animation
    end
  end,

  transition = function(self, new_state)
    self.state = new_state
    self.time_in_state = 0
  end,
}
```

Lua coroutines can also model AI states elegantly (each state is a coroutine that yields when transitioning).

### What Was Game-Side Rust That Now Needs Lua Bindings

| Was (Rust game code) | Becomes (Engine Lua binding) |
|---------------------|------------------------------|
| `sprite_batch.add_sprite_4d(pos, slice_w, cam, size, frame, tint, fade)` | `sprites:add(pos4d, { size={w,h}, frame=N, tint={r,g,b,a} })` |
| `sprite_animation.update(dt)` / `.current_frame()` | `anim:update(dt)` / `anim:frame()` -- or engine auto-updates based on component |
| `SpriteAnimation::new(frames, fps, looping)` | `animation.new({ frames={0,1,2,3}, fps=8, loop=true })` |
| `world.query_sphere(center, radius, layer_filter)` | `world:query_sphere(center, radius, layer?)` returns Lua table |
| `world.query_sphere_sorted(center, radius, layer_filter)` | `world:query_sphere_sorted(center, radius, layer?)` |
| `world.query_area_effect(center, radius, layer, require_los)` | `world:area_effect(center, radius, layer?, los?)` returns hits with falloff |
| `world.line_of_sight(from, to, block_layers)` | `world:line_of_sight(from, to, block_layers?)` returns boolean |
| `world.apply_impulse(key, impulse)` | `world:impulse(entity, impulse_vec4)` |
| `StateMachine<S>` generic FSM | **Removed** -- Lua tables/functions/coroutines handle this natively |

### Boundary Principle
Enemy types, specific AI behaviors, and enemy-specific game logic are 100% game-side Lua scripts. The engine provides sprite rendering, spatial queries, area damage math, and Lua bindings to access them. The engine has zero knowledge of what an "enemy" is.

### What Gets Simpler With Lua

- **FSM becomes unnecessary**: `StateMachine<S>` is removed. Lua tables, functions, and coroutines natively handle state machine patterns -- more flexible, hot-reloadable, and faster to iterate on.
- **AI state logic**: In Rust, AI required defining an `EnemyState` enum, implementing match arms, managing state transitions through the FSM API. In Lua, it is just tables and functions.
- **Enemy definitions**: `EnemyDef` data (health, speed, sprite sheets) was a Rust struct. In Lua, it is a simple data table loaded from a file -- trivially hot-reloadable.

### What Gets Removed From Engine Scope

- **`StateMachine<S>` in `rust4d_game`**: Removed entirely. Saves ~0.25 sessions. Lua provides native FSM capability.
- **`fsm.rs` file**: No longer created.
- **`EnemyState` enum as a Rust type**: Was always game-side, now confirmed as a Lua table field, not a Rust type.
- **`EnemyAI` struct, `EnemyDef` data, `WBehavior` enum**: Were always game-side Rust types; now they are Lua tables. No engine change needed.

---

## 3. Sub-Phase A: Sprite/Billboard Rendering Pipeline

**Session estimate: 1.5 sessions** (unchanged -- all Rust implementation work)

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

These thresholds are configurable per sprite. The engine provides the fade calculation; the game (Lua scripts) decides the thresholds.

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

The engine provides the animation ticker. The game (Lua scripts) defines which frames correspond to which states (e.g., idle frames 0-3, walk frames 4-11, attack frames 12-15, pain frame 16, death frames 17-21).

### Lua Sprite API Bindings (~0.25 session)

Thin wrappers exposing the Rust sprite APIs to Lua scripts:

- `sprites:load_sheet(name, path, cols, rows)` -- load a sprite sheet by name
- `sprites:add(position_4d, config_table)` -- add a sprite to the current frame's batch
  - Config table: `{ size={w,h}, frame=N, tint={r,g,b,a}, w_fade_range=3.0 }`
- `sprites:animate(entity, config_table)` -- attach animation to an entity
- Engine auto-updates sprite animations for entities with animation components
- W-fade parameters configurable per sprite: `{ w_fade_range=3.0 }`

**Lua animation control** (~0.1 session, included in sprite bindings work):

- `animation.new(config)` -- create animation state from Lua table: `{ frames={0,1,2,3}, fps=8, loop=true }`
- `anim:update(dt)` / `anim:frame()` / `anim:reset()` / `anim:finished()`
- Or: engine manages animations as components, Lua just sets which animation is active: `entity:set_animation("walk")`

### Sub-Phase A Session Breakdown

| Task | Sessions | Details |
|------|----------|---------|
| SpriteSheet loading, SpriteInstance type, SpriteBatch | 0.5 | Data types and CPU-side logic |
| SpritePipeline (wgpu render pipeline, WGSL billboard shader) | 0.5 | New GPU pipeline alongside slice pipeline |
| W-distance fade integration, depth buffer sharing, SpriteAnimation | 0.5 | Integration with existing render pipeline |
| Lua sprite API wrappers + animation control | 0.35 | Thin Lua bindings over Rust APIs |

### New Files

| File | Crate | Description |
|------|-------|-------------|
| `sprite.rs` | `rust4d_render` | SpriteSheet, SpriteInstance, SpriteBatch |
| `sprite_pipeline.rs` | `rust4d_render` | SpritePipeline (wgpu render pipeline) |
| `sprite_billboard.wgsl` | `rust4d_render` | Billboard vertex/fragment shader |
| `sprite_animation.rs` | `rust4d_render` | SpriteAnimation state |
| `lua_sprite_api.rs` | `rust4d_scripting` | Lua bindings for sprite and animation APIs |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_render/src/lib.rs` | Export new sprite modules |
| `rust4d_render/src/pipeline/mod.rs` | Export SpritePipeline |
| `rust4d_render/src/pipeline/render_pipeline.rs` | Expose depth texture via `ensure_depth_texture()` for shared depth buffer |
| `rust4d_scripting/src/lib.rs` | Register sprite and animation Lua API tables |

---

## 4. Sub-Phase B: Spatial Queries

**Session estimate: 0.5 sessions** (Rust implementation, combined with Sub-Phase D)

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

### Lua Spatial Query Wrappers (~0.15 session)

Thin wrappers exposing the Rust spatial query APIs to Lua scripts:

- `world:query_sphere(center, radius, layer_mask?)` -- returns array of `{ entity, distance, position }`
- `world:query_sphere_sorted(center, radius, layer_mask?)` -- same, sorted by distance
- `world:area_effect(center, radius, layer_mask?, require_los?)` -- returns array of `{ entity, distance, falloff, position, direction }`
- `world:line_of_sight(from, to, block_layers?)` -- returns boolean
- `world:impulse(entity, impulse_vec4)` -- apply velocity impulse

Results are returned as Lua tables (not userdata) so scripts can read fields freely.

### New Files

| File | Crate | Description |
|------|-------|-------------|
| `spatial_query.rs` | `rust4d_physics` | SpatialQueryResult, query_sphere, query_area_effect, apply_impulse |
| `lua_spatial_api.rs` | `rust4d_scripting` | Lua bindings for spatial query and impulse APIs |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_physics/src/world.rs` | Add query_sphere, query_area_effect, line_of_sight, apply_impulse methods |
| `rust4d_physics/src/lib.rs` | Export new spatial query types |
| `rust4d_scripting/src/lib.rs` | Register spatial query Lua API tables |

---

## 5. Sub-Phase C: FSM Framework -- REMOVED

**Original session estimate**: included in Sub-Phase B's 0.5 sessions (FSM was ~30 lines)
**Amended**: REMOVED from engine scope. Saves ~0.25 sessions.

### Rationale

The `StateMachine<S>` was a generic Rust type intended for `rust4d_game` to provide a state management pattern for game code. With the shift to Lua scripting, this is unnecessary:

- Lua tables and functions handle state machines natively (see example in Section 2)
- Lua coroutines can model AI states elegantly (each state is a coroutine that yields when transitioning)
- The Rust FSM added no capability that Lua does not already provide
- The entire point of the FSM was to give compiled Rust game code a state management pattern; with Lua, the language itself provides this

### What Was Planned (Now Removed)

The following Rust types are **not being implemented**:

- `StateMachine<S>` generic struct in `rust4d_game`
- `fsm.rs` file in `rust4d_game`
- Export in `rust4d_game/src/lib.rs`

### What Replaces It

Lua scripts implement FSMs directly. Example showing how the game-side AI code works entirely in Lua:

```lua
-- Enemy AI defined entirely in Lua -- no Rust FSM needed
local EnemyAI = {}
EnemyAI.__index = EnemyAI

function EnemyAI.new(entity, def)
  return setmetatable({
    entity = entity,
    state = "idle",
    previous_state = "idle",
    time_in_state = 0,
    def = def,  -- enemy definition table (health, speed, etc.)
  }, EnemyAI)
end

function EnemyAI:transition(new_state)
  if new_state ~= self.state then
    self.previous_state = self.state
    self.state = new_state
    self.time_in_state = 0
    return true
  end
  return false
end

function EnemyAI:update(dt)
  self.time_in_state = self.time_in_state + dt
  if self.state == "idle" then self:update_idle(dt)
  elseif self.state == "chase" then self:update_chase(dt)
  elseif self.state == "attack" then self:update_attack(dt)
  elseif self.state == "pain" then self:update_pain(dt)
  elseif self.state == "dead" then -- despawn after death animation
  end
end

function EnemyAI:update_idle(dt)
  -- Use engine spatial queries to detect player
  local nearby = world:query_sphere(self.entity:position(), self.def.awareness_range)
  for _, hit in ipairs(nearby) do
    if hit.entity:has_tag("player") then
      if world:line_of_sight(self.entity:position(), hit.position) then
        self:transition("chase")
      end
    end
  end
end
```

---

## 6. Sub-Phase D: Area Damage / Hyperspherical Explosions

**Session estimate: 0.5 sessions** (combined with Sub-Phase B spatial queries)

### Design

Area damage queries ("what got hit by this explosion?") are a physics-level spatial query. The engine provides the query; the game (Lua scripts) applies damage.

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

### Game-Side Usage (Lua)

```lua
-- In Lua game scripts
function apply_explosion(center, radius, base_damage)
  local hits = world:area_effect(
    center, radius,
    LAYER.ENEMY + LAYER.PLAYER,  -- layer filter
    true  -- require LOS (no damage through walls)
  )

  for _, hit in ipairs(hits) do
    local damage = base_damage * hit.falloff
    hit.entity:damage(damage)
    world:impulse(hit.entity, hit.direction * base_damage * hit.falloff * 10.0)
  end
end
```

---

## 7. Particle System

**Session estimate: 1.5 sessions** (unchanged -- all Rust implementation work)

### Design Philosophy

The particle system is an engine rendering feature. The engine provides a generic emitter/updater/renderer. The game (Lua scripts) defines specific effects (blood, sparks, explosions, muzzle flash).

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

### Game-Side Effect Presets (Lua Examples)

```lua
-- Particle presets defined as Lua data tables (hot-reloadable)
particles:define("muzzle_flash", {
  max_particles = 20,
  lifetime = {0.05, 0.15},
  speed = {5, 15},
  spread = 0.3,
  color_start = {1, 0.9, 0.3, 1},
  color_end = {1, 0.4, 0, 0},
  size_start = {0.1, 0.2},
  size_end = {0.02, 0.05},
})

particles:define("blood", {
  max_particles = 30,
  lifetime = {0.3, 0.8},
  speed = {3, 8},
  spread = math.pi,  -- hemisphere
  gravity = {0, -9.8, 0},
  color_start = {0.5, 0, 0, 1},
  color_end = {0.3, 0, 0, 0},
})

particles:define("explosion", {
  max_particles = 100,
  lifetime = {0.2, 1.0},
  speed = {5, 20},
  spread = math.pi * 2,  -- full sphere
  color_start = {1, 0.6, 0, 1},
  color_end = {0.3, 0.3, 0.3, 0},
})
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
| C: FSM framework | **REMOVED** | Lua handles state machines natively. Saves ~0.25 sessions |
| D: Area damage / hyperspherical explosions | 0.5 | query_area_effect + AreaEffectHit + apply_impulse (combined with B) |
| Particle system | 1.5 | Emitter, renderer, pipeline, blending modes |
| NEW: Lua sprite API wrappers | 0.25 | Thin bindings over Rust sprite/animation APIs |
| NEW: Lua spatial query wrappers | 0.15 | Thin bindings over Rust spatial query APIs |
| NEW: Lua animation control | 0.1 | Animation state creation and control from Lua |
| **Total Engine** | **3.75-4.5** | **Original 4.0 adjusted: -0.25 FSM, +0.5 Lua bindings** |

Net effect is roughly neutral. The FSM removal saves a small amount, and the Lua bindings add a small amount. Sprite and spatial query bindings are thin wrappers over existing Rust APIs.

### Game-Side Work (Lua Scripts, NOT Engine, for reference)

| Feature | Game Sessions | Engine Dependency |
|---------|---------------|-------------------|
| Enemy AI state logic (Lua tables/functions) | 0.5 | Spatial queries (engine), raycasting (P1) |
| 3 enemy types (rusher, gunner, phaser) | 1.0 | Sprites, physics body, spatial queries (all via Lua bindings) |
| Enemy spawn system | 0.5 | ECS (foundation), trigger system (P1) |
| Specific particle effects (blood, explosions) | 0.5 | Particle system (engine, via Lua `particles:define()` and `particles:burst()`) |
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

**Mitigation**: Sprite rendering and particle system can all be built without raycasting. LOS is stubbed as `true` (all enemies always see the player) until P1 delivers raycasting.

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

### On Scripting Phase

| P3 Needs | Scripting Phase Provides | Blocking? |
|----------|------------------------|-----------|
| Lua runtime | `rust4d_scripting` crate with mlua, Lua 5.4 | **Yes** -- all Lua bindings depend on this |
| Hot-reload | Script hot-reload support | **No** -- nice to have, not blocking |
| Error handling | Lua error capture and reporting | **Yes** -- bindings need safe error handling |

**Mitigation**: Lua binding work can be deferred until the scripting phase delivers the `rust4d_scripting` crate. All Rust implementation work (sprites, spatial queries, particles) can proceed independently.

---

## 10. Parallelization

All Wave 2 engine items can run in parallel. The total critical path is **1.5 sessions** (the longest of the three parallel streams). Lua binding work can run as a follow-up wave or be integrated into each stream.

```
Wave 1 (Foundation dependencies -- must complete first)
  Requires: Fixed timestep (Agent F), ECS migration complete

Wave 2 (Engine Phase 3 Rust Implementation -- can start after Foundation)
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
  +-- Agent/Session C: Physics queries (0.5 session)  [PARALLEL with A & B]
      1. query_sphere, query_area_effect, apply_impulse in rust4d_physics
      2. line_of_sight (depends on P1 raycasting; stub if not ready)
      (Note: FSM work REMOVED from this session -- frees up time)

Wave 2.5 (Lua Bindings -- after scripting phase + Wave 2 Rust APIs exist)
  |
  +-- Lua sprite API wrappers (0.25 session)
  +-- Lua spatial query wrappers (0.15 session)  [PARALLEL with sprite wrappers]
  +-- Lua animation control (0.1 session)  [PARALLEL with above]

Wave 3 (Game-side Lua scripts, after engine Wave 2 + 2.5)
  Enemy types, AI logic, spawn system, particle effect presets
  (3 sessions, Lua scripts in game repo, NOT engine work)
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

### Sub-Phase A: Lua Sprite Integration Tests
- [ ] Lua script loads a sprite sheet via `sprites:load_sheet()` and adds sprites to the batch
- [ ] Lua script creates and drives a sprite animation via `animation.new()` and `anim:update()`
- [ ] W-fade correctly applied to Lua-spawned sprites
- [ ] Invalid sprite sheet names produce Lua errors (not crashes)

### Sub-Phase B: Spatial Queries
- [ ] `query_sphere()` returns all bodies within the specified 4D radius
- [ ] `query_sphere_sorted()` returns results nearest-first
- [ ] Layer filtering correctly excludes non-matching bodies
- [ ] `line_of_sight()` returns true when no geometry blocks the ray (or always true when stubbed)
- [ ] `line_of_sight()` returns false when static geometry blocks the ray (after P1 raycasting)

### Sub-Phase B: Lua Spatial Query Integration Tests
- [ ] Lua script queries `world:query_sphere()` and gets correct nearby entities
- [ ] Lua script checks `world:line_of_sight()` and gets correct boolean result
- [ ] Lua script calls `world:area_effect()` and receives correct falloff values
- [ ] Lua script calls `world:impulse()` and body velocity changes correctly

### Sub-Phase C: FSM -- REMOVED
~~- [ ] `StateMachine::transition()` changes state and resets timer~~
~~- [ ] `StateMachine::time_in_state()` accumulates correctly~~
~~- [ ] `StateMachine::just_entered()` returns true only on first frame after transition~~
~~- [ ] `StateMachine::previous()` returns the state before the last transition~~

**Replaced by**: Lua FSM pattern verification (pure Lua, but good to have example/test):
- [ ] Lua FSM pattern works (state transitions, time tracking in Lua tables)
- [ ] Lua coroutine-based state machine works as alternative pattern

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

### Lua Binding General Tests
- [ ] Error in Lua callback does not crash engine (logged, execution continues)
- [ ] All Lua API functions return correct types (tables, booleans, numbers, nil)
- [ ] Lua bindings handle nil/missing optional parameters gracefully

---

## 13. 4D-Specific Challenges and Solutions

### W-Flanking Visibility
Enemies approaching from adjacent W-slices should have visual tells before they become fully visible.

**Engine solution**: The sprite W-distance fade handles this automatically. An enemy at W=player_W + 1.0 renders as a ghosted/transparent sprite. The game (Lua scripts) can additionally add particle trails, spatial audio cues, or HUD indicators.

### W-Phasing Enemy Rendering
A W-phasing enemy shifts between W-slices during combat. When it phases out, it should not abruptly disappear.

**Engine solution**: The sprite rendering system handles this through continuous W-distance fade. As the enemy's W coordinate moves away from the player's slice, the sprite smoothly fades out. When the enemy phases back in, it smoothly fades in. No special engine support needed beyond the W-fade system.

### Explosion Visual Across W-Slices
A hyperspherical explosion should be visible as a sphere growing and then shrinking as its W-cross-section passes through the player's slice.

**Engine solution**: The game spawns an explosion particle emitter at the 3D-projected position. The "W-shrinking" visual is handled by the particles themselves -- they dissipate naturally. For more accuracy, the game could modulate particle radius based on W-distance from the explosion center. Full 4D geometry slicing of the explosion is deferred (particles are sufficient and look good).

### Enemy Pathfinding in 4D
Full 4D pathfinding (navmesh in 4D) is computationally expensive.

**Solution (game-level, in Lua)**: Simple "steer toward player" for initial implementation:
1. Calculate direction: `dir = (player_pos - enemy_pos):normalized()`
2. Set velocity: `entity:set_velocity(dir * move_speed)`
3. Physics handles wall collision (enemy slides along walls)

This works for boomer shooters (Doom's AI is essentially this). Waypoint pathfinding is a game-level concern if needed later, and is well-suited to Lua scripting.

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
| `lua_sprite_api.rs` | `rust4d_scripting` | Lua bindings for sprite and animation APIs |
| `lua_spatial_api.rs` | `rust4d_scripting` | Lua bindings for spatial query and impulse APIs |

### Removed Files (compared to original plan)

| File | Crate | Reason |
|------|-------|--------|
| ~~`fsm.rs`~~ | ~~`rust4d_game`~~ | Removed -- Lua handles state machines natively |

### Modified Files

| File | Change |
|------|--------|
| `rust4d_render/src/lib.rs` | Export new sprite and particle modules |
| `rust4d_render/src/pipeline/mod.rs` | Export SpritePipeline, ParticleSystem |
| `rust4d_render/src/pipeline/render_pipeline.rs` | Expose depth texture for shared depth buffer |
| `rust4d_physics/src/world.rs` | Add query_sphere, query_area_effect, line_of_sight, apply_impulse |
| `rust4d_physics/src/lib.rs` | Export new spatial query types |
| `rust4d_scripting/src/lib.rs` | Register sprite, animation, and spatial query Lua API tables |

---

## 15. Game-Side Reference: Enemy Types

For completeness, here is the game-side enemy architecture that builds on these engine primitives. This is NOT engine work, but shows how the engine APIs are consumed **from Lua scripts**.

### Engine-to-Game Mapping

| Engine Provides | Game Builds (in Lua) |
|----------------|----------------------|
| Spatial queries (`world:query_sphere()`) | Enemy awareness ("is player nearby?") |
| LOS check (`world:line_of_sight()`) | Enemy sight ("can I see the player?") |
| Physics body + impulse (`world:impulse()`) | Enemy movement and knockback |
| Sprite rendering (`sprites:add()`) | Enemy visuals (sprite sheets, animations) |
| Particles (`particles:burst()`) | Enemy death effects, projectile trails |
| Area damage (`world:area_effect()`) | Enemy explosion attacks |
| Collision filters (layer masks) | Enemy-player, enemy-projectile interactions |
| *Lua tables/functions/coroutines* | *FSM state management (replaces `StateMachine<S>`)* |

### Three Planned Enemy Types (Lua Data Tables)

```lua
-- Enemy definitions as hot-reloadable Lua data
local enemy_defs = {
  melee_rusher = {
    speed = 15.0,
    health = 50,
    awareness_range = 20.0,
    attack_range = 2.0,
    pain_chance = 0.5,
    w_behavior = "standard",
    sprite_sheet = "rusher",
    animations = {
      idle = { frames = {0,1,2,3}, fps = 4, loop = true },
      walk = { frames = {4,5,6,7,8,9,10,11}, fps = 10, loop = true },
      attack = { frames = {12,13,14,15}, fps = 12, loop = false },
      pain = { frames = {16}, fps = 1, loop = false },
      death = { frames = {17,18,19,20,21}, fps = 8, loop = false },
    },
  },
  projectile_gunner = {
    speed = 6.0,
    health = 80,
    awareness_range = 25.0,
    attack_range = 25.0,
    pain_chance = 0.3,
    w_behavior = "standard",
    sprite_sheet = "gunner",
    -- animations...
  },
  w_phaser = {
    speed = 10.0,
    health = 60,
    awareness_range = 20.0,
    attack_range = 15.0,
    pain_chance = 0.2,  -- hard to stagger, countered by hyperspherical explosions
    w_behavior = "phase",
    phase_cooldown = 3.0,
    phase_duration = 1.5,
    sprite_sheet = "phaser",
    -- animations...
  },
}
```
