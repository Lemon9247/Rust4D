# Post-Split Phase 4: Level Design Pipeline (Engine)

**Source:** Agent P4 report (`scratchpad/reports/2026-01-30-engine-roadmap/agent-p4-report.md`)
**Created:** 2026-01-31
**Status:** Planning
**Engine Effort:** 4.5 sessions
**Prerequisite:** Engine/game split complete (ECS migration done, `rust4d_game` exists, game repo exists)

---

## 1. Overview

Phase 4 builds the engine infrastructure required for 4D level design. The original cross-swarm synthesis described this phase as "Can build real levels" and included four items: RON preview tool, additional shape types, door/elevator mechanics, and a pickup system. After re-evaluation for the engine/game split, the scope divides cleanly:

**Engine provides:**
- Shape type expansion (new geometric primitives for level geometry)
- RON preview tool with hot-reload (level design feedback loop)
- Tween/interpolation system (smooth property animation)
- Declarative trigger data model and runtime (RON-defined level scripting)

**Game repo implements (NOT in scope for this document):**
- Door/elevator mechanics (FSM, key/door pairing, elevator waypoints)
- Pickup system (health, ammo, weapons, key items)
- Level scripting logic (enemy spawn triggers, secret areas, completion conditions)
- Game-specific event handlers for `GameEvent(String)` trigger actions

The engine delivers generic, reusable systems. The game consumes them to build boomer-shooter-specific behaviors.

---

## 2. Engine vs Game Boundary

### Engine Responsibility

| System | Engine Crate | What It Provides |
|--------|-------------|-----------------|
| Shape types | `rust4d_math` + `rust4d_core` | `Hyperprism4D`, `Hypersphere4D`, `ShapeTemplate` variants |
| RON preview tool | `examples/ron_preview.rs` | Standalone hot-reloading scene viewer |
| Interpolation trait | `rust4d_math` | `Interpolatable` trait with `lerp()` for `f32`, `Vec4`, `Transform4D` |
| Tween system | `rust4d_game` | `Tween<T>`, `EasingFunction`, `TweenManager` |
| Trigger data model | `rust4d_core` | `TriggerDef`, `TriggerZone`, `TriggerAction`, `TriggerRepeat` (serializable) |
| Trigger runtime | `rust4d_game` | `TriggerRuntime` processes triggers each frame |

### Game Responsibility

| System | What the Game Builds |
|--------|---------------------|
| Door mechanics | `Door` component, open/close FSM, key requirement checks, tween triggers |
| Elevator mechanics | `Elevator` component, waypoint cycling via `TweenManager` |
| Pickup system | `Pickup` component, pickup effect application, respawn timers |
| Game event handlers | Interpret `GameEvent(String)` from trigger actions (e.g., "pickup_health_50") |
| Level scripting | Custom Rust systems for the 20% that declarative triggers cannot handle |

### The GameEvent Escape Hatch

The key design principle: `TriggerAction::GameEvent(String)` is the bridge between engine and game. The engine defines a small set of built-in trigger actions (TweenPosition, DespawnSelf, PlaySound). For anything game-specific (heal player, give ammo, spawn enemies), the trigger fires a named event string and the GAME interprets it. This keeps the engine generic while allowing arbitrary trigger responses.

---

## 3. Sub-Phase A: Shape Type Expansion

**Sessions:** 1.0
**Dependencies:** NONE (can start immediately, zero dependencies on any other phase)
**Priority:** P0 -- required for basic level geometry

### Current State

The engine has exactly **two renderable shape types** and **three physics collision primitives**:

**Renderable shapes** (in `rust4d_math`, serialized via `ShapeTemplate` in `rust4d_core`):

| Shape | File | Vertices | Purpose |
|-------|------|----------|---------|
| `Tesseract4D` | `crates/rust4d_math/src/tesseract.rs` | 16 | 4D hypercube -- the only solid object |
| `Hyperplane4D` | `crates/rust4d_math/src/hyperplane.rs` | 16 per cell | Floor/ground -- grid of tesseract-shaped pillars |

**Physics collision shapes** (in `rust4d_physics/src/shapes.rs`, `collision.rs`):

| Shape | Type | Used For |
|-------|------|----------|
| `Sphere4D` | Collision only | Player body, future projectiles |
| `AABB4D` | Collision only | Dynamic bodies, bounded floors |
| `Plane4D` | Collision only | Infinite floor planes |

The `ShapeTemplate` enum (in `crates/rust4d_core/src/shapes.rs`) has only two variants: `Tesseract` and `Hyperplane`. Both use the `ConvexShape4D` trait which requires `vertices()` and `tetrahedra()` for the GPU slicing pipeline.

### Critical Gap: Rectangular Hyperprism

The most impactful single change is making `Tesseract4D` support independent axis dimensions. Currently:

```rust
// Current: all dimensions are equal
pub fn new(size: f32) -> Self {
    let h = size * 0.5;
    // vertices at +/-h on all four axes
}
```

This means every solid object is a perfect hypercube. Building real levels requires walls (thin in one axis), floors (thin in Y), platforms (thin in Y and W), and corridors (long in one axis). All of these are rectangular hyperprisms with independent X/Y/Z/W dimensions.

