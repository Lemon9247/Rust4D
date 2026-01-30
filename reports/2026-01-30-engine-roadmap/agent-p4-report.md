# Agent P4: Level Design Pipeline -- Engine Implementation Plan

**Agent**: P4 -- Level Design Pipeline
**Date**: 2026-01-30
**Scope**: Phase 4 items from cross-swarm synthesis, re-evaluated for engine/game split
**Assumption**: ECS migration and engine/game split (9.5-14 sessions) are complete

---

## Executive Summary

Phase 4 (Level Design Pipeline) spans four items: a RON preview tool, additional shape types, door/elevator mechanics, and a pickup system. After thorough code review, the split is clear:

- **RON preview tool with hot-reload** -- ENGINE. A standalone binary in a new `rust4d_tools` crate, leveraging the existing `AssetCache` hot-reload infrastructure and `rust4d_render` GPU pipeline.
- **Additional shape types** -- ENGINE (`rust4d_math` + `rust4d_core`). Currently only Tesseract4D and Hyperplane4D exist. Level geometry needs at minimum: Hypersphere4D, Ramp4D/Wedge4D, and Cylinder4D.
- **Door/elevator mechanics** -- Split. ENGINE provides a generic property interpolation/tween system and the trigger zone callback mechanism. GAME implements the actual door logic, key/door pairing, and elevator behavior.
- **Pickup system** -- Primarily GAME. ENGINE's trigger zones + event system (from P1) provide the foundation. GAME defines pickup types and effects.

Total engine-side effort: **4-6 sessions** (reduced from the original 4-6, since game-specific logic moves to the game repo).

---

## 1. Shape Type Inventory

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

### What's Missing for Level Geometry

Building real levels requires more geometric primitives. Here is a prioritized list of shapes needed:

#### P0 -- Required for basic levels

1. **Hypersphere4D** (renderable + collision)
   - A 4D sphere approximated by tetrahedra for GPU slicing
   - Already has collision primitive (`Sphere4D`), but no renderable version
   - Use: pillars, decorative objects, projectile visualization
   - Implementation: Icosphere-like subdivision in 4D, or parametric approach
   - Slicing a 4D sphere yields a 3D sphere (the cross-section), which is visually intuitive

2. **Ramp4D / Wedge4D** (renderable + collision)
   - A tesseract with one face sloped -- creates a ramp
   - Use: slopes, stairs (approximated), angled surfaces
   - Implementation: Modified tesseract with top vertices shifted along one axis
   - Collision: AABB approximation or per-face plane collision

3. **Wall4D** (renderable + collision)
   - A thin tesseract oriented as a vertical barrier
   - Could be a convenience constructor for Tesseract4D with asymmetric dimensions
   - Use: walls, barriers, partitions between rooms
   - Note: Currently `Tesseract4D` is always a perfect hypercube (all sides equal). Need to support **rectangular hyperprisms** (independent X/Y/Z/W dimensions).

#### P1 -- Expected for interesting levels

4. **Cylinder4D** (renderable + collision)
   - Circular cross-section in two dimensions, extruded in the other two
   - Use: pillars, columns, tubes, pipe corridors
   - Collision: Could use sphere-based approximation or custom cylinder test

5. **Cone4D / Frustum4D** (renderable)
   - Tapered cylinder
   - Use: decorative architecture, funnels, interesting geometry

6. **Composite shapes** (multiple primitives)
   - Allow entities to reference multiple shape templates
   - Use: L-shaped corridors, complex room geometry
   - This is a data model change, not a new primitive

#### Critical Gap: Rectangular Hyperprism

The most impactful single change is making `Tesseract4D` support independent axis dimensions. Currently:

```rust
// Current: all dimensions are equal
pub fn new(size: f32) -> Self {
    let h = size * 0.5;
    // vertices at +/-h on all four axes
}
```

Needed:

```rust
// Proposed: independent dimensions
pub fn new_rect(x_size: f32, y_size: f32, z_size: f32, w_size: f32) -> Self {
    // vertices at +/-(x_size/2), +/-(y_size/2), etc.
}
```

This single change enables walls (thin in one axis), floors (thin in Y), platforms (thin in Y and W), and corridors (long in one axis). The `ShapeTemplate` would gain a `RectHyperprism` variant.

