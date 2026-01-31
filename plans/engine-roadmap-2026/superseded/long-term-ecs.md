# ECS (Entity Component System) Migration Plan

**Status:** FUTURE/DRAFT - Not for Immediate Implementation
**Estimated Effort:** 8-12 sessions
**Priority:** P6 (Deferred)
**Created:** 2026-01-27

---

## ⚠️ Important Notice

**This plan should NOT be executed now.** The current monolithic Entity architecture is working well and appropriate for the engine's current scope. ECS migration is a major architectural change that should only be undertaken when:

1. The current system becomes a bottleneck (performance or extensibility)
2. There's a clear pain point that ECS would solve
3. The engine has matured enough to justify the migration cost

See **Trigger Conditions** section below for specific criteria.

---

## Executive Summary

This document outlines a comprehensive strategy for migrating Rust4D from its current monolithic Entity architecture to a component-based Entity Component System (ECS). While the current architecture is sound and appropriate for the engine's current stage, future growth may require the flexibility and performance characteristics that ECS provides.

**Current State:**
- Monolithic `Entity` struct with transform, shape, material, physics body, tags, name
- Single `World` container managing all entities via SlotMap
- Tight coupling between entity data and behavior
- ~800 lines across entity.rs and world.rs

**ECS State (Target):**
- Components: Transform4D, Mesh4D, Material, PhysicsBody, Name, Tags
- Systems: PhysicsSystem, RenderSystem, InputSystem, etc.
- Flexible composition - entities are just IDs with attached components
- Query-based access patterns for better cache locality

**Why Consider ECS:**
- **Extensibility:** Easy to add new component types without modifying core
- **Performance:** Better cache locality, parallelizable systems
- **Flexibility:** Entities can have arbitrary component combinations
- **Modularity:** Systems are self-contained and testable

**Why NOT Now:**
- Current Entity works well for current scope
- Migration is 8-12 sessions of effort
- No immediate pain points
- Other features (scene persistence, config) provide more value

---

## Current Architecture Analysis

### Strengths

**1. Clean and Simple**
The current Entity struct is straightforward and easy to understand:

```rust
pub struct Entity {
    pub name: Option<String>,
    pub tags: HashSet<String>,
    pub transform: Transform4D,
    pub shape: ShapeRef,
    pub material: Material,
    pub physics_body: Option<BodyKey>,
    dirty: DirtyFlags,
}
```

- All entity data in one place
- No indirection - direct field access
- Easy to reason about ownership
- Works well with current API patterns

**2. Generational Indices**
Using SlotMap for entity storage provides:
- Safe entity references (EntityKey prevents dangling pointers)
- O(1) lookup by key
- Efficient memory reuse
- Protection against ABA problem

**3. Recent Improvements**
The entity system has been progressively enhanced:
- Dirty tracking (added recently) - efficient GPU upload
- Tags for categorization
- Name-based lookup
- Physics integration via optional BodyKey

**4. Good Test Coverage**
- 58 tests in entity.rs alone
- Comprehensive tests for dirty tracking
- Name/tag query tests
- Physics sync tests

### Limitations

**1. Inflexible Composition**
Every entity has all fields whether needed or not:
- Static decorative objects don't need physics
- UI elements don't need 3D shapes
- Pure collision volumes don't need materials
- Memory waste for sparse fields

**2. Adding Features Requires Core Changes**
Adding new entity capabilities requires:
- Modifying Entity struct
- Updating serialization
- Changing constructors
- Updating all entity creation sites

Example: Adding animation, health, inventory, AI would bloat Entity struct.

**3. Limited Query Patterns**
Current queries are basic:
- By key (O(1))
- By name (O(1) via index)
- By tag (O(n) iteration)

Can't efficiently query:
- "All entities with physics but no AI"
- "All visible entities in frustum"
- "All damaged entities with health < 50%"

**4. System Coupling**
Physics, rendering, and gameplay logic are intertwined:
- Physics sync in World::update()
- Dirty tracking manually managed
- No clear system boundaries

**5. Parallelization Challenges**
Current architecture makes parallel processing difficult:
- World is a single mutable structure
- Can't safely update multiple systems in parallel
- Physics and rendering can't run concurrently

### When These Become Problems

**Scenario 1: Complex Gameplay**
Building a full game with:
- NPCs with AI, health, inventory
- Interactive objects with state machines
- Particle effects, decals
- UI elements mixed with world entities

Current Entity struct becomes bloated with dozens of Optional fields.

