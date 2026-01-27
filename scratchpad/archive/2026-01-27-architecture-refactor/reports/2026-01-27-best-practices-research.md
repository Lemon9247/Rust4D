# Game Engine Best Practices Research Report

**Date:** 2026-01-27
**Agent:** Best Practices Research Agent
**Purpose:** Research industry best practices for game engine architecture to inform Rust4D refactoring

---

## Executive Summary

This report synthesizes research on game engine architecture patterns, physics systems, rendering pipelines, and 4D-specific considerations. The key recommendation for Rust4D is to adopt a **lightweight component-based architecture** rather than full ECS, implement **generational handles** for entity references, and add **dirty tracking** for efficient rendering.

---

## 1. Entity-Component-System (ECS) vs Alternatives

### What is ECS?

Entity-Component-System is an architectural pattern where:
- **Entities** are lightweight IDs (typically just integers)
- **Components** are pure data structures (Position, Velocity, Health)
- **Systems** are functions that operate on entities with specific component combinations

ECS separates data from behavior and encourages composition over inheritance.

### When Full ECS is Worth It

Full ECS excels when:
- Processing 100s-1000s of similar entities (RTS games, particle systems)
- Parallelization is critical (multi-threaded game loops)
- Components are frequently added/removed at runtime
- Cache-friendly memory access patterns matter

Real-world examples: Overwatch processes thousands of entities at 60Hz using ECS. Unity's DOTS handles massive crowd simulations.

### When Simpler Patterns Suffice

Full ECS may be overkill for:
- Small prototypes and early-stage projects
- Games with relatively few entities (<100)
- Narrative-driven games with mostly unique objects
- Projects where learning curve investment isn't justified

### Recommendation for Rust4D

Given Rust4D's current scope (a handful of entities in early development), I recommend a **lightweight Entity-Component (EC) pattern** without the full Systems scheduling machinery:

```rust
// Lightweight component system - entities own their components
struct Entity {
    id: EntityId,
    transform: Transform4D,
    physics: Option<PhysicsBody>,
    mesh: Option<Mesh4D>,
    collider: Option<Collider4D>,
    // ... other optional components
}
```

This provides composition benefits without ECS complexity. If Rust4D grows to need full ECS, the migration path is straightforward.