### New Shape: Hyperprism4D

**Crate:** `rust4d_math`
**File:** `crates/rust4d_math/src/hyperprism.rs` (NEW)

```rust
/// A 4D rectangular hyperprism (generalization of tesseract with independent axis dimensions)
pub struct Hyperprism4D {
    half_extents: Vec4, // half-size along each axis
}

impl Hyperprism4D {
    /// Create a hyperprism with independent dimensions
    pub fn new(x_size: f32, y_size: f32, z_size: f32, w_size: f32) -> Self {
        Self {
            half_extents: Vec4::new(x_size * 0.5, y_size * 0.5, z_size * 0.5, w_size * 0.5),
        }
    }

    /// Create a regular hypercube (all dimensions equal)
    pub fn cube(size: f32) -> Self {
        Self::new(size, size, size, size)
    }
}
```

**Implementation details:**
- Store `half_extents: Vec4` instead of `half_size: f32`
- Vertex generation uses `half_extents.x`, `.y`, `.z`, `.w` per axis
- 16 vertices (same as tesseract: 2^4 combinations of +/- half-extents)
- Tetrahedra decomposition is identical to Tesseract4D (Kuhn triangulation works on any rectangular parallelepiped)
- Implements `ConvexShape4D` trait (provides `vertices()` and `tetrahedra()`)
- `Tesseract4D::new(size)` remains as a convenience that delegates internally to Hyperprism with equal dimensions

**ShapeTemplate variant:**
```rust
// In crates/rust4d_core/src/shapes.rs
pub enum ShapeTemplate {
    Tesseract { size: f32 },
    Hyperplane { /* existing fields */ },
    Hyperprism { x: f32, y: f32, z: f32, w: f32 },  // NEW
    Hypersphere { radius: f32, subdivisions: u32 },   // NEW
}
```

**What Hyperprism4D unlocks:**
- Walls: `Hyperprism(0.5, 4.0, 10.0, 2.0)` -- thin in X, tall, long, spans W
- Floors: `Hyperprism(20.0, 0.5, 20.0, 2.0)` -- thin in Y, wide, spans W
- Platforms: `Hyperprism(5.0, 0.5, 5.0, 0.5)` -- thin in Y and W
- Corridors: `Hyperprism(3.0, 4.0, 20.0, 2.0)` -- narrow, tall, long
- Pillars: `Hyperprism(1.0, 6.0, 1.0, 1.0)` -- tall and thin in XZW

### New Shape: Hypersphere4D

**Crate:** `rust4d_math`
**File:** `crates/rust4d_math/src/hypersphere.rs` (NEW)

The collision primitive `Sphere4D` exists but has no renderable counterpart. A renderable 4D sphere is needed for pillars, decorative objects, and projectile visualization.

**Implementation approach:** Icosphere-like subdivision in 4D.

1. Start from the 16-cell (4D analog of octahedron, 8 vertices)
2. At each subdivision step:
   a. Take a tetrahedron
   b. Find the midpoint of each edge
   c. Project midpoints onto the sphere surface (normalize to radius)
   d. Create new tetrahedra from the subdivided simplex
3. At subdivision level 2, yields ~200-500 tetrahedra (reasonable for GPU slicing)

```rust
pub struct Hypersphere4D {
    radius: f32,
    subdivisions: u32,
    // Cached geometry (vertices + tetrahedra generated at construction time)
    vertices: Vec<Vec4>,
    tetrahedra: Vec<[usize; 4]>,
}

impl Hypersphere4D {
    pub fn new(radius: f32, subdivisions: u32) -> Self {
        let (vertices, tetrahedra) = Self::generate(radius, subdivisions);
        Self { radius, subdivisions, vertices, tetrahedra }
    }
}
```

**GPU slicing behavior:** Slicing a 4D sphere yields a 3D sphere (the cross-section). This is visually intuitive -- at the center W-slice, the player sees a full 3D sphere; at offset W-slices, a smaller sphere; beyond the radius in W, nothing.

**Subdivision level recommendation:** Default to subdivision level 2 (~200 tetrahedra). Allow user to specify in `ShapeTemplate::Hypersphere { radius, subdivisions }`. Too few subdivisions make the sphere blocky after slicing; too many create GPU performance concerns.

### Design Decision: Hyperprism vs Replace Tesseract

**Recommendation: Both coexist (Option B).**
- Keep `Tesseract4D` for API compatibility and clarity
- `Tesseract4D` internally can use `Hyperprism4D::cube()` vertex generation
- `ShapeTemplate::Tesseract { size }` remains for simple equal-sided usage
- `ShapeTemplate::Hyperprism { x, y, z, w }` for asymmetric shapes

### Shape Priority Tiers

**P0 -- Implemented in Phase 4:**
1. `Hyperprism4D` -- walls, floors, platforms, corridors (the most impactful single addition)
2. `Hypersphere4D` -- pillars, decorations, projectile visualization

**P1 -- Expected for interesting levels (deferred to later phases):**
3. `Cylinder4D` -- pillars, columns, tubes, pipe corridors
4. `Cone4D / Frustum4D` -- decorative architecture, funnels
5. Composite shapes -- multiple primitives per entity (data model change)