### Shape Type Implementation Plan

**Session 1 (1 session): Core shape expansion**

- Generalize `Tesseract4D` to support rectangular dimensions (or add `Hyperprism4D`)
- Add `Hypersphere4D` renderable shape (icosphere-like tetrahedral decomposition)
- Add corresponding `ShapeTemplate` variants with serde support
- Update `crates/rust4d_core/src/shapes.rs` with new variants
- Tests for vertex counts, serialization round-trips, and tetrahedra validity

**Implementation details for Hypersphere4D:**

A 4D sphere can be approximated by recursively subdividing a regular polytope (e.g., the 24-cell or 600-cell). For a first pass, start with the 16-cell (4D analog of octahedron, 8 vertices) and subdivide. Each subdivision step:
1. Take a tetrahedron
2. Find the midpoint of each edge
3. Project midpoints onto the sphere surface
4. Create new tetrahedra from the subdivided simplex

At subdivision level 2, this yields a reasonable approximation (~200-500 tetrahedra) suitable for GPU slicing.

**Implementation details for Hyperprism4D (rectangular tesseract):**

This is a minimal change to `Tesseract4D`:
- Store `half_extents: Vec4` instead of `half_size: f32`
- Vertex generation uses `half_extents.x`, `.y`, `.z`, `.w` per axis
- Tetrahedra decomposition is identical (Kuhn triangulation works on any rectangular parallelepiped)
- `ShapeTemplate::Hyperprism { x: f32, y: f32, z: f32, w: f32 }` variant

### Crate Placement

| Shape | Crate | Rationale |
|-------|-------|-----------|
| `Hyperprism4D` | `rust4d_math` | Pure geometry, extends existing pattern |
| `Hypersphere4D` | `rust4d_math` | Pure geometry, new primitive |
| `ShapeTemplate` variants | `rust4d_core` | Serialization bridge |
| Collision shape updates | `rust4d_physics` | Already has collision primitives |

---

## 2. RON Preview Tool Architecture

### Purpose

The RON preview tool is a standalone application that:
1. Opens a RON scene file
2. Renders it using the existing `rust4d_render` GPU pipeline
3. Watches the file for changes and hot-reloads automatically
4. Provides camera controls and W-slice navigation for inspecting the scene

This is the critical "level design feedback loop" that makes 4D level creation tractable without a full editor. A level designer edits RON in a text editor, saves, and immediately sees the result.

### Current Hot-Reload Infrastructure

The engine already has hot-reload support via `AssetCache`:

- `AssetCache::check_hot_reload<T>()` compares file modification times and reloads changed assets (lines 243-294 of `asset_cache.rs`)
- `AssetCache::set_watch_for_changes(true)` enables polling
- The `Asset` trait requires `load_from_file(path) -> Result<Self, AssetError>`
- `Scene` already implements `load()` from RON files

**What's missing:**
- `Scene` does not implement the `Asset` trait (it has its own `Scene::load()` method)
- There is no standalone viewer binary -- the only binary is the game's `src/main.rs`
- The hot-reload polls but does not use filesystem watchers (no `notify` crate) -- this is fine for a preview tool (poll every frame or every 500ms)
- No mechanism to re-instantiate an `ActiveScene` from a reloaded `Scene` template

### Architecture Design

```
rust4d_tools/                        # NEW crate in engine workspace
    src/
        bin/
            ron_preview.rs           # The preview tool binary
        lib.rs                       # Shared tool utilities (optional)
```

**Why a new crate (rust4d_tools) rather than an example:**
- Tools need their own dependencies (winit for window, possibly egui for UI overlays)
- Tools are ENGINE artifacts, not examples for learning
- Separating tools from the core library keeps the library dependency-free
- The tool can depend on all engine crates without polluting them

**Alternative: just an example binary.** This is simpler and avoids creating a whole new crate. For the first iteration, an example binary (`examples/ron_preview.rs`) is acceptable. Promote to `rust4d_tools` when it needs its own dependencies beyond what examples already have.

**Recommendation: Start as `examples/ron_preview.rs`, promote to `rust4d_tools` if/when egui overlay is added.**

### Preview Tool Feature Set

**Core features (Session 1 of 2):**

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

**Enhanced features (Session 2 of 2, optional):**

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