**Scenario 2: Performance Bottlenecks**
Thousands of entities where:
- Cache misses dominate frame time
- Iterating all entities for specific queries is slow
- Want to parallelize physics, animation, AI

**Scenario 3: Modding/Extensibility**
Users want to:
- Add custom component types
- Create new entity types without engine changes
- Script custom behaviors

Current architecture requires modifying core code.

---

## ECS Options for Rust

### 1. hecs

**Repository:** https://github.com/Ralith/hecs
**Design:** Minimal, archetype-based ECS

**Pros:**
- Minimal dependencies, small code footprint
- Excellent performance (archetype storage)
- Simple API, easy to learn
- Good documentation
- No proc macros (compile times)

**Cons:**
- Fewer features than bevy_ecs or legion
- No built-in scheduler/system runner
- Manual system orchestration
- Limited query syntax compared to bevy

**Fit for Rust4D:**
- ✅ Good for custom engines wanting full control
- ✅ Lightweight, won't add bloat
- ⚠️ Requires building system infrastructure ourselves

**Code Example:**
```rust
let mut world = hecs::World::new();

let entity = world.spawn((
    Transform4D::identity(),
    Mesh4D::tesseract(2.0),
    Material::RED,
));

// Query all entities with transform + mesh
for (id, (transform, mesh)) in world.query::<(&mut Transform4D, &Mesh4D)>().iter() {
    // Process
}
```

---

### 2. legion

**Repository:** https://github.com/amethyst/legion
**Design:** Full-featured, production-ready ECS

**Pros:**
- Mature, used in production games
- Excellent scheduler with automatic parallelization
- Rich query syntax
- Good performance
- Resources and events built-in

**Cons:**
- Heavier than hecs (more dependencies)
- More complex API surface
- Uses proc macros (compile time impact)
- Less active development (Amethyst sunset)

**Fit for Rust4D:**
- ✅ Production-ready with proven track record
- ✅ Scheduler saves implementing parallel execution
- ⚠️ More complexity than we might need
- ⚠️ Maintenance concerns (less active)

**Code Example:**
```rust
let mut world = legion::World::default();

world.push((
    Transform4D::identity(),
    Mesh4D::tesseract(2.0),
    Material::RED,
));

#[system]
fn physics_system(query: &mut Query<(&mut Transform4D, &Velocity)>) {
    for (transform, velocity) in query.iter_mut() {
        // Update
    }
}
```

---

### 3. bevy_ecs

**Repository:** https://github.com/bevyengine/bevy (bevy_ecs crate)
**Design:** Modern, feature-rich, standalone ECS (extracted from Bevy game engine)

**Pros:**
- Cutting-edge design, very actively developed
- Excellent query ergonomics (queries are systems parameters)
- Rich scheduler with stages, labels, ordering
- Change detection built-in (like our DirtyFlags)
- Resources, events, observers, hooks
- Can use standalone without rest of Bevy

**Cons:**
- Heaviest dependency of the three
- Most complex API (lots of features)
- Proc macros required (compile time)
- Tight coupling to Bevy ecosystem

**Fit for Rust4D:**
- ✅ Most feature-complete
- ✅ Change detection perfect for our render pipeline
- ✅ Active development, modern patterns
- ⚠️ Might be overkill for custom engine
- ⚠️ Steepest learning curve

**Code Example:**
```rust
use bevy_ecs::prelude::*;

#[derive(Component)]
struct Transform4D { /* ... */ }

#[derive(Component)]
struct Velocity(Vec4);

fn physics_system(mut query: Query<(&mut Transform4D, &Velocity)>) {
    for (mut transform, velocity) in &mut query {
        transform.position += velocity.0;
    }
}

fn main() {
    let mut world = World::new();
    let mut schedule = Schedule::default();
    schedule.add_systems(physics_system);

    world.spawn((Transform4D::identity(), Velocity(Vec4::ZERO)));
    schedule.run(&mut world);
}
```

---

### 4. Custom ECS

**Design:** Build our own minimal ECS tailored to Rust4D

**Pros:**
- Full control over architecture
- Zero dependencies (just std/slotmap)
- Exactly the features we need, nothing more
- Learning opportunity
- Perfect fit for our use cases

**Cons:**
- Development time (2-3 sessions just for ECS core)
- Likely less performant than battle-tested libraries
- No community support/examples
- Maintenance burden
- Reinventing the wheel