### Crate Placement Summary

| Shape | Crate | Rationale |
|-------|-------|-----------|
| `Hyperprism4D` | `rust4d_math` | Pure geometry, extends existing pattern |
| `Hypersphere4D` | `rust4d_math` | Pure geometry, new primitive |
| `ShapeTemplate` variants | `rust4d_core` | Serialization bridge |
| Collision shape updates | `rust4d_physics` | Already has collision primitives |

### Files Modified

- `crates/rust4d_math/src/hyperprism.rs` -- NEW: Hyperprism4D struct + ConvexShape4D impl
- `crates/rust4d_math/src/hypersphere.rs` -- NEW: Hypersphere4D struct + subdivision algorithm
- `crates/rust4d_math/src/lib.rs` -- export new types
- `crates/rust4d_core/src/shapes.rs` -- add `ShapeTemplate::Hyperprism` and `ShapeTemplate::Hypersphere` variants, update `create_shape()`

### Verification Criteria

- New shapes produce valid tetrahedra (all indices in range, non-degenerate volumes)
- Round-trip RON serialization preserves all parameters
- GPU slicing produces visible geometry at the correct W-slice positions
- `Hyperprism4D::cube(2.0)` produces identical geometry to `Tesseract4D::new(2.0)`
- Hypersphere4D at subdivision 2 produces 100-500 tetrahedra (reasonable GPU load)

---

## 4. Sub-Phase B: RON Preview Tool

**Sessions:** 2.0 (1.0 core + 1.0 enhanced)
**Dependencies:** Sub-Phase A (shape types), Foundation (serialization)
**Priority:** P0 -- critical level design feedback loop

### Purpose

The RON preview tool is the critical "level design feedback loop" that makes 4D level creation tractable without a full editor. A level designer edits RON in a text editor, saves, and immediately sees the result rendered in 4D with camera controls and W-slice navigation.

This is a standalone application that:
1. Opens a RON scene file from the command line
2. Renders it using the existing `rust4d_render` GPU pipeline
3. Watches the file for changes and hot-reloads automatically
4. Provides camera controls and W-slice navigation for inspecting the scene

### Current Hot-Reload Infrastructure

The engine already has hot-reload support via `AssetCache`:
- `AssetCache::check_hot_reload<T>()` compares file modification times and reloads changed assets (lines 243-294 of `asset_cache.rs`)
- `AssetCache::set_watch_for_changes(true)` enables polling
- The `Asset` trait requires `load_from_file(path) -> Result<Self, AssetError>`
- `Scene` already implements `Scene::load()` from RON files
- 35 tests cover the asset cache system

**Gaps to address:**
- `Scene` does NOT implement the `Asset` trait (it has its own `Scene::load()` method)
- No standalone viewer binary exists -- the only binary is the game's `src/main.rs`
- Hot-reload polls modification times but does not use filesystem watchers (no `notify` crate) -- fine for a preview tool (poll every 500ms)
- No mechanism to re-instantiate an `ActiveScene` from a reloaded `Scene` template

### Architecture Decision

**Start as `examples/ron_preview.rs`, promote to `rust4d_tools` crate if/when egui overlay is added.**

Rationale:
- Tools need their own dependencies (winit for window, possibly egui for UI overlays)
- For the first iteration, an example binary avoids creating a whole new crate
- Promote to `rust4d_tools` when it needs dependencies beyond what examples already have
- The preview tool is essentially a specialized version of the tech demo minus game logic plus hot-reload

### Session 4a: Core Viewer (1.0 session)

**Features:**

1. **Open a RON file from command line argument**
   - `cargo run --example ron_preview -- scenes/test_chamber.ron`
   - Parse `Scene`, create `ActiveScene` with physics

2. **Render using existing pipeline**
   - Reuse `rust4d_render` GPU slicing pipeline
   - Reuse camera controller from `rust4d_input`
   - Standard winit window with wgpu surface

3. **Hot-reload loop**
   - Every 500ms (configurable), check file modification time
   - If changed: reload `Scene` from RON, re-instantiate `ActiveScene`
   - Preserve camera position/orientation across reloads
   - Log reload events to console

4. **W-slice navigation**
   - Scroll wheel adjusts W-slice offset (already implemented in the tech demo)
   - Display current W-slice value in window title

**Hot-reload cycle detail:**

```
1. Poll file modification time (every ~500ms)
2. If file changed:
   a. Save current camera state (position, rotation, W-slice)
   b. Parse RON file -> Scene
   c. Validate scene (use SceneValidator)
   d. Instantiate ActiveScene from Scene template
   e. Restore camera state
   f. Rebuild GPU geometry buffers (RenderableGeometry)
   g. Log: "Reloaded scene: N entities, gravity=G"
3. If parse error:
   a. Log error to console (with line number from RON parser)
   b. Keep displaying the last valid scene
   c. Continue polling for fixes
```

**Error resilience is critical.** The tool must never crash on a malformed RON file. Log the error and keep showing the last good state. The RON parser already provides `SpannedError` with line/column information.

### Session 4b: Enhanced Viewer (1.0 session, optional)

