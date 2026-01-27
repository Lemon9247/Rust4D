# Codebase Review: Rust4D Architecture Analysis

**Agent:** Codebase Review Agent
**Date:** 2026-01-27
**Task:** Analyze current architecture for refactoring planning

---

## Executive Summary

Rust4D is a 4D rendering engine in early development. The current architecture has a clean separation into crates (`rust4d_core`, `rust4d_physics`, `rust4d_render`, `rust4d_math`, `rust4d_input`), but **`main.rs` has grown into a monolithic "god struct"** that handles application lifecycle, physics orchestration, rendering, input processing, and scene setup. This document identifies specific pain points and coupling issues that should be addressed in the upcoming refactoring.

---

## Current Architecture Diagram

```
                         +----------------+
                         |    main.rs     |
                         |  (App struct)  |
                         +-------+--------+
                                 |
          +----------------------+----------------------+
          |                      |                      |
          v                      v                      v
+------------------+   +------------------+   +------------------+
|  rust4d_render   |   |  rust4d_physics  |   |   rust4d_core    |
|  - Camera4D      |   |  - PhysicsWorld  |   |   - World        |
|  - SlicePipeline |   |  - PlayerPhysics |   |   - Entity       |
|  - RenderPipeline|   |  - RigidBody4D   |   |   - Transform4D  |
|  - Renderable    |   |  - Colliders     |   |   - Material     |
+------------------+   +------------------+   +------------------+
          |                      |                      |
          +----------------------+----------------------+
                                 |
                                 v
                         +----------------+
                         |  rust4d_math   |
                         |  - Vec4        |
                         |  - Rotor4      |
                         |  - Shapes      |
                         +----------------+
```

**Data Flow:**
```
main.rs creates entities --> World stores entities --> World has PhysicsWorld
    ^                                                        |
    |                                                        v
    +-- syncs positions <-- World.update() <-- PhysicsWorld.step()
    |
    +-- rebuilds geometry --> RenderableGeometry --> GPU upload
```

---

## 1. main.rs Analysis

### Current Responsibilities (App struct)

The `App` struct in `main.rs` currently handles:

| Responsibility | Lines | Should Move To |
|----------------|-------|----------------|
| Window management | ~20 | GameEngine/Runtime |
| Render context lifecycle | ~30 | Renderer |
| Pipeline creation/management | ~40 | Renderer |
| **Scene setup (hardcoded entities)** | ~30 | **Scene/Registry** |
| Player physics management | ~50 | **PlayerController** |
| Game loop orchestration | ~200 | **GameEngine** |
| Input-to-physics translation | ~40 | **PlayerController** |
| Entity-player collision | ~40 | **CollisionSystem** |
| Camera sync to physics | ~10 | **PlayerController** |
| Geometry rebuild detection | ~15 | **RenderSystem** |
| Window title updates | ~15 | UI/Debug |

**Total: ~490 lines of mixed concerns in one struct.**

### Hardcoded Values in main.rs

```rust
const GRAVITY: f32 = -20.0;           // Should be in PhysicsConfig
const FLOOR_Y: f32 = -2.0;            // Should be in scene data

// In App::new():
let tesseract_start = Vec4::new(0.0, 0.0, 0.0, 0.0);  // Hardcoded position
let player_start = Vec4::new(0.0, 0.0, 5.0, 0.0);      // Hardcoded position
let tesseract = Tesseract4D::new(2.0);                 // Hardcoded size
let floor = Hyperplane4D::new(FLOOR_Y, 10.0, 10, 5.0, 0.001);  // Hardcoded floor

// In build_geometry():
if idx == 0 { ... }  // Entity type determined by index position!
```

### The Entity Index Problem

The most concerning pattern is entity identification by array index:
```rust
for (idx, entity) in world.iter().enumerate() {
    if idx == 0 {
        // Tesseract: use position gradient
        geometry.add_entity_with_color(entity, &position_gradient_color);
    } else {
        // Floor: use checkerboard pattern
        geometry.add_entity_with_color(entity, &|v, _m| {
            checkerboard.color_for_position(v.x, v.z)
        });
    }
}
```

This breaks if entities are reordered, added, or removed.

---

## 2. Entity System Analysis

### Current Implementation (`rust4d_core/src/entity.rs`)

```rust
pub struct Entity {
    pub transform: Transform4D,
    pub shape: ShapeRef,           // Geometry (Arc or Box)
    pub material: Material,        // Visual properties
    pub physics_body: Option<BodyHandle>,  // Optional physics link
}
```

### Strengths
- Clean separation of visual (shape, material) and physics (body handle)
- ShapeRef allows shared vs owned geometry
- Transform4D supports position, rotation, scale

### Limitations