**Fit for Rust4D:**
- ✅ Educational value
- ✅ No external dependencies
- ⚠️ Significant time investment
- ⚠️ Performance likely worse than hecs/bevy_ecs

**Implementation Sketch:**
```rust
// Simplified component storage
struct ComponentStore<T> {
    components: SlotMap<EntityKey, T>,
}

struct World {
    entities: SlotMap<EntityKey, ()>,
    transforms: ComponentStore<Transform4D>,
    materials: ComponentStore<Material>,
    // etc.
}

impl World {
    fn query_transforms_materials(&self) -> impl Iterator<Item = (&Transform4D, &Material)> {
        // Manual iteration over both stores
    }
}
```

---

## Pros/Cons Comparison Matrix

| Feature | hecs | legion | bevy_ecs | Custom |
|---------|------|--------|----------|--------|
| **Performance** | Excellent | Excellent | Excellent | Good |
| **Learning Curve** | Low | Medium | High | Low |
| **Dependencies** | Minimal | Medium | Heavy | None |
| **Compile Time** | Fast | Medium | Slow | Fast |
| **Parallelization** | Manual | Automatic | Automatic | Manual |
| **Query Ergonomics** | Good | Good | Excellent | Basic |
| **Change Detection** | No | Limited | Yes | Manual |
| **Ecosystem** | Small | Medium | Large | N/A |
| **Maintenance** | Active | Stable | Very Active | Us |
| **Documentation** | Good | Good | Excellent | N/A |
| **Code Size** | Small | Medium | Large | Tiny |

---

## Recommended Approach

### Primary Recommendation: **hecs**

**Rationale:**

1. **Minimal but sufficient**: Provides core ECS functionality without bloat
2. **Performance**: Archetype-based storage is excellent
3. **Simplicity**: Easy to understand, no magic, predictable behavior
4. **Control**: We build systems/scheduling ourselves (good fit for custom engine)
5. **Dependencies**: Won't drag in half of crates.io
6. **Compile times**: No proc macros means fast rebuilds
7. **Stability**: Mature, well-tested, actively maintained

**Trade-offs We Accept:**
- No built-in scheduler (we build our own) - ~1 session of work
- No change detection (we implement if needed) - ~0.5 sessions
- Less ergonomic queries than bevy_ecs - acceptable for a small team

**Why Not Others:**

- **legion**: Good option, but less active development is concerning for long-term
- **bevy_ecs**: Too heavy for a custom engine, pulls in too much Bevy ecosystem
- **custom**: Not worth the time investment vs. using battle-tested hecs

### Fallback Option: **bevy_ecs**

If we decide we want:
- Built-in change detection (critical for rendering)
- Rich query syntax
- Mature scheduler with automatic parallelization
- Active development/community

Then bevy_ecs is worth the complexity cost. Can be used standalone.

---

## Migration Strategy

### Approach: **Incremental Hybrid Migration**

**NOT** a big-bang rewrite. Incremental migration over multiple sessions, keeping the engine working at each step.

### Phase 0: Preparation (1 session)

**Goal:** Set up ECS alongside existing code, no breaking changes

**Tasks:**
1. Add `hecs` dependency to `rust4d_core/Cargo.toml`
2. Create `crates/rust4d_core/src/ecs/` module
3. Define component types mirroring current Entity fields
4. Add World wrapper that contains both old World and hecs::World
5. Write conversion helpers (Entity -> ECS entity)

**Deliverables:**
- ECS infrastructure exists but unused
- Tests prove conversion works
- No changes to main.rs or existing code

**Risk:** Low - purely additive

---

### Phase 1: Component Definition (1 session)

**Goal:** Define all components matching current Entity capabilities

**Components to Create:**

```rust
// Transform component (replace Transform4D field)
#[derive(Component, Clone, Copy)]
pub struct Transform {
    pub position: Vec4,
    pub rotation: Rotor4,
    pub scale: f32,
}

// Mesh component (replace shape field)
#[derive(Component)]
pub struct Mesh4D {
    shape: ShapeRef,
}

// Material component (replace material field)
#[derive(Component, Clone, Copy)]
pub struct Material {
    pub base_color: [f32; 4],
}

// Physics component (replace physics_body field)
#[derive(Component)]
pub struct PhysicsBody {
    pub body_key: BodyKey,
}

// Name component (replace name field)
#[derive(Component)]
pub struct Name(pub String);

// Tag component (replace tags field)
#[derive(Component)]
pub struct Tags(pub HashSet<String>);

// Dirty tracking (replace dirty field)
#[derive(Component, Default)]
pub struct DirtyFlags(pub bitflags::BitFlags<DirtyBit>);

// Visibility/culling (NEW - was implicit)
#[derive(Component)]
pub struct Visible(pub bool);

// Optional: Entity type marker
#[derive(Component)]
pub enum EntityType {
    Static,      // Decoration, no physics
    Dynamic,     // Physics-enabled
    Trigger,     // Collision volume, no physics
    UI,          // Screen-space element
}
```