5. **Entity highlight/selection**
   - Click to select entity
   - Display entity name, tags, transform, shape info
   - Highlight selected entity with wireframe or color tint

6. **Overlay information panel (egui)**
   - Entity list (tree view)
   - Selected entity properties
   - W-position indicator / W-slice slider
   - Scene metadata (name, gravity, entity count)

7. **Multiple W-slice views**
   - Split viewport to show 2-3 different W-slices simultaneously
   - Helps understand how objects span the W dimension

8. **Physics visualization toggle**
   - Wireframe collision shapes
   - Scene statistics (entity count, tetrahedra count, FPS)

9. **Screenshot/export**
   - Save current view as PNG
   - Useful for documentation and level design iteration

### Dependency Chain

```
ron_preview depends on:
  rust4d_core   (Scene, ActiveScene, World, entities)
  rust4d_math   (Vec4, shapes)
  rust4d_physics (PhysicsConfig, bodies)
  rust4d_render  (GPU pipeline, RenderableGeometry)
  rust4d_input   (CameraController)
  winit          (window)
  wgpu           (GPU)
```

This is essentially the same dependency set as the current tech demo binary.

### Files Created

- `examples/ron_preview.rs` -- main preview tool binary (NEW)

### Build-Time Dependencies

- `rust4d_core`, `rust4d_math`, `rust4d_physics`, `rust4d_render`, `rust4d_input`
- `winit`, `wgpu` (already workspace dependencies)
- `env_logger` (already a dependency)
- `clap` or `std::env::args` for CLI argument parsing

---

## 5. Sub-Phase C: Tween/Interpolation System

**Sessions:** 0.5
**Dependencies:** ECS migration complete, fixed timestep
**Priority:** P1 -- required for doors, elevators, and all moving level geometry
**Crates:** `rust4d_math` (trait) + `rust4d_game` (system)

### Why the Engine Needs This

Doors, elevators, platforms, and any moving level geometry require interpolating properties over time. This is not game-specific -- it is a fundamental game framework capability that any 4D game would use. The engine currently has NO animation or interpolation system. Physics bodies can have velocity, but there is no way to smoothly move an entity from point A to point B over a duration.

### Interpolatable Trait (in rust4d_math)

```rust
// crates/rust4d_math/src/interpolation.rs -- NEW

/// Trait for types that can be linearly interpolated
pub trait Interpolatable: Clone {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}

// Implementations for engine types
impl Interpolatable for f32 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        a + (b - a) * t
    }
}

impl Interpolatable for Vec4 {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Vec4::new(
            a.x + (b.x - a.x) * t,
            a.y + (b.y - a.y) * t,
            a.z + (b.z - a.z) * t,
            a.w + (b.w - a.w) * t,
        )
    }
}

impl Interpolatable for Transform4D {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        // lerp position, slerp rotation via Rotor4
        Transform4D {
            position: Vec4::lerp(&a.position, &b.position, t),
            rotation: Rotor4::slerp(&a.rotation, &b.rotation, t),
            // scale if applicable
        }
    }
}
```

### Easing Functions (in rust4d_game)

```rust
// crates/rust4d_game/src/tween.rs -- NEW

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EasingFunction {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    // extensible as needed
}

impl EasingFunction {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => t * (2.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 { 2.0 * t * t }
                else { -1.0 + (4.0 - 2.0 * t) * t }
            },
            // ... etc
        }
    }
}
```

### Tween Struct (in rust4d_game)

```rust
/// A tween that interpolates a value over time
pub struct Tween<T: Interpolatable> {
    from: T,
    to: T,
    duration: f32,
    elapsed: f32,
    easing: EasingFunction,
    state: TweenState,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TweenState {
    Running,
    Completed,
    Paused,
}

impl<T: Interpolatable> Tween<T> {
    pub fn new(from: T, to: T, duration: f32, easing: EasingFunction) -> Self { ... }
    pub fn update(&mut self, dt: f32) -> T { ... }  // returns current interpolated value
    pub fn is_complete(&self) -> bool { ... }
    pub fn pause(&mut self) { ... }
    pub fn resume(&mut self) { ... }
}
```

### TweenManager (in rust4d_game)

```rust
/// Manages active tweens for entities
pub struct TweenManager {
    position_tweens: HashMap<hecs::Entity, Tween<Vec4>>,
    // Future: rotation_tweens, scale_tweens, material_tweens
}

impl TweenManager {
    /// Start a position tween for an entity
    pub fn tween_position(
        &mut self,
        entity: hecs::Entity,
        target: Vec4,
        duration: f32,
        easing: EasingFunction,
    );

    /// Update all active tweens, apply to world, returns list of completed entities
    pub fn update(&mut self, dt: f32, world: &mut hecs::World) -> Vec<hecs::Entity>;
}
```

### Engine vs Game Boundary

| Component | Location | Rationale |
|-----------|----------|-----------|
| `Interpolatable` trait | `rust4d_math` | Pure math utility, no game dependencies |
| `EasingFunction` enum | `rust4d_game` | Common game pattern, serde for RON triggers |
| `Tween<T>` struct | `rust4d_game` | Common game pattern |
| `TweenManager` | `rust4d_game` | Manages entity tweens via ECS |
| Door logic (open/close FSM) | Game repo | Game-specific behavior |
| Elevator logic (waypoints, timing) | Game repo | Game-specific behavior |
| Key/door pairing | Game repo | Game-specific data |