1. **No entity identity beyond array index**
   - EntityHandle is just a `usize` wrapper
   - No name, tag, or type field
   - No way to query "give me all walls" or "find the player"

2. **No component extensibility**
   - Can't attach arbitrary data (e.g., health, AI state, audio source)
   - Each new property requires modifying Entity struct

3. **No lifecycle hooks**
   - No on_spawn, on_destroy, on_update callbacks
   - No way to run per-entity logic

4. **Static vs dynamic not distinguished**
   - Floor and tesseract both use Entity
   - No efficient way to skip static geometry in certain operations

### World Container (`rust4d_core/src/world.rs`)

```rust
pub struct World {
    entities: Vec<Entity>,
    physics_world: Option<PhysicsWorld>,
}
```

**Missing Features:**
- No entity removal (only add)
- No entity lookup by name/tag/type
- No spatial queries (find entities near point)
- No change detection (which entities moved?)

---

## 3. Physics System Analysis

### PhysicsWorld (`rust4d_physics/src/world.rs`)

```rust
pub struct PhysicsWorld {
    bodies: Vec<RigidBody4D>,
    floor: Plane4D,         // <-- HARDCODED SINGLE FLOOR
    pub config: PhysicsConfig,
}
```

### Pain Points

1. **Single hardcoded floor**
   - `floor: Plane4D` is baked into PhysicsWorld
   - No way to add multiple floors, walls, or arbitrary static colliders
   - Floor is separate from entities (not in the World)

2. **No static collider concept**
   - `RigidBody4D.is_static` exists but:
     - Static bodies still go through the full physics step
     - Floor plane is a completely different type

3. **Missing friction model**
   ```rust
   pub struct PhysicsConfig {
       pub gravity: f32,
       pub floor_y: f32,
       pub restitution: f32,
       // NO friction field!
   }
   ```

4. **No collision layers/masks**
   - Every body can collide with every other body
   - Can't say "player collides with walls but not trigger zones"

5. **Player physics is separate from world physics**
   ```rust
   // In main.rs - PlayerPhysics is NOT in PhysicsWorld!
   player_physics: PlayerPhysics,
   physics_floor: PhysicsPlane,  // Separate floor for player!
   ```
   - Two separate collision systems running in parallel
   - Manual collision code between player and world entities

6. **Collision response is basic**
   - Only position correction and velocity bounce
   - No events/callbacks for collision start/end
   - No contact point information returned to game logic

### RigidBody4D (`rust4d_physics/src/body.rs`)

```rust
pub struct RigidBody4D {
    pub position: Vec4,
    pub velocity: Vec4,
    pub mass: f32,
    pub restitution: f32,
    pub affected_by_gravity: bool,
    pub collider: Collider,
    pub is_static: bool,
    // No angular velocity/rotation
    // No friction
    // No custom collision group
}
```

---

## 4. Rendering Pipeline Analysis

### Entity-Render Coupling

The rendering system has a tight coupling through `RenderableGeometry`:

```rust
// rust4d_render/src/renderable.rs
pub struct RenderableGeometry {
    pub vertices: Vec<Vertex4D>,
    pub tetrahedra: Vec<GpuTetrahedron>,
}
```

**Problem:** Geometry is rebuilt entirely when ANY entity moves:

```rust
// In main.rs RedrawRequested:
if current_pos != self.last_tesseract_pos {
    self.geometry = Self::build_geometry(&self.world);  // FULL REBUILD
    slice_pipeline.upload_tetrahedra(...);              // FULL RE-UPLOAD
}
```

This is O(n) for all entities when one entity moves.

### Missing Features

1. **No dirty tracking per entity**
   - Can't know which entities changed
   - Can't incrementally update geometry

2. **No render batching by material/shader**
   - Each entity is processed individually
   - No instancing support

3. **No culling**
   - All geometry goes to GPU regardless of visibility
   - 4D distance culling would be valuable

### Coloring System

Coloring is handled via callbacks in `build_geometry`:
```rust
if idx == 0 {
    geometry.add_entity_with_color(entity, &position_gradient_color);
} else {
    geometry.add_entity_with_color(entity, &checkerboard.color_fn());
}
```

This is inflexible. Color function should be part of entity or material.

---

## 5. Scene Management

### Current State: No Scene Management

There is no scene concept. Everything is hardcoded in `App::new()`:

```rust
fn new() -> Self {
    // Physics config
    let physics_config = PhysicsConfig::new(GRAVITY, FLOOR_Y, 0.0);
    let mut world = World::with_capacity(2).with_physics(physics_config);

    // Create tesseract (manually)
    let tesseract_body = RigidBody4D::new_aabb(...);
    let tesseract_body = world.physics_mut().unwrap().add_body(tesseract_body);
    let tesseract = Tesseract4D::new(2.0);
    let tesseract_entity = world.add_entity(...);

    // Create floor (manually)
    let floor = Hyperplane4D::new(...);
    world.add_entity(...);

    // Create player (manually, NOT in World!)
    let player_physics = PlayerPhysics::new(player_start);
    // ...
}
```