**Testing:**
- Each component has unit tests
- Serialization tests (if components need to save)
- Conversion from old Entity struct

---

### Phase 2: Hybrid World (1 session)

**Goal:** World can manage both old entities and ECS entities

```rust
pub struct World {
    // Old system (to be phased out)
    legacy_entities: SlotMap<EntityKey, Entity>,
    name_index: HashMap<String, EntityKey>,

    // New ECS system
    ecs: hecs::World,
    ecs_name_index: HashMap<String, hecs::Entity>,

    // Shared systems
    physics_world: Option<PhysicsWorld>,
}

impl World {
    // Create entity via ECS
    pub fn spawn_entity(&mut self) -> EntityBuilder {
        EntityBuilder::new(&mut self.ecs)
    }

    // Legacy method (deprecated)
    pub fn add_entity(&mut self, entity: Entity) -> EntityKey {
        // Old path
    }

    // Query ECS entities
    pub fn query<Q: Query>(&self) -> QueryBorrow<Q> {
        self.ecs.query::<Q>()
    }
}
```

**Deliverables:**
- Can create entities via either old or new API
- Both systems coexist peacefully
- Physics works with both

**Risk:** Medium - introduces dual code paths

---

### Phase 3: System Architecture (2 sessions)

**Goal:** Define and implement core systems

**Systems to Create:**

```rust
// Physics integration system
pub fn physics_sync_system(
    world: &hecs::World,
    physics: &PhysicsWorld,
) {
    for (id, (mut transform, body)) in world.query_mut::<(&mut Transform, &PhysicsBody)>() {
        if let Some(physics_body) = physics.get_body(body.body_key) {
            if transform.position != physics_body.position {
                transform.position = physics_body.position;
                // Mark dirty (if using change detection)
            }
        }
    }
}

// Dirty tracking system (mark entities needing GPU update)
pub fn dirty_marking_system(
    world: &hecs::World,
    changed_transforms: &ChangedTransforms, // From change detection
) {
    for id in changed_transforms.iter() {
        if let Ok(mut dirty) = world.get_mut::<DirtyFlags>(id) {
            dirty.0.insert(DirtyBit::Transform);
        }
    }
}

// Rendering prep system
pub fn render_prep_system(
    world: &hecs::World,
) -> RenderableGeometry {
    let mut geometry = RenderableGeometry::new();

    for (id, (transform, mesh, material)) in world.query::<(&Transform, &Mesh4D, &Material)>().iter() {
        // Build GPU buffers
        geometry.add_mesh(transform, mesh.shape(), material);
    }

    geometry
}

// Tag query helper
pub fn query_by_tag(world: &hecs::World, tag: &str) -> impl Iterator<Item = hecs::Entity> {
    world.query::<&Tags>()
        .iter()
        .filter_map(move |(id, tags)| {
            if tags.0.contains(tag) { Some(id) } else { None }
        })
}
```

**System Scheduler:**

```rust
pub struct SystemScheduler {
    systems: Vec<Box<dyn System>>,
}

impl SystemScheduler {
    pub fn run(&mut self, world: &mut World, dt: f32) {
        // Run systems in order (later: parallelize)
        for system in &mut self.systems {
            system.run(world, dt);
        }
    }
}
```

**Deliverables:**
- Core systems implemented
- Systems can run via scheduler
- Tests for each system

**Risk:** Medium - requires careful system design

---

### Phase 4: Migration of Entity Creation (1 session)

**Goal:** Update SceneBuilder to create ECS entities

**Before:**
```rust
builder
    .add_entity(
        Entity::with_transform(
            ShapeRef::shared(Tesseract4D::new(2.0)),
            transform,
            Material::RED,
        )
        .with_name("player")
        .with_tag("dynamic")
    );
```