8. **Screenshot/export**
   - Save current view as PNG
   - Useful for documentation and level design iteration

### Hot-Reload Implementation Details

The reload cycle:

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

Error resilience is critical. The tool must never crash on a malformed RON file -- log the error and keep showing the last good state. The RON parser already provides `SpannedError` with line/column information.

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

This is essentially the same dependency set as the current tech demo binary. The preview tool IS a specialized version of the tech demo, minus game logic plus hot-reload.

---

## 3. Engine-Level Animation/Interpolation System

### Why the Engine Needs This

Doors, elevators, platforms, and any moving level geometry require interpolating properties over time. This is not game-specific -- it is a fundamental engine capability that any 4D game would use.

The engine currently has NO animation or interpolation system. Physics bodies can have velocity, but there is no way to smoothly move an entity from point A to point B over a duration.

### Design: Property Tween System

A lightweight tween system in `rust4d_game`:

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

pub enum TweenState {
    Running,
    Completed,
    Paused,
}

pub enum EasingFunction {
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    // etc.
}

pub trait Interpolatable: Clone {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}

// Implement for engine types
impl Interpolatable for f32 { ... }
impl Interpolatable for Vec4 { ... }
impl Interpolatable for Transform4D { ... }  // lerp position, slerp rotation
```

### Tween Manager

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

    /// Update all active tweens, returns list of completed entities
    pub fn update(&mut self, dt: f32, world: &mut hecs::World) -> Vec<hecs::Entity>;
}
```

### Engine vs Game Boundary

| Component | Location | Rationale |
|-----------|----------|-----------|
| `Interpolatable` trait | `rust4d_math` | Pure math utility |
| `EasingFunction` enum | `rust4d_game` | Common game pattern |
| `Tween<T>` struct | `rust4d_game` | Common game pattern |
| `TweenManager` | `rust4d_game` | Manages entity tweens |
| Door logic (open/close FSM) | Game repo | Game-specific behavior |
| Elevator logic (waypoints, timing) | Game repo | Game-specific behavior |
| Key/door pairing | Game repo | Game-specific data |

The engine provides the interpolation primitives. The game uses them to build door/elevator behaviors.

### What the Game Builds

```rust
// In Rust4D-Shooter (game repo)

/// Door component
struct Door {
    closed_position: Vec4,
    open_position: Vec4,
    open_duration: f32,
    state: DoorState,
    required_key: Option<KeyColor>,
}

enum DoorState {
    Closed,
    Opening,
    Open(f32), // timer until close
    Closing,
}

/// Door system: triggered by events from engine trigger zones
fn door_system(world: &mut World, tweens: &mut TweenManager, events: &EventBus) {
    for event in events.read::<TriggerEnterEvent>() {
        if let Some(door) = world.get::<Door>(event.trigger_entity) {
            // Check key requirement, then tween open
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

---

## 4. Declarative Trigger System Design

### Context

The B2 agent recommended a declarative RON trigger system as a lightweight alternative to full scripting. The collision layer system already has a `TRIGGER` layer and `CollisionFilter::trigger()`, but there are NO callbacks -- the engine can detect trigger overlaps but cannot notify anyone about them.

### What the Engine Needs

The engine needs two things:
1. **Trigger zone collision detection with callbacks** (from P1 event system)
2. **A way to define trigger behavior in RON scene files**

The first is P1's responsibility (event system + trigger callbacks). P4's contribution is the declarative format.

### Declarative Trigger Format

Add an optional `triggers` section to the `Scene` RON format:

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

### Trigger System Components

```rust
// In rust4d_core or rust4d_game

/// Serializable trigger definition
#[derive(Serialize, Deserialize)]
pub struct TriggerDef {
    pub name: String,
    pub zone: TriggerZone,
    pub detects: Vec<CollisionLayer>,
    pub actions: Vec<TriggerAction>,
    pub repeat: TriggerRepeat,
}

#[derive(Serialize, Deserialize)]
pub enum TriggerZone {
    AABB { center: [f32; 4], half_extents: [f32; 4] },
    Sphere { center: [f32; 4], radius: f32 },
}