### What's Missing

1. **Scene file format** - No way to load/save scenes
2. **Prefabs/templates** - Can't define "spawn a standard enemy"
3. **Entity registry** - No central place to look up entities
4. **Named references** - Can't reference "the player" or "main floor"
5. **Runtime spawn/despawn** - No API for adding entities during gameplay

---

## 6. Coupling Issues Summary

### Tight Couplings

| From | To | Issue |
|------|-----|-------|
| main.rs | All crates | Orchestrates everything directly |
| main.rs | Entity indices | Uses array position for identification |
| PhysicsWorld | Plane4D floor | Hardcoded single floor |
| PlayerPhysics | main.rs | Not integrated with World/PhysicsWorld |
| RenderableGeometry | World | Full rebuild on any change |
| build_geometry | Entity index | Color logic based on index |

### Missing Abstractions

1. **Scene** - Collection of entity definitions that can be loaded
2. **EntityRegistry** - Name/tag-based entity lookup
3. **StaticCollider** - Efficient static geometry for physics
4. **CollisionLayer** - Control what collides with what
5. **RenderBatch** - Group entities for efficient rendering
6. **DirtyFlags** - Track what changed for incremental updates

---

## 7. Specific Pain Points for Refactoring Goals

### Goal: Scene Registry

**Current Pain:**
- Entities identified by Vec index only
- No way to query by type/name/tag
- Floor is hardcoded, not a "named entity"

**Needed:**
- `EntityId` that's stable across add/remove
- Entity tags/names for lookup
- Scene serialization format

### Goal: main.rs Decomposition

**Current Pain:**
- App struct is 500+ lines
- Mixes window, render, physics, input, game logic
- Hard to test any component in isolation

**Needed:**
- GameEngine struct that composes systems
- Clear system boundaries (PhysicsSystem, RenderSystem, etc.)
- Frame-based update loop that calls systems

### Goal: Physics Improvements

**Current Pain:**
- Single hardcoded floor plane
- No friction on any surface
- PlayerPhysics outside World
- Entity-entity collision in main.rs, not physics system

**Needed:**
- Static colliders as entities (or separate list)
- Friction coefficient per surface/body
- Player as regular physics entity
- Collision events/callbacks

### Goal: Entity Composition

**Current Pain:**
- Entity struct has fixed fields
- Can't add custom data without modifying struct
- Physics and rendering tightly coupled

**Needed:**
- Component pattern (either full ECS or simpler)
- Optional components (not all entities have physics)
- Custom components for game logic

---

## 8. Recommendations

### Immediate Priorities

1. **Extract scene setup from main.rs**
   - Create `SceneBuilder` or `SceneLoader`
   - Define entities in data, not code

2. **Add entity identification**
   - Add `name: Option<String>` or `tags: Vec<String>` to Entity
   - Create lookup methods on World

3. **Integrate PlayerPhysics into PhysicsWorld**
   - Make player a regular body
   - Remove duplicate floor plane

4. **Add static colliders to PhysicsWorld**
   - `static_colliders: Vec<Collider>` separate from bodies
   - Or: static bodies that are efficiently skipped

### Medium-term

5. **Decompose App struct**
   - Extract `Renderer`, `PhysicsSystem`, `InputSystem`
   - Main loop just calls `system.update()`

6. **Incremental geometry updates**
   - Track which entities changed
   - Only rebuild changed portions

7. **Add friction to physics**
   - Per-body `friction: f32`
   - Apply during floor collision

### Long-term Considerations

8. **Consider ECS adoption**
   - Could use `hecs`, `bevy_ecs`, or custom
   - But: adds complexity, may not be needed yet

9. **Scene serialization**
   - RON or TOML scene files
   - Hot-reload support

---

## Appendix: File Locations

| Concept | File |
|---------|------|
| App (monolith) | `/home/lemoneater/Projects/Personal/Rust4D/src/main.rs` |
| Entity | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_core/src/entity.rs` |
| World | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_core/src/world.rs` |
| PhysicsWorld | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_physics/src/world.rs` |
| PlayerPhysics | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_physics/src/player.rs` |
| RigidBody4D | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_physics/src/body.rs` |
| Colliders | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_physics/src/shapes.rs` |
| RenderableGeometry | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_render/src/renderable.rs` |
| Camera4D | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_render/src/camera4d.rs` |
| CameraController | `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_input/src/camera_controller.rs` |

---

*Report generated by Codebase Review Agent*