**After:**
```rust
builder
    .spawn_entity()
    .with(Transform::from_position(Vec4::new(0.0, 5.0, 0.0, 0.0)))
    .with(Mesh4D::tesseract(2.0))
    .with(Material::RED)
    .with(Name("player".into()))
    .with(Tags::from(["dynamic"]))
    .with(EntityType::Dynamic);
```

**Tasks:**
1. Update SceneBuilder API
2. Migrate test scene creation
3. Update main.rs scene setup
4. Remove legacy entity creation

**Deliverables:**
- All entities created via ECS
- SceneBuilder API simplified
- Legacy add_entity() deprecated

**Risk:** Medium - touches many entity creation sites

---

### Phase 5: Update Systems to Use ECS (2 sessions)

**Goal:** Physics, rendering, input all query ECS

**Updates:**

**main.rs RedrawRequested:**
```rust
// OLD:
let geometry = app.build_geometry(&world);

// NEW:
let geometry = render_prep_system(&world.ecs);
```

**Physics Sync:**
```rust
// OLD:
for (key, entity) in &mut world.entities {
    if let Some(body_key) = entity.physics_body {
        // Sync transform
    }
}

// NEW:
physics_sync_system(&mut world.ecs, &physics);
```

**Deliverables:**
- Physics uses ECS queries
- Rendering uses ECS queries
- Input/camera uses ECS queries
- No more legacy entity iteration

**Risk:** High - core update loop changes

---

### Phase 6: Serialization Update (1 session)

**Goal:** Scene save/load works with ECS

**Challenges:**
- hecs doesn't have built-in serialization
- Need custom serialization for components
- Scene format changes

**Approach:**
```rust
// Serialize scene by querying all components
pub fn save_scene(world: &hecs::World, path: &Path) -> Result<()> {
    let mut scene = Scene::new();

    for (id, (transform, mesh, material)) in world.query::<(&Transform, &Mesh4D, &Material)>().iter() {
        scene.entities.push(SerializedEntity {
            transform: *transform,
            mesh: mesh.to_template(),
            material: *material,
            name: world.get::<Name>(id).ok().map(|n| n.0.clone()),
            tags: world.get::<Tags>(id).ok().map(|t| t.0.clone()),
        });
    }

    save_ron(path, &scene)
}
```

**Deliverables:**
- Can save ECS world to RON
- Can load RON into ECS world
- Scene format documented
- Migration tool for old scenes

**Risk:** Medium - serialization complexity

---

### Phase 7: Remove Legacy System (1 session)

**Goal:** Delete old Entity/World code

**Tasks:**
1. Remove `SlotMap<EntityKey, Entity>` from World
2. Remove Entity struct definition
3. Remove legacy query methods
4. Update tests to use ECS
5. Clean up deprecated code

**Deliverables:**
- Clean codebase with only ECS
- All tests passing
- Documentation updated
- Migration guide for users

**Risk:** Low - everything already migrated

---

### Phase 8: Optimization (1-2 sessions)

**Goal:** Tune ECS performance

**Tasks:**
1. Profile component access patterns
2. Add archetype hints for common entity types
3. Implement parallel system execution
4. Optimize query patterns
5. Add benchmarks

**Deliverables:**
- Performance >= old system
- Parallel systems where safe
- Benchmarks prove improvements

**Risk:** Low - optimization only

---

## Component Design

### Core Components (Always Present)

**Transform** - Spatial position
```rust
#[derive(Component, Copy, Clone)]
pub struct Transform {
    pub position: Vec4,
    pub rotation: Rotor4,
    pub scale: f32,
}
```

**Every entity needs a transform** (even UI uses it for position).

---

### Rendering Components

**Mesh4D** - Geometric shape
```rust
#[derive(Component)]
pub struct Mesh4D {
    shape: ShapeRef,
}
```

**Material** - Visual appearance
```rust
#[derive(Component, Copy, Clone)]
pub struct Material {
    pub base_color: [f32; 4],
    // Future: roughness, metallic, emission
}
```

**Visible** - Rendering toggle
```rust
#[derive(Component)]
pub struct Visible(pub bool);
```

---

### Physics Components

**PhysicsBody** - Physics integration
```rust
#[derive(Component)]
pub struct PhysicsBody {
    pub body_key: BodyKey,
}
```

**Collider** - Collision shape (optional, if different from mesh)
```rust
#[derive(Component)]
pub struct Collider {
    pub shape: Arc<dyn ConvexShape4D>,
    pub layer: CollisionLayer,
}
```

---

### Metadata Components

**Name** - Unique identifier
```rust
#[derive(Component)]
pub struct Name(pub String);
```