#[derive(Serialize, Deserialize)]
pub enum TriggerAction {
    /// Tween an entity's position (engine-level)
    TweenPosition {
        target_entity: String,
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
    /// Enable/disable another trigger
    SetTriggerEnabled { trigger: String, enabled: bool },
}

#[derive(Serialize, Deserialize)]
pub enum TriggerRepeat {
    Once,
    Cooldown(f32), // seconds between activations
    Always,
}
```

### Engine vs Game Boundary for Triggers

| Component | Location | Rationale |
|-----------|----------|-----------|
| `TriggerDef` struct | `rust4d_core` (scene data) | Serializable scene data |
| `TriggerZone` enum | `rust4d_core` | Maps to physics collision shapes |
| `TriggerAction::TweenPosition` | `rust4d_game` | Uses tween system |
| `TriggerAction::GameEvent` | `rust4d_game` | Fires named events on the event bus |
| `TriggerAction::DespawnSelf` | `rust4d_game` | ECS entity removal |
| Trigger runtime system | `rust4d_game` | Processes triggers each frame |
| Game-specific event handlers | Game repo | Interprets `GameEvent` strings |

The key insight: `TriggerAction::GameEvent(String)` is the escape hatch. The engine defines a small set of built-in actions (tween, despawn, sound). For anything game-specific (heal player, give ammo, spawn enemies), the trigger fires a named event and the GAME handles it. This keeps the engine generic while allowing the game to define arbitrarily complex trigger responses.

---

## 5. 4D-Specific Level Design Considerations

### W-Layered Rooms

A core 4D level design pattern: rooms that exist at different W-coordinates, connected by transitions. The engine must support:

1. **W-portals** -- Regions where the player transitions from one W-layer to another
   - Implementation: A trigger zone that, when entered, shifts the player's W-coordinate
   - This is just a `TriggerAction::TweenPosition` targeting the player, moving only the W component
   - Or a special `TriggerAction::ShiftW { delta: f32 }` for clarity

2. **W-visibility ranges** -- Objects should only be rendered when near the current W-slice
   - Already partially handled by the GPU slicing pipeline (objects far from the W-slice produce no cross-section triangles)
   - May need a LOD-like system to skip GPU processing for objects far from the current W-slice
   - The `Hyperplane4D` already has a `w_extent` parameter controlling W visibility range

3. **W-layered collision** -- Physics should only collide objects in the same W-layer
   - Already handled by the 4D collision system (AABB collision checks all 4 dimensions)
   - An object at W=10 naturally does not collide with one at W=0 (unless they both span W=0 to W=10)

### Level Design Patterns for RON

The RON format should support level design patterns:

```ron
// A room at W=0
EntityTemplate(
    name: "room_w0_floor",
    tags: ["static"],
    transform: Transform4D(position: Vec4(x: 0, y: 0, z: 0, w: 0), ...),
    shape: Hyperprism(x: 20.0, y: 0.5, z: 20.0, w: 2.0),  // thin in W
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
        TweenPosition(target_entity: "player", to_w: 5.0, duration: 0.5, easing: EaseInOutQuad),
    ],
    repeat: Always,
),
```

### Entity Prefabs (Future)

For larger levels, repeating entity groups becomes painful. A prefab system would help:

```ron
Prefab(
    name: "standard_door",
    entities: [
        EntityTemplate(name: "door_frame", ...),
        EntityTemplate(name: "door_panel", tags: ["door"], ...),
    ],
    triggers: [
        TriggerDef(name: "door_open_trigger", ...),
    ],
)
```

This is NOT in scope for Phase 4 but informs the design. The trigger and entity naming systems should support namespacing (e.g., `prefab_instance_1.door_panel`).

---

## 6. Dependencies

### Dependencies on Foundation (from Agent F)

| Foundation Item | P4 Dependency | Why |
|----------------|---------------|-----|
| Serialization (Rotor4 Serialize) | Required | Scene templates contain Transform4D with Rotor4 rotations |
| Fixed timestep | Required | Tween system needs consistent dt for smooth animation |
| ECS migration complete | Required | Trigger system references entities by hecs::Entity |

### Dependencies on P1 (Combat Core)

| P1 Item | P4 Dependency | Why |
|---------|---------------|-----|
| Event system | Required | Trigger actions fire events; the game receives them |
| Trigger zone callbacks | Required | Triggers need to know when entities enter/exit zones |

### Dependencies on P2 (Weapons & Feedback)

| P2 Item | P4 Dependency | Why |
|---------|---------------|-----|
| Audio system | Optional | `TriggerAction::PlaySound` needs audio playback |

### Dependencies on P3 (Enemies & AI)

None. P4 is independent of P3.

### What P5 (Editor) Depends on from P4

| P4 Item | P5 Usage |
|---------|----------|
| All shape types | Editor property panel shows shape parameters |
| ShapeTemplate variants | Editor creates entities with any shape type |
| TriggerDef format | Editor visualizes and edits trigger zones |
| RON preview tool | Editor may share camera/render code with preview tool |

---

## 7. Implementation Plan

### Wave 1: Shape Type Expansion (1 session)

**Goal:** Add rectangular hyperprisms and hypersphere to unlock diverse level geometry.

**Tasks:**
- [ ] Add `Hyperprism4D` to `rust4d_math` (tesseract with independent axis dimensions)
  - `Hyperprism4D::new(x_size, y_size, z_size, w_size)`
  - Kuhn triangulation works identically to Tesseract4D
  - Keep `Tesseract4D::new(size)` as convenience (delegates to Hyperprism with equal dimensions)
- [ ] Add `Hypersphere4D` to `rust4d_math` (subdivided 4D sphere)
  - Start from 16-cell, subdivide edges, project to sphere
  - Parameterized by radius and subdivision level
  - Implements `ConvexShape4D`
- [ ] Add `ShapeTemplate::Hyperprism { x, y, z, w }` and `ShapeTemplate::Hypersphere { radius, subdivisions }` to `rust4d_core/src/shapes.rs`
- [ ] Update `ShapeTemplate::create_shape()` for new variants
- [ ] RON serialization/deserialization tests
- [ ] Update scene examples to use new shapes
- [ ] Verify GPU slicing works with new shapes (render test)

**Files modified:**
- `crates/rust4d_math/src/lib.rs` -- export new types
- `crates/rust4d_math/src/hyperprism.rs` -- NEW
- `crates/rust4d_math/src/hypersphere.rs` -- NEW
- `crates/rust4d_core/src/shapes.rs` -- add ShapeTemplate variants

**Verification:**
- New shapes produce valid tetrahedra (all indices in range, non-degenerate)
- Round-trip RON serialization preserves all parameters
- GPU slicing produces visible geometry at the correct W-slice positions

### Wave 2: Declarative Trigger Data Model (0.5 session)

**Goal:** Define the trigger format in the scene data model, serializable to/from RON.

**Prerequisite:** Foundation serialization must be done.

**Tasks:**
- [ ] Define `TriggerDef`, `TriggerZone`, `TriggerAction`, `TriggerRepeat` types in `rust4d_core`
- [ ] Add `triggers: Vec<TriggerDef>` field to `Scene` struct (optional, defaults to empty)
- [ ] Serde derive for all trigger types
- [ ] RON round-trip tests
- [ ] Update `SceneValidator` to validate trigger references (target entities exist)

**Files modified:**
- `crates/rust4d_core/src/trigger.rs` -- NEW: trigger data types
- `crates/rust4d_core/src/scene.rs` -- add triggers field to Scene
- `crates/rust4d_core/src/scene_validator.rs` -- validate trigger references
- `crates/rust4d_core/src/lib.rs` -- export trigger module

**Note:** The trigger RUNTIME (checking overlaps, executing actions) lives in `rust4d_game` and depends on P1's event system. This wave only defines the data model.

### Wave 3: Tween/Interpolation System (0.5 session)

**Goal:** Engine-level property interpolation for smooth movement of level geometry.

**Prerequisite:** ECS migration complete, fixed timestep.

**Tasks:**
- [ ] Add `Interpolatable` trait to `rust4d_math` (lerp for f32, Vec4)
- [ ] Implement `Transform4D` interpolation (lerp position, slerp rotation via Rotor4)
- [ ] Add `EasingFunction` enum to `rust4d_game`
- [ ] Add `Tween<T>` struct to `rust4d_game`
- [ ] Add `TweenManager` to `rust4d_game` (manages entity tweens)
- [ ] Tests: easing functions, tween completion, tween update

**Files modified:**
- `crates/rust4d_math/src/interpolation.rs` -- NEW: Interpolatable trait, lerp/slerp
- `crates/rust4d_game/src/tween.rs` -- NEW: Tween, TweenManager, EasingFunction

### Wave 4: RON Preview Tool (2 sessions)

**Goal:** Standalone hot-reloading scene viewer for level design iteration.

**Prerequisites:** Wave 1 (shapes), Foundation complete.

**Session 4a: Core viewer (1 session)**
- [ ] Create `examples/ron_preview.rs` (or `rust4d_tools` if needed)
- [ ] Command-line argument parsing (scene file path)
- [ ] winit window + wgpu surface initialization (copy pattern from tech demo)
- [ ] Scene loading and ActiveScene instantiation
- [ ] Render loop using existing GPU pipeline
- [ ] Camera controls (reuse CameraController from rust4d_input)
- [ ] W-slice navigation (scroll wheel)
- [ ] File modification time polling (every 500ms)
- [ ] Hot-reload: re-parse RON, re-instantiate scene, preserve camera state
- [ ] Error resilience: log parse errors, keep showing last valid scene

**Session 4b: Enhanced viewer (1 session)**
- [ ] Entity information overlay (egui or window title)
- [ ] Entity list display
- [ ] W-position indicator
- [ ] Multiple W-slice views (side-by-side or overlaid)
- [ ] Physics visualization toggle (wireframe collision shapes)
- [ ] Scene statistics (entity count, tetrahedra count, FPS)

**Files created:**
- `examples/ron_preview.rs` -- main preview tool binary

**Dependencies at build time:**
- `rust4d_core`, `rust4d_math`, `rust4d_physics`, `rust4d_render`, `rust4d_input`
- `winit`, `wgpu` (already workspace dependencies)
- `env_logger` (already a dependency)
- `clap` or `std::env::args` for CLI parsing

### Wave 5: Trigger Runtime (0.5 session)

**Goal:** Wire up the declarative trigger system to actually execute at runtime.

**Prerequisites:** P1 event system and trigger callbacks, Wave 2 (trigger data model), Wave 3 (tween system).

**Tasks:**
- [ ] `TriggerRuntime` in `rust4d_game` that processes `TriggerDef` definitions
- [ ] Each frame: check trigger zones against entity positions
- [ ] When entity enters zone: execute `TriggerAction` list
- [ ] `TweenPosition` action: create tween via TweenManager
- [ ] `GameEvent` action: fire named event on event bus
- [ ] `DespawnSelf` action: mark entity for removal
- [ ] Respect `TriggerRepeat` (once, cooldown, always)
- [ ] Integration test: load scene with triggers, simulate player entering zone, verify actions fire

**Files modified:**
- `crates/rust4d_game/src/trigger_runtime.rs` -- NEW: trigger execution logic

---

## 8. What the Game Repo Builds On Top

The game repo (Rust4D-Shooter) uses the engine's level design infrastructure to implement game-specific mechanics:

### Door/Elevator Mechanics (1-2 sessions in game repo)

Using engine tween system + trigger system:
- `Door` component: closed_position, open_position, key requirement
- `Elevator` component: waypoints (Vec<Vec4>), speed, pause_at_each
- Door system: listens for trigger events, checks key inventory, starts tween
- Elevator system: cycles through waypoints using tween manager
- Key/door color system: Red/Blue/Yellow keys unlock matching doors

### Pickup System (0.5 session in game repo)

Using engine trigger zones + event system:
- `Pickup` component: type (health, ammo, weapon, key), amount
- Pickup entities with PICKUP collision layer
- When player enters pickup trigger zone: fire `GameEvent("pickup_health_50")`
- Game event handler: apply pickup effect, despawn pickup entity
- Respawn timer (optional): after N seconds, re-enable pickup

### Level Scripting (game repo)

Using declarative triggers for 80% of needs:
- Secret doors revealed by wall interaction
- Trap triggers (ceiling crushing, floor opening)
- Enemy spawn triggers (player enters area -> spawn wave)
- W-portal triggers (shift player to different W-layer)
- Completion triggers (all enemies dead -> open exit)

For the remaining 20%: custom Rust code in the game's systems.

---

## 9. Session Estimates Summary

| Wave | Task | Sessions | Dependencies |
|------|------|----------|-------------|
| Wave 1 | Shape type expansion | 1.0 | None (can start immediately) |
| Wave 2 | Declarative trigger data model | 0.5 | Foundation serialization |
| Wave 3 | Tween/interpolation system | 0.5 | ECS, fixed timestep |
| Wave 4 | RON preview tool | 2.0 | Wave 1, Foundation |
| Wave 5 | Trigger runtime | 0.5 | P1 events, Wave 2, Wave 3 |
| **Total Engine** | | **4.5** | |

### Parallelism

```
Wave 1 (Shape types) -----> Wave 4a (Preview tool core)
                        \--> Wave 4b (Preview tool enhanced)