### Files Modified

- `crates/rust4d_math/src/interpolation.rs` -- NEW: `Interpolatable` trait, `lerp`/`slerp` implementations
- `crates/rust4d_math/src/lib.rs` -- export interpolation module
- `crates/rust4d_game/src/tween.rs` -- NEW: `Tween<T>`, `TweenManager`, `EasingFunction`
- `crates/rust4d_game/src/lib.rs` -- export tween module

### Verification Criteria

- Easing functions produce correct curves (t=0 -> 0.0, t=1 -> 1.0 for all types)
- `Tween<Vec4>` correctly interpolates between two 4D points
- `TweenManager::update()` applies position changes to ECS entities
- Tween completes after `duration` seconds and returns `TweenState::Completed`
- Paused tweens do not advance

---

## 6. Sub-Phase D: Declarative Trigger System

**Sessions:** 1.0 (0.5 data model + 0.5 runtime)
**Dependencies:** Foundation serialization (Wave 2), P1 event system + trigger callbacks (Wave 5)
**Priority:** P1 -- covers 80% of level scripting needs

### Context

The collision layer system already has a `TRIGGER` layer and `CollisionFilter::trigger()`, but there are NO callbacks -- the engine can detect trigger overlaps but cannot notify anyone about them. Agent P1 identified that the current trigger system is non-functional due to a bug in symmetric `collides_with()` checks.

Phase 4 provides the *declarative* layer on top of P1's *imperative* trigger detection. P1 fixes the collision bug and provides `drain_collision_events()` with `TriggerEnter`/`TriggerExit` events. P4 defines the RON format for declaring triggers and the runtime that executes trigger actions.

### Wave 2: Declarative Trigger Data Model (0.5 session)

**Prerequisite:** Foundation serialization must be done (Rotor4 serde).

#### TriggerDef RON Format

```ron
Scene(
    name: "Level 1",
    entities: [ ... ],
    triggers: [
        TriggerDef(
            name: "door_trigger_1",
            zone: AABB(
                center: (5.0, 1.0, 0.0, 0.0),
                half_extents: (2.0, 2.0, 2.0, 1.0),
            ),
            detects: [Player],
            actions: [
                TweenPosition(
                    target_entity: "secret_door",
                    to: (5.0, 4.0, 0.0, 0.0),
                    duration: 1.5,
                    easing: EaseInOutQuad,
                ),
            ],
            repeat: Once,
        ),
        TriggerDef(
            name: "pickup_health",
            zone: Sphere(center: (10.0, 1.0, 3.0, 0.0), radius: 1.0),
            detects: [Player],
            actions: [
                GameEvent("pickup_health_large"),
                DespawnSelf,
            ],
            repeat: Once,
        ),
    ],
    gravity: Some(-20.0),
    player_spawn: Some((0.0, 2.0, 5.0, 0.0)),
)
```

#### Trigger System Types

```rust
// In crates/rust4d_core/src/trigger.rs -- NEW

/// Serializable trigger definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerDef {
    pub name: String,
    pub zone: TriggerZone,
    pub detects: Vec<CollisionLayer>,
    pub actions: Vec<TriggerAction>,
    pub repeat: TriggerRepeat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriggerZone {
    AABB { center: [f32; 4], half_extents: [f32; 4] },
    Sphere { center: [f32; 4], radius: f32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriggerAction {
    /// Tween an entity's position (engine-level)
    TweenPosition {
        target_entity: String,  // entity name reference
        to: [f32; 4],
        duration: f32,
        easing: EasingFunction,
    },
    /// Fire a named game event (game interprets it)
    GameEvent(String),
    /// Despawn the trigger entity itself
    DespawnSelf,
    /// Play a sound (engine-level, requires audio system from P2)
    PlaySound { path: String, volume: f32 },
    /// Enable/disable another trigger by name
    SetTriggerEnabled { trigger: String, enabled: bool },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TriggerRepeat {
    Once,
    Cooldown(f32),  // seconds between activations
    Always,
}
```

#### Scene Integration

Add `triggers: Vec<TriggerDef>` field to the `Scene` struct (optional, defaults to empty vec).

#### Engine vs Game Boundary for Trigger Types

| Component | Location | Rationale |
|-----------|----------|-----------|
| `TriggerDef` struct | `rust4d_core` (scene data) | Serializable scene data |
| `TriggerZone` enum | `rust4d_core` | Maps to physics collision shapes |
| `TriggerAction` enum | `rust4d_core` | Serializable action definitions |
| Trigger runtime | `rust4d_game` | Processes triggers each frame |
| Game-specific event handlers | Game repo | Interprets `GameEvent` strings |

#### Files Modified (Data Model)

- `crates/rust4d_core/src/trigger.rs` -- NEW: trigger data types
- `crates/rust4d_core/src/scene.rs` -- add `triggers` field to Scene
- `crates/rust4d_core/src/scene_validator.rs` -- validate trigger references (target entities exist)
- `crates/rust4d_core/src/lib.rs` -- export trigger module