**Tags** - Categorization
```rust
#[derive(Component)]
pub struct Tags(pub HashSet<String>);
```

**EntityType** - Type marker (optional optimization)
```rust
#[derive(Component)]
pub enum EntityType {
    Static,
    Dynamic,
    Trigger,
    UI,
}
```

---

### Future Components (Not Initial Implementation)

**Lifetime** - Auto-despawn after time
```rust
#[derive(Component)]
pub struct Lifetime {
    pub remaining: f32,
}
```

**Health** - Damage/destruction
```rust
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}
```

**Inventory** - Item storage
```rust
#[derive(Component)]
pub struct Inventory {
    pub items: Vec<hecs::Entity>,
    pub capacity: usize,
}
```

**AI** - Behavior state
```rust
#[derive(Component)]
pub struct AI {
    pub state: AIState,
    pub target: Option<hecs::Entity>,
}
```

---

## System Design

### System Categories

**1. Input Systems** (Read input, write to components)
- Runs first each frame
- Examples: CameraInputSystem, PlayerInputSystem

**2. Logic Systems** (Game logic, AI, etc.)
- Runs after input
- Examples: AISystem, HealthSystem, LifetimeSystem

**3. Physics Systems** (Physics simulation)
- Runs after logic
- Examples: PhysicsStepSystem, CollisionSystem

**4. Sync Systems** (Keep components in sync)
- Runs after physics
- Examples: PhysicsSyncSystem, TransformHierarchySystem

**5. Rendering Systems** (Prepare for rendering)
- Runs last
- Examples: FrustumCullingSystem, RenderPrepSystem

---

### Core System Implementations

**PhysicsSyncSystem** - Sync transforms from physics bodies

```rust
pub fn physics_sync_system(world: &mut hecs::World, physics: &PhysicsWorld) {
    for (id, (mut transform, body)) in world.query_mut::<(&mut Transform, &PhysicsBody)>() {
        if let Some(physics_body) = physics.get_body(body.body_key) {
            if transform.position != physics_body.position {
                transform.position = physics_body.position;
            }
        }
    }
}
```

---

**RenderPrepSystem** - Build GPU geometry

```rust
pub fn render_prep_system(world: &hecs::World) -> RenderableGeometry {
    let mut geometry = RenderableGeometry::new();

    for (id, (transform, mesh, material)) in
        world.query::<(&Transform, &Mesh4D, &Material)>()
             .with::<Visible>() // Only visible entities
             .iter()
    {
        geometry.add_mesh(transform, mesh.shape(), material);
    }

    geometry
}
```

---

**LifetimeSystem** - Auto-despawn timed entities (future)

```rust
pub fn lifetime_system(world: &mut hecs::World, dt: f32) {
    let mut to_despawn = Vec::new();

    for (id, lifetime) in world.query_mut::<&mut Lifetime>() {
        lifetime.remaining -= dt;
        if lifetime.remaining <= 0.0 {
            to_despawn.push(id);
        }
    }

    for id in to_despawn {
        world.despawn(id).ok();
    }
}
```

---

### System Scheduling

**Initial: Sequential Execution**

```rust
pub fn update(world: &mut World, dt: f32) {
    // Phase 1: Input
    camera_input_system(&mut world.ecs, &world.input_state, dt);

    // Phase 2: Physics
    world.physics_world.as_mut().map(|p| p.step(dt));
    physics_sync_system(&mut world.ecs, world.physics_world.as_ref().unwrap());

    // Phase 3: Rendering prep
    let geometry = render_prep_system(&world.ecs);
    world.cached_geometry = Some(geometry);
}
```

**Future: Parallel Execution**

```rust
pub fn update(world: &mut World, dt: f32) {
    // Systems that can run in parallel
    rayon::scope(|s| {
        s.spawn(|_| lifetime_system(&mut world.ecs, dt));
        s.spawn(|_| ai_system(&mut world.ecs, dt));
        // More parallel systems...
    });

    // Systems that must run sequentially
    physics_step_system(&mut world.physics_world, dt);
    physics_sync_system(&mut world.ecs, &world.physics_world);
}
```

---

## Phased Implementation Plan

### Timeline Overview

```
Phase 0: Preparation         ████ (1 session)
Phase 1: Components          ████ (1 session)
Phase 2: Hybrid World        ████ (1 session)
Phase 3: Systems             ████████ (2 sessions)
Phase 4: Migration           ████ (1 session)
Phase 5: Update Systems      ████████ (2 sessions)
Phase 6: Serialization       ████ (1 session)
Phase 7: Cleanup             ████ (1 session)
Phase 8: Optimization        ████████ (1-2 sessions)

Total: 10-11 sessions (conservative)
```