**Sources:**
- [Entity Component System - Wikipedia](https://en.wikipedia.org/wiki/Entity_component_system)
- [ECS FAQ - SanderMertens](https://github.com/SanderMertens/ecs-faq)
- [Game Programming Patterns - Component](https://gameprogrammingpatterns.com/component.html)

---

## 2. Scene Management Patterns

### Scene Graph vs Flat Registry

**Traditional Scene Graph:**
- Hierarchical tree structure (parent-child relationships)
- Transform inheritance (move parent, children follow)
- Used for skeletal animation, attached objects (weapons in hands)

**Modern Approach - Multiple Specialized Structures:**
- **Transform hierarchy** - Only for parent-child relationships (usually shallow)
- **Spatial acceleration structure** - Octree/quadtree for collision/rendering queries
- **Render queue** - Flat list sorted for optimal GPU submission
- **Entity registry** - Flat lookup table with handles

The consensus is that monolithic scene graphs are "outdated junk for modern games." Keep structures specialized for their purpose.

### Handle-Based Entity References

**Generational Indices** are preferred over raw pointers:

```rust
struct EntityHandle {
    index: u32,       // Index into entity array
    generation: u32,  // Increments when entity is destroyed
}
```

Benefits:
- **Dangling reference detection** - Stale handles detected by comparing generations
- **Serialization friendly** - IDs can be saved/loaded, unlike pointers
- **Network sync** - IDs work across network boundaries
- **No ownership complexity** - Handles don't own data, can be freely copied
- **Memory pool friendly** - Slots can be reused safely

Implementation notes:
- When entity destroyed, increment generation and add slot to free list
- On access, compare handle's generation with slot's current generation
- Consider disabling slots when generation counter overflows (rare but possible)

### Recommendation for Rust4D

1. Implement `EntityHandle` with generational indices
2. Store entities in a pool/registry with fast lookup
3. Keep transform hierarchy separate and shallow (only for attached objects)
4. Add spatial structure later when needed for broad-phase collision

**Sources:**
- [Handles are the Better Pointers](https://floooh.github.io/2018/06/17/handles-vs-pointers.html)
- [Game Engine Containers - handle_map](https://www.gamedev.net/tutorials/programming/general-and-gameplay-programming/game-engine-containers-handle_map-r4495/)
- [Scene Graphs - Wisp Wiki](https://teamwisp.github.io/research/scene_graph.html)

---

## 3. Physics Engine Architecture

### Broad Phase vs Narrow Phase Collision

Physics engines split collision detection into two phases:

**Broad Phase (Fast, Conservative):**
- Uses bounding volumes (AABB, spheres) to quickly eliminate non-colliding pairs
- Scales well (avoid O(n^2) comparisons)
- Common algorithms:
  - **Sweep and Prune** - Sort AABBs along axes
  - **Spatial Partitioning** - Octrees, grids, BVH trees
  - **Dynamic BVH** - Binary tree of AABBs, good for moving objects

**Narrow Phase (Precise, Expensive):**
- Exact collision detection on potentially-colliding pairs
- Computes contact points, normals, penetration depth
- Algorithms: GJK, SAT, specialized convex hull tests

### Static vs Dynamic Colliders

Static colliders (walls, floors, terrain) should be handled differently:

1. **Separate static acceleration structure** - Built once, rarely updated
2. **Skip static-static collision tests** - Static objects don't collide with each other
3. **Efficient dynamic-vs-static queries** - O(n) not O(n^2)

Current Rust4D has `is_static` flag on bodies but also a separate `floor: Plane4D`. Recommend: unified collider system where static colliders are in a separate list/structure from dynamic bodies.

### Collision Layers and Masks

Collision layers dramatically reduce workload:

```rust
struct CollisionLayer(u32);  // Bitmask

struct Collider {
    layer: CollisionLayer,      // What layer this collider is on
    mask: CollisionLayer,       // What layers this collider collides with
    // ...
}
```

Example layers:
- Layer 0: Default
- Layer 1: Player
- Layer 2: Enemies
- Layer 3: Projectiles
- Layer 4: Static world

Configure collision matrix to skip irrelevant pairs (player projectiles don't hit player, etc.).

### Friction and Material Properties

Physics materials define surface properties:

```rust
struct PhysicsMaterial {
    friction: f32,      // 0.0 = ice, 1.0 = rubber
    restitution: f32,   // 0.0 = no bounce, 1.0 = perfect bounce
    density: f32,       // For mass calculation
}
```

**Friction combining formula** (Box2D standard):
```rust
combined_friction = (friction_a * friction_b).sqrt();
```

For solver implementation:
- Friction affects the parallel component of collision response
- Restitution affects the perpendicular component
- Clamp total impulse, not per-iteration impulse (prevents jitter)

### 4D Physics Considerations

- **GJK algorithm generalizes to 4D** - Same principle, higher dimensions
- **4D objects are tetrahedra meshes** - Like 3D uses triangles, 4D uses tetrahedra
- **Hyperplane intersection tests** - Key primitive for 4D collision
- **Rotation axes are planes in 4D, not lines** - Affects angular momentum calculations
- **Concavity challenges** - Rotating 4D objects between timesteps may create concave swept volumes

**Sources:**
- [Box2D Documentation](https://box2d.org/documentation/)
- [Jolt Physics Architecture](https://jrouwe.github.io/JoltPhysics/)
- [Video Game Physics Tutorial - Collision Detection](https://www.toptal.com/game/video-game-physics-part-ii-collision-detection-for-solid-objects)
- [Physics Broad/Narrow Phase - Newcastle University](https://research.ncl.ac.uk/game/mastersdegree/gametechnologies/physicstutorials/6accelerationstructures/Physics%20-%20Spatial%20Acceleration%20Structures.pdf)
- [4D Collision Detection Research - Wiley](https://onlinelibrary.wiley.com/doi/abs/10.1111/exsy.12668)

---

## 4. Rendering Architecture

### Render Queues

Render queues decouple rendering from game logic:

1. **Collect render requests** - Objects push what they want rendered
2. **Sort requests** - By material, depth, transparency
3. **Execute batches** - Minimize GPU state changes

Sorting strategies:
- **Front-to-back** for opaque objects (early-z rejection)
- **Back-to-front** for transparent objects
- **By material/shader** to minimize state changes

### Batching

Reduce draw calls through batching:

- **Static batching** - Combine static geometry at load time
- **Dynamic batching** - Combine small meshes at runtime
- **GPU instancing** - Draw many copies with single draw call

For Rust4D, start simple (one draw call per object) and add batching if draw calls become a bottleneck.

### Dirty Tracking

**Critical optimization** - Only rebuild what changed:

```rust
struct RenderableEntity {
    mesh: Mesh4D,
    transform_dirty: bool,
    mesh_dirty: bool,
}

impl RenderableEntity {
    fn mark_transform_dirty(&mut self) {
        self.transform_dirty = true;
    }

    fn update_if_dirty(&mut self, gpu_buffer: &mut Buffer) {
        if self.transform_dirty {
            gpu_buffer.update_transform(&self.mesh, &self.transform);
            self.transform_dirty = false;
        }
        if self.mesh_dirty {
            gpu_buffer.update_mesh(&self.mesh);
            self.mesh_dirty = false;
        }
    }
}
```

Agent 1's review noted Rust4D rebuilds ALL geometry when ANY entity moves. Dirty tracking can reduce this dramatically (one source cited 12ms -> 1.7ms improvement).

### 4D Rendering Approaches

Two main approaches for visualizing 4D:

**Projection (4D -> 3D -> 2D):**
- Apply 4D perspective projection to get 3D result
- Then standard 3D rendering to screen
- Shows entire 4D object as distorted 3D shape
- Famous "nested cube" tesseract visualization

**Slicing (Hyperplane Intersection):**
- Intersect 4D object with 3D hyperplane
- Like MRI scan of higher dimension
- More intuitive for understanding 4D structure
- Can animate slice position through 4D

Implementation note: Slicing produces cleaner geometry (tetrahedra sliced by hyperplane always produce 0, 3, or 4 vertices).

**Sources:**
- [Batch Rendering Guide - Torque2D](https://github.com/TorqueGameEngines/Torque2D/wiki/Batch-Rendering-Guide)
- [Optimization using batching - Godot](https://docs.godotengine.org/en/3.5/tutorials/performance/batching.html)
- [Tesseract Explorer](https://tsherif.github.io/tesseract-explorer/)
- [Four - 4D Renderer](https://github.com/mwalczyk/four)
- [Bartosz Ciechanowski - Tesseract](https://ciechanow.ski/tesseract/)

---

## 5. Rust-Specific Patterns (Bevy Reference)

Bevy is the leading Rust game engine using ECS. Relevant patterns:

### Component Definition
```rust
#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
struct Velocity(Vec3);
```

### Resource-Based Singleton State
```rust
#[derive(Resource)]
struct GameConfig {
    gravity: f32,
    // ...
}
```

### Query-Based System Access
```rust
fn movement_system(
    time: Res<Time>,
    mut query: Query<(&mut Position, &Velocity)>,
) {
    for (mut pos, vel) in &mut query {
        pos.0 += vel.0 * time.delta_seconds();
    }
}
```

Even if Rust4D doesn't adopt full ECS, these patterns (components as simple structs, resources for singletons, query-based iteration) are worth emulating.

**Sources:**
- [Bevy ECS Quick Start](https://bevy.org/learn/quick-start/getting-started/ecs/)
- [Unofficial Bevy Cheat Book - ECS Intro](https://bevy-cheatbook.github.io/programming/ecs-intro.html)

---

## 6. Recommendations for Rust4D

### Immediate Priorities

1. **Implement EntityHandle with generational indices**
   - Replace array index entity identification
   - Enable safe entity references and lookups

2. **Add dirty tracking for transforms**
   - Each entity tracks if transform changed
   - Only rebuild geometry for dirty entities
   - Major performance win

3. **Add friction to PhysicsMaterial**
   - Simple coefficient per material
   - Use sqrt(a*b) combining formula

4. **Unify static colliders**
   - Replace hardcoded floor with collider list
   - `is_static` flag determines update frequency

### Medium-Term Goals

5. **Entity registry with type queries**
   - "Find all entities with Collider component"
   - "Get entity by name/tag"

6. **Collision layers**
   - Bitmask-based filtering
   - Configurable collision matrix

7. **Separate transform hierarchy**
   - Optional parent reference
   - Only needed for attached objects

### Architecture Patterns to Adopt

| Pattern | Current State | Recommended |
|---------|--------------|-------------|
| Entity ID | Array index | Generational handle |
| Component system | Implicit | Explicit component types |
| Static colliders | Separate floor field | Collider list with is_static |
| Rendering | Full rebuild | Dirty tracking |
| Materials | None | PhysicsMaterial with friction |
| Entity lookup | Manual iteration | Registry with queries |

### Questions Answered

**Should we adopt full ECS or simpler component system?**
Simpler component system. Full ECS adds complexity (system scheduling, archetype storage, parallelization) that isn't needed for Rust4D's current scope. A lightweight EC pattern provides composition benefits without the overhead.

**How should static colliders differ from dynamic bodies?**
Keep them in the same data structure but:
- Flag as `is_static`
- Skip static-static collision tests
- Build static colliders into spatial acceleration structure (later optimization)
- Don't run physics integration on static bodies

**What's the right abstraction boundary between physics and rendering?**
- Physics owns position/velocity data
- Rendering queries physics for transforms
- Dirty flags bridge the gap: physics marks entities dirty when moved, renderer checks flags before rebuilding
- Entity owns both components but doesn't couple them directly

---

## Sources Summary

### ECS and Architecture
- [Entity Component System - Wikipedia](https://en.wikipedia.org/wiki/Entity_component_system)
- [ECS FAQ - SanderMertens](https://github.com/SanderMertens/ecs-faq)
- [Game Programming Patterns - Component](https://gameprogrammingpatterns.com/component.html)
- [Bevy ECS Quick Start](https://bevy.org/learn/quick-start/getting-started/ecs/)

### Scene Management
- [Handles are the Better Pointers](https://floooh.github.io/2018/06/17/handles-vs-pointers.html)
- [Scene Graphs - Wisp Wiki](https://teamwisp.github.io/research/scene_graph.html)
- [LearnOpenGL - Scene Graph](https://learnopengl.com/Guest-Articles/2021/Scene/Scene-Graph)

### Physics
- [Box2D Documentation](https://box2d.org/documentation/)
- [Jolt Physics Architecture](https://jrouwe.github.io/JoltPhysics/)
- [Video Game Physics Tutorial](https://www.toptal.com/game/video-game-physics-part-ii-collision-detection-for-solid-objects)
- [Unity Collision Layers](https://docs.unity3d.com/6000.3/Documentation/Manual/physics-optimization-cpu-collision-layers.html)

### Rendering
- [Batch Rendering Guide - Torque2D](https://github.com/TorqueGameEngines/Torque2D/wiki/Batch-Rendering-Guide)
- [Godot Batching Optimization](https://docs.godotengine.org/en/3.5/tutorials/performance/batching.html)

### 4D Specific
- [Tesseract - Bartosz Ciechanowski](https://ciechanow.ski/tesseract/)
- [Four - 4D Renderer](https://github.com/mwalczyk/four)
- [4D Collision Detection Research](https://onlinelibrary.wiley.com/doi/abs/10.1111/exsy.12668)
- [Interactive 4D Handbook](https://baileysnyder.com/interactive-4d/4d-cubes/)

### GDC and Industry
- [GDC Vault - Overwatch ECS Architecture](https://www.gdcvault.com/play/1024001/-Overwatch-Gameplay-Architecture-and)
- [GDC Vault - Destiny Core Engine](https://gdcvault.com/play/1022106/Lessons-from-the-Core-Engine)