#### Verification (Data Model)

- RON round-trip serialization for all trigger types
- SceneValidator catches invalid entity name references in TriggerAction::TweenPosition
- Scene with empty triggers field loads correctly (backward compatible)

### Wave 5: Trigger Runtime (0.5 session)

**Prerequisites:** P1 event system and trigger callbacks, Wave 2 (trigger data model), Wave 3 (tween system).

#### TriggerRuntime

```rust
// In crates/rust4d_game/src/trigger_runtime.rs -- NEW

pub struct TriggerRuntime {
    triggers: Vec<ActiveTrigger>,
}

struct ActiveTrigger {
    def: TriggerDef,
    entity: hecs::Entity,       // the trigger zone entity
    enabled: bool,
    fired: bool,                // for Once triggers
    cooldown_remaining: f32,    // for Cooldown triggers
}

impl TriggerRuntime {
    /// Initialize from scene trigger definitions
    pub fn from_scene(triggers: &[TriggerDef], world: &hecs::World) -> Self { ... }

    /// Process triggers for this frame
    pub fn update(
        &mut self,
        dt: f32,
        collision_events: &[CollisionEvent],
        world: &mut hecs::World,
        tween_manager: &mut TweenManager,
        event_bus: &mut EventBus,
    ) { ... }
}
```

**Each frame, the runtime:**
1. Checks trigger zones against entity positions (via collision events from P1)
2. When entity enters zone: executes the `TriggerAction` list
3. `TweenPosition` action: creates a tween via `TweenManager`
4. `GameEvent` action: fires the named event on the `EventBus`
5. `DespawnSelf` action: marks entity for removal
6. `PlaySound` action: plays sound via audio system (deferred if P2 audio not ready)
7. `SetTriggerEnabled` action: enables/disables another trigger by name
8. Respects `TriggerRepeat` (Once fires once, Cooldown respects timer, Always fires every overlap)

#### Error Handling for Entity References

When a trigger action references an entity by name (e.g., `TweenPosition { target_entity: "secret_door" }`):
- **At scene load time:** `SceneValidator` validates all entity name references exist
- **At runtime:** Log warning and skip the action if the entity has been despawned

#### Files Modified (Runtime)

- `crates/rust4d_game/src/trigger_runtime.rs` -- NEW: trigger execution logic
- `crates/rust4d_game/src/lib.rs` -- export trigger_runtime module

#### Verification (Runtime)

- Integration test: load scene with triggers, simulate player entering zone, verify actions fire
- Once triggers only fire once
- Cooldown triggers respect timer
- `TweenPosition` correctly creates position tweens for target entities
- `GameEvent` correctly fires named events on the event bus
- `DespawnSelf` correctly removes the trigger entity

---

## 7. 4D-Specific Level Design Considerations

### W-Layered Rooms

A core 4D level design pattern: rooms that exist at different W-coordinates, connected by transitions. The engine supports this through the combination of triggers and tweens:

1. **W-portals** -- Regions where the player transitions from one W-layer to another
   - Implementation: A trigger zone that tweens the player's W-coordinate
   - Just a `TriggerAction::TweenPosition` targeting the player, moving only the W component
   - Or a special `TriggerAction::ShiftW { delta: f32 }` for clarity (future)

2. **W-visibility ranges** -- Objects only rendered when near the current W-slice
   - Already partially handled by the GPU slicing pipeline (objects far from the W-slice produce no cross-section triangles)
   - `Hyperplane4D` already has a `w_extent` parameter controlling W visibility range
   - May need a LOD-like system later to skip GPU processing for objects far from current W-slice

3. **W-layered collision** -- Physics only collides objects in the same W-layer
   - Already handled by the 4D collision system (AABB collision checks all 4 dimensions)
   - An object at W=10 naturally does not collide with one at W=0

### RON Level Design Patterns

```ron
// A room at W=0
EntityTemplate(
    name: "room_w0_floor",
    tags: ["static"],
    transform: Transform4D(position: Vec4(x: 0, y: 0, z: 0, w: 0), ...),
    shape: Hyperprism(x: 20.0, y: 0.5, z: 20.0, w: 2.0),  // thin in Y, spans W
    material: Material(base_color: (0.3, 0.3, 0.3, 1.0)),
),

// A room at W=5 (different W-layer)
EntityTemplate(
    name: "room_w5_floor",
    tags: ["static"],
    transform: Transform4D(position: Vec4(x: 0, y: 0, z: 0, w: 5.0), ...),
    shape: Hyperprism(x: 15.0, y: 0.5, z: 15.0, w: 2.0),
    material: Material(base_color: (0.4, 0.2, 0.2, 1.0)),
),

// A W-portal connecting the two rooms
TriggerDef(
    name: "w_portal_0_to_5",
    zone: AABB(center: (10.0, 1.0, 0.0, 0.5), half_extents: (1.0, 2.0, 1.0, 0.5)),
    detects: [Player],
    actions: [
        TweenPosition(
            target_entity: "player",
            to_w: 5.0,
            duration: 0.5,
            easing: EaseInOutQuad,
        ),
    ],
    repeat: Always,
),
```