### Dependencies

```
Phase 0 (Foundation)
  ↓
Phase 1 (Components) ──┐
  ↓                    │
Phase 2 (Hybrid)       │
  ↓                    │
Phase 3 (Systems) ─────┤ Can parallelize
  ↓                    │
Phase 4 (Migration) ───┘
  ↓
Phase 5 (Update Systems)
  ↓
Phase 6 (Serialization)
  ↓
Phase 7 (Cleanup)
  ↓
Phase 8 (Optimization)
```

### Session-by-Session Breakdown

**Session 1: Preparation**
- [ ] Add hecs dependency
- [ ] Create ecs module structure
- [ ] Define component trait/types
- [ ] Write conversion helpers
- [ ] Tests: conversion Entity -> ECS components

**Session 2: Component Definition**
- [ ] Implement all core components
- [ ] Add serialization support
- [ ] Component builder API
- [ ] Tests: component creation, serialization

**Session 3: Hybrid World**
- [ ] Dual storage (legacy + ECS)
- [ ] Unified API for both paths
- [ ] Physics integration for both
- [ ] Tests: both systems work simultaneously

**Session 4-5: System Architecture**
- [ ] System trait definition
- [ ] Implement core systems (physics, render prep)
- [ ] System scheduler
- [ ] Tests: system execution, ordering

**Session 6: Entity Creation Migration**
- [ ] Update SceneBuilder for ECS
- [ ] Migrate test scenes
- [ ] Update main.rs scene
- [ ] Tests: scene creation via ECS

**Session 7-8: Update Core Systems**
- [ ] Physics uses ECS queries
- [ ] Rendering uses ECS queries
- [ ] Input uses ECS queries
- [ ] Update main loop
- [ ] Tests: full integration

**Session 9: Serialization**
- [ ] Scene save with ECS
- [ ] Scene load into ECS
- [ ] Migration tool for old scenes
- [ ] Tests: save/load roundtrip

**Session 10: Cleanup**
- [ ] Remove legacy Entity struct
- [ ] Remove old World fields
- [ ] Update documentation
- [ ] Tests: all passing with ECS only

**Session 11: Optimization (optional)**
- [ ] Profile and optimize
- [ ] Parallel systems
- [ ] Benchmarks
- [ ] Tests: performance regression

---

## Risk Assessment

### High Risk Areas

**1. Physics Integration (Phase 5)**
- **Risk:** Breaking physics sync, simulation bugs
- **Mitigation:** Extensive testing, keep old code until verified
- **Fallback:** Revert to hybrid mode if issues found

**2. Serialization Changes (Phase 6)**
- **Risk:** Breaking scene loading, data loss
- **Mitigation:** Write migration tool, version scenes
- **Fallback:** Support both old and new formats

**3. Performance Regression (Phase 8)**
- **Risk:** ECS slower than monolithic Entity
- **Mitigation:** Benchmark before/after, profile hotspots
- **Fallback:** Optimize or revert if unacceptable

### Medium Risk Areas

**1. API Changes (Phase 4)**
- **Risk:** Breaking user code, SceneBuilder API churn
- **Mitigation:** Deprecation warnings, migration guide
- **Fallback:** Support both APIs temporarily

**2. System Ordering (Phase 3-5)**
- **Risk:** Wrong system order causes bugs (transform before physics, etc.)
- **Mitigation:** Document dependencies, integration tests
- **Fallback:** Sequential execution (no parallelism)

### Low Risk Areas

**1. Component Definition (Phase 1)**
- **Risk:** Minimal - just data structures
- **Mitigation:** Unit tests, type safety

**2. Hybrid World (Phase 2)**
- **Risk:** Low - both systems isolated
- **Mitigation:** Don't remove old system until migration complete

---

## Success Criteria

### Functional Requirements

- [ ] All entity types can be represented via components
- [ ] Scene loading/saving works identically to old system
- [ ] Physics integration works correctly
- [ ] Rendering produces identical output
- [ ] All tests passing (260+ existing tests)
- [ ] No memory leaks or resource leaks

### Non-Functional Requirements