Foundation (serialization) --> Wave 2 (Trigger data model) --\
                                                              --> Wave 5 (Trigger runtime)
ECS + Fixed timestep -------> Wave 3 (Tween system) --------/

P1 (Event system) ----------------------------------------/
```

- **Wave 1** can start immediately (no dependencies)
- **Wave 2** requires Foundation serialization (Rotor4 serde)
- **Wave 3** requires ECS and fixed timestep
- **Wave 4** requires Wave 1 + Foundation (needs shapes and serialization for scene loading)
- **Wave 5** requires P1 events + Wave 2 + Wave 3

- **Waves 1, 2, and 3 are independent of each other** and could be done in parallel
- **Wave 4** is the largest single task and benefits from Wave 1 being complete first
- **Wave 5** is the integration point and comes last

### Game Repo Effort (not counted in engine total)

| Task | Sessions | Dependencies |
|------|----------|-------------|
| Door/elevator mechanics | 1-2 | Engine Waves 3+5 |
| Pickup system | 0.5 | Engine Wave 5 (P1 events) |
| Level scripting | ongoing | Declarative triggers |

---

## 10. Open Questions

1. **Should `Hyperprism4D` replace `Tesseract4D` or coexist?**
   - Option A: Hyperprism with `new_cube(size)` convenience replaces Tesseract
   - Option B: Both exist, Tesseract delegates to Hyperprism internally
   - Recommendation: Option B. Keep Tesseract4D for API compatibility and clarity. Internally it can use Hyperprism's vertex generation.

2. **How many subdivision levels for Hypersphere4D?**
   - Too few: sphere looks blocky after slicing
   - Too many: thousands of tetrahedra per sphere, GPU performance concern
   - Recommendation: Default to subdivision level 2 (~200 tetrahedra). Allow user to specify in ShapeTemplate.

3. **Should the preview tool be headless-capable?**
   - A headless mode could validate RON scenes in CI (parse + validate, no rendering)
   - This is separate from the preview tool -- could be a `ron_validate` example
   - Recommendation: Defer headless validation to P5 (editor).

4. **Trigger actions that reference entities by name: what happens if the entity doesn't exist?**
   - Option A: Panic (fail-fast, catch at load time)
   - Option B: Log warning, skip action (resilient)
   - Recommendation: Validate at scene load time (SceneValidator). At runtime, log warning and skip.

5. **For Agent P1**: Does the event system support named events (string-based)? The declarative trigger system assumes `GameEvent(String)` which the game interprets. If P1's event system is typed, we need an `AnyEvent` or `NamedEvent` variant.

---

## 11. Cross-Agent Notes

### For Agent F (Foundation)
- P4 needs Rotor4 Serialize/Deserialize for scene hot-reload (Transform4D round-trips through RON)
- P4 needs fixed timestep for the tween system

### For Agent P1 (Combat Core)
- P4 depends on trigger zone callbacks. The trigger system defined here is the *declarative* layer on top of P1's *imperative* trigger detection.
- P4 needs the event system to support string-named events for `GameEvent` trigger actions.

### For Agent P2 (Weapons & Feedback)
- `TriggerAction::PlaySound` depends on P2's audio system. This action type can be deferred if audio isn't ready.

### For Agent P5 (Editor)
- The RON preview tool (Wave 4) shares rendering infrastructure with the editor. The preview tool could become the foundation for the editor's viewport.
- Shape type expansion (Wave 1) directly feeds the editor's entity creation UI.
- The trigger data model (Wave 2) needs editor visualization (trigger zones drawn as wireframes).