### Entity Prefabs (Future, NOT in Phase 4 scope)

For larger levels, repeating entity groups becomes painful. A prefab system would help but is deferred. The trigger and entity naming systems should support namespacing (e.g., `prefab_instance_1.door_panel`) to keep this future-compatible.

---

## 8. Session Estimates Summary

| Wave | Sub-Phase | Task | Sessions | Dependencies |
|------|-----------|------|----------|-------------|
| Wave 1 | A | Shape type expansion (Hyperprism4D + Hypersphere4D) | 1.0 | **None** (can start immediately) |
| Wave 2 | D (data) | Declarative trigger data model | 0.5 | Foundation serialization |
| Wave 3 | C | Tween/interpolation system | 0.5 | ECS, fixed timestep |
| Wave 4a | B (core) | RON preview tool -- core viewer | 1.0 | Wave 1, Foundation |
| Wave 4b | B (enhanced) | RON preview tool -- enhanced features | 1.0 | Wave 4a |
| Wave 5 | D (runtime) | Trigger runtime | 0.5 | P1 events, Wave 2, Wave 3 |
| **Total** | | | **4.5** | |

---

## 9. Dependencies

### Dependencies on Foundation (from Agent F)

| Foundation Item | P4 Dependency | Why |
|----------------|---------------|-----|
| Serialization (Rotor4 Serialize) | **Required** | Scene templates contain Transform4D with Rotor4 rotations |
| Fixed timestep | **Required** | Tween system needs consistent dt for smooth animation |
| ECS migration complete | **Required** | Trigger system references entities by `hecs::Entity` |

### Dependencies on P1 (Combat Core)

| P1 Item | P4 Dependency | Why |
|---------|---------------|-----|
| Event system | **Required** | Trigger actions fire events; game receives them via EventBus |
| Trigger zone callbacks | **Required** | Triggers need to know when entities enter/exit zones |
| Collision bug fix | **Required** | Current symmetric `collides_with()` prevents triggers from detecting players |

### Dependencies on P2 (Weapons & Feedback)

| P2 Item | P4 Dependency | Why |
|---------|---------------|-----|
| Audio system (`rust4d_audio`) | **Optional** | `TriggerAction::PlaySound` needs audio playback; defer if not ready |

### Dependencies on P3 (Enemies & AI)

None. Phase 4 is independent of Phase 3.

### What P5 (Editor) Depends on from P4

| P4 Item | P5 Usage |
|---------|----------|
| All shape types (Hyperprism4D, Hypersphere4D) | Editor property panel shows shape parameters |
| ShapeTemplate variants | Editor creates entities with any shape type |
| TriggerDef format | Editor visualizes and edits trigger zones as wireframes |
| RON preview tool | Editor may share camera/render code with preview tool viewport |

---

## 10. Parallelization Strategy

```
Wave 1 (Shape types) -----> Wave 4a (Preview tool core) --> Wave 4b (Preview enhanced)
                       \
Foundation (serde) -----> Wave 2 (Trigger data model) --\
                                                          --> Wave 5 (Trigger runtime)
ECS + Fixed timestep ---> Wave 3 (Tween system) --------/
                                                        /
P1 (Event system) ------------------------------------/
```

**Key observations:**
- **Waves 1, 2, and 3 are independent of each other** and can be done in parallel by different agents
- **Wave 1 has zero dependencies** -- can start immediately, even before Foundation
- **Wave 2** requires Foundation serialization (Rotor4 serde) for RON round-trips
- **Wave 3** requires ECS and fixed timestep for entity tweens and consistent animation
- **Wave 4** is the largest single task (2 sessions) and requires Wave 1 + Foundation
- **Wave 5** is the integration point (trigger runtime) and comes last, needing P1 events + Waves 2 + 3
- **Critical path:** Foundation -> Wave 2 -> (wait for P1 events + Wave 3) -> Wave 5

### What Can Start Before This Phase

Wave 1 (shape type expansion) has zero dependencies on any other phase and can begin as soon as any agent is available. It does not require the ECS migration, the engine/game split, or Foundation serialization. This makes it an excellent parallel task to assign during earlier phases.

---

## 11. Verification Criteria (Phase-Wide)

### Sub-Phase A (Shapes)
- [ ] `Hyperprism4D::new(x, y, z, w)` produces valid geometry with 16 vertices
- [ ] `Hyperprism4D::cube(size)` matches `Tesseract4D::new(size)` output
- [ ] `Hypersphere4D::new(radius, 2)` produces 100-500 tetrahedra
- [ ] `ShapeTemplate::Hyperprism` round-trips through RON
- [ ] `ShapeTemplate::Hypersphere` round-trips through RON
- [ ] GPU slicing renders new shapes correctly at various W-slices

### Sub-Phase B (Preview Tool)
- [ ] `cargo run --example ron_preview -- path/to/scene.ron` opens and renders a scene
- [ ] Modifying the RON file triggers automatic reload within ~1 second
- [ ] Malformed RON shows error in console, keeps displaying last valid scene
- [ ] Camera position/orientation preserved across reloads
- [ ] W-slice navigation via scroll wheel works
- [ ] Window title shows current W-slice value