- [ ] Performance >= current system (no regression)
- [ ] Memory usage <= current system
- [ ] Compile times reasonable (<10% increase)
- [ ] Code size manageable (ECS worth the complexity)
- [ ] Documentation complete (migration guide, component guide)

### Developer Experience

- [ ] Entity creation is easier/cleaner than before
- [ ] Queries are more ergonomic
- [ ] Adding new components is straightforward
- [ ] Systems are testable in isolation
- [ ] Clear examples of common patterns

---

## Trigger Conditions (When to Start This Work)

**DO NOT start ECS migration until at least TWO of these are true:**

### Performance Triggers

1. **Entity count bottleneck**: >10,000 entities causing frame drops
2. **Query performance**: Iteration over entities >5ms per frame
3. **Memory pressure**: Entity struct waste >10MB
4. **Cache misses**: Profiler shows poor locality in entity access

### Extensibility Triggers

1. **Component bloat**: Entity struct has >15 fields, mostly Optional
2. **Frequent Entity changes**: Adding fields weekly
3. **Modding requirement**: Users need custom components
4. **Plugin system**: Third-party code needs to extend entities

### Feature Triggers

1. **Complex gameplay**: Need 10+ component types
2. **Parallel systems**: Physics + AI + animation need to run concurrently
3. **Query complexity**: Need advanced queries (tag combinations, exclusions)
4. **Entity types**: 5+ distinct entity archetypes (player, NPC, item, projectile, etc.)

### Project Maturity Triggers

1. **Production use**: Building real game, not prototyping
2. **Team size**: >2 developers working on entity code
3. **Stability**: Core engine stable, minimal breaking changes
4. **Time available**: Can dedicate 10-11 sessions without urgent features

---

## Alternatives to Full ECS Migration

**If triggers are met but full ECS seems too costly, consider:**

### Option 1: Component Storage (Partial ECS)

Keep Entity struct but store components separately:

```rust
pub struct Entity {
    pub transform: Transform4D,
    // Core fields stay
}

// Optional components in separate stores
pub struct World {
    entities: SlotMap<EntityKey, Entity>,
    health: ComponentStore<EntityKey, Health>,
    inventory: ComponentStore<EntityKey, Inventory>,
    ai: ComponentStore<EntityKey, AI>,
}
```

**Effort:** 2-3 sessions
**Benefit:** Extensibility without full ECS migration

---

### Option 2: Archetype Specialization

Create specialized entity types:

```rust
pub enum EntityVariant {
    Static(StaticEntity),
    Dynamic(DynamicEntity),
    Character(CharacterEntity),
    UI(UIElement),
}

pub struct StaticEntity {
    pub transform: Transform4D,
    pub mesh: Mesh4D,
    pub material: Material,
    // No physics, AI, etc.
}
```

**Effort:** 3-4 sessions
**Benefit:** Memory savings, type safety

---

### Option 3: Component Traits

Add component storage without ECS:

```rust
pub trait Component: 'static {}

pub struct World {
    entities: SlotMap<EntityKey, Entity>,
    components: HashMap<TypeId, Box<dyn ComponentStorage>>,
}

impl World {
    pub fn add_component<C: Component>(&mut self, entity: EntityKey, component: C) {
        // Store in type-erased storage
    }
}
```

**Effort:** 2 sessions
**Benefit:** Flexibility without architectural overhaul

---

## Conclusion

ECS migration is a **major architectural change** that should be undertaken thoughtfully and only when clear pain points emerge. The current monolithic Entity architecture is:

- ✅ Working well for current scope
- ✅ Simple and easy to understand
- ✅ Adequately performant
- ✅ Well-tested and stable

**ECS provides value when:**
- Entity types proliferate
- Performance becomes critical
- Extensibility is needed
- Parallel systems are required

**Recommendation:**
1. **Monitor trigger conditions** as engine evolves
2. **Defer migration** until 2+ triggers are met
3. **Start with partial solutions** if only 1 trigger is met
4. **Use this plan as roadmap** when time comes

**Expected Timeline:**
If triggers are met, ECS migration is 10-11 sessions of focused work. This is only justified when the benefits outweigh the cost of existing architecture plus migration effort.

**Next Review:**
Revisit this plan when:
- Near-term roadmap (Phases 1-5) is complete (~20 sessions)
- Building a real game/demo with complex entities
- Performance profiling shows entity bottlenecks
- Users requesting extensibility features

---

**Plan Status:** DRAFT - Not for immediate implementation
**Prepared by:** Claude Code (Architecture Planning)
**Last Updated:** 2026-01-27