### Sub-Phase C (Tweens)
- [ ] `Interpolatable` implemented for `f32`, `Vec4`, `Transform4D`
- [ ] All easing functions produce 0.0 at t=0 and 1.0 at t=1
- [ ] `TweenManager::update()` correctly applies position changes to ECS entities
- [ ] Completed tweens are cleaned up from the manager

### Sub-Phase D (Triggers)
- [ ] Scene with triggers loads and validates correctly from RON
- [ ] Empty triggers field backward-compatible with existing scenes
- [ ] `TweenPosition` action creates tweens for named entities
- [ ] `GameEvent` action fires events on the event bus
- [ ] `DespawnSelf` removes the trigger entity
- [ ] `Once` triggers fire exactly once
- [ ] `Cooldown(n)` triggers respect the cooldown timer
- [ ] Invalid entity references logged as warnings, not panics

---

## 12. Cross-Phase Coordination Notes

### For Agent F (Foundation)
- P4 needs Rotor4 Serialize/Deserialize for scene hot-reload (Transform4D round-trips through RON)
- P4 needs fixed timestep for the tween system's smooth animation

### For Agent P1 (Combat Core)
- P4 depends on trigger zone callbacks. P4's declarative trigger system is the *declarative* layer on top of P1's *imperative* trigger detection via `drain_collision_events()`.
- P4 needs the event system to support string-named events for `GameEvent(String)` trigger actions. If P1's `CollisionEvent` is purely typed, the `rust4d_game` EventBus needs an `AnyEvent` or `NamedEvent` variant.
- P1 must fix the trigger detection bug (symmetric `collides_with()` prevents triggers from detecting players).

### For Agent P2 (Weapons & Feedback)
- `TriggerAction::PlaySound` depends on P2's audio system (`rust4d_audio` with kira). This action type can be deferred/stubbed if audio is not ready yet.

### For Agent P5 (Editor & Polish)
- The RON preview tool (Wave 4) shares rendering infrastructure with the editor. The preview tool could become the foundation for the editor's viewport.
- Shape type expansion (Wave 1) directly feeds the editor's entity creation UI -- editor needs all ShapeTemplate variants.
- The trigger data model (Wave 2) needs editor visualization (trigger zones drawn as wireframes in the editor viewport).

---

## 13. Open Questions

1. **Should `Hyperprism4D` replace `Tesseract4D` or coexist?**
   - Recommendation: **Option B -- both coexist.** Keep `Tesseract4D` for API compatibility and clarity. Internally it can use `Hyperprism4D::cube()` vertex generation.

2. **How many subdivision levels for Hypersphere4D?**
   - Recommendation: Default to subdivision level 2 (~200 tetrahedra). Allow user to specify via `ShapeTemplate::Hypersphere { radius, subdivisions }`.

3. **Should the preview tool be headless-capable?**
   - Recommendation: Defer headless validation to P5 (editor). Could be a separate `ron_validate` example.

4. **Trigger actions referencing entities by name: what if entity doesn't exist?**
   - Recommendation: Validate at scene load time via `SceneValidator`. At runtime, log warning and skip the action. Never panic.

5. **For P1: Does the event system support string-named events?**
   - The declarative trigger system assumes `GameEvent(String)` which the game interprets. If P1's event system is typed-only, an `AnyEvent` or `NamedEvent` variant is needed in the `rust4d_game` EventBus.

---

## 14. What the Game Repo Builds on Top

This section is informational -- none of this is engine work, but it shows how the engine systems are consumed.

### Door/Elevator Mechanics (1-2 sessions in game repo)

```rust
// In Rust4D-Shooter (game repo)
struct Door {
    closed_position: Vec4,
    open_position: Vec4,
    open_duration: f32,
    state: DoorState,
    required_key: Option<KeyColor>,
}

enum DoorState { Closed, Opening, Open(f32), Closing }

fn door_system(world: &mut World, tweens: &mut TweenManager, events: &EventBus) {
    for event in events.read::<TriggerEnterEvent>() {
        if let Some(door) = world.get::<Door>(event.trigger_entity) {
            tweens.tween_position(
                event.trigger_entity,
                door.open_position,
                door.open_duration,
                EasingFunction::EaseInOutQuad,
            );
        }
    }
}
```

### Pickup System (0.5 session in game repo)

- `Pickup` component with type (health, ammo, weapon, key) and amount
- Trigger zones with `GameEvent("pickup_health_50")` actions
- Game event handler applies effect and despawns entity
- Optional respawn timer

### Level Scripting (ongoing in game repo)

Declarative triggers handle 80% of needs:
- Secret doors revealed by wall interaction
- Trap triggers (ceiling crushing, floor opening)
- Enemy spawn triggers (player enters area -> spawn wave)
- W-portal triggers (shift player to different W-layer)
- Completion triggers (all enemies dead -> open exit)

The remaining 20%: custom Rust code in game-side systems.

### Game Repo Effort (NOT counted in engine total)

| Task | Sessions | Dependencies |
|------|----------|-------------|
| Door/elevator mechanics | 1-2 | Engine Waves 3+5 |
| Pickup system | 0.5 | Engine Wave 5 (P1 events) |
| Level scripting | ongoing | Declarative triggers |
