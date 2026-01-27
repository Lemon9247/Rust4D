# Rust Game Engine Patterns Research Report

**Agent:** Rust Game Engine Patterns Agent
**Date:** 2026-01-27
**Task:** Research Rust game engine and physics library architectural patterns

---

## Executive Summary

This report analyzes patterns from major Rust game engines (Bevy, Fyrox) and physics libraries (Rapier) to identify idiomatic approaches that Rust4D could adopt. The key findings suggest that while full ECS may be overkill for Rust4D's scope, several specific patterns (generational handles, separated RigidBody/Collider, collision groups, physics materials) would significantly improve the architecture.

---

## 1. Entity-Component-System (ECS) Patterns

### 1.1 Bevy ECS Overview

Bevy's ECS separates data and behavior into three core concepts:

- **Component**: Plain Rust structs with `#[derive(Component)]` - single piece of functionality
- **Entity**: A unique ID with a collection of components
- **System**: Functions that query and operate on components

```rust
// Bevy component example
#[derive(Component)]
struct Position(Vec3);

#[derive(Component)]
struct Velocity(Vec3);

// System that operates on components
fn movement_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in &mut query {
        pos.0 += vel.0;
    }
}
```

**Key insight**: ECS shines when you have thousands of entities with varying component combinations. For Rust4D's current scope (likely dozens to hundreds of entities), a simpler approach may suffice.

### 1.2 Lightweight ECS Alternatives

**hecs** - Minimal, no-framework ECS:
```rust
// hecs example
let mut world = World::new();
let entity = world.spawn((Position(0.0, 0.0), Velocity(1.0, 0.0)));

// Query all entities with Position and Velocity
for (id, (pos, vel)) in world.query_mut::<(&mut Position, &Velocity)>() {
    pos.0 += vel.0;
}
```

**legion** - High-performance with parallel scheduling:
- Archetype-based storage for cache efficiency
- Best with homogeneous entity types
- Filtering happens at archetype level, not entity level

**Recommendation for Rust4D**: Consider hecs if moving toward ECS - it's minimal and doesn't impose framework constraints. However, the current Entity/World design may be sufficient with targeted improvements.

### 1.3 Fyrox's Non-ECS Approach

Fyrox explicitly **does not** use ECS. Instead, it uses:
- Generational arenas (pools) for memory management
- Handles for referencing objects
- Scene graph with nodes

```rust
// Fyrox-style handle pattern
struct Handle<T> {
    index: u32,
    generation: u32,
    _marker: PhantomData<T>,
}
```

This is notable because Fyrox is a successful, production-ready engine that proves ECS isn't mandatory for game engines.

---

## 2. Generational Indices / Handle Pattern

### 2.1 The Problem

Rust4D currently uses `BodyHandle(pub(crate) usize)` - a simple index. This has the **ABA problem**:

```rust
// Current Rust4D pattern - vulnerable to ABA
let handle = world.add_body(body_a);  // handle = 0
world.remove_body(handle);            // Slot 0 freed
let new_handle = world.add_body(body_b);  // new_handle = 0 (reuses slot)
// Old handle now incorrectly references body_b!
```

### 2.2 The Solution: Generational Indices

```rust
// Generational handle (from slotmap/generational-arena patterns)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyHandle {
    index: u32,
    generation: u32,
}

pub struct PhysicsWorld {
    bodies: Vec<Option<RigidBody4D>>,
    generations: Vec<u32>,
}

impl PhysicsWorld {
    pub fn get_body(&self, handle: BodyHandle) -> Option<&RigidBody4D> {
        if handle.index as usize >= self.generations.len() {
            return None;
        }
        if self.generations[handle.index as usize] != handle.generation {
            return None;  // Stale handle
        }
        self.bodies[handle.index as usize].as_ref()
    }

    pub fn remove_body(&mut self, handle: BodyHandle) -> Option<RigidBody4D> {
        // ... validate handle ...
        self.generations[handle.index as usize] += 1;  // Invalidate old handles
        self.bodies[handle.index as usize].take()
    }
}
```

### 2.3 Recommended Crates

- **slotmap** - Most mature, provides `SlotMap`, `HopSlotMap`, `DenseSlotMap`
- **generational-arena** - Simpler API, good for basic cases
- **thunderdome** - Newer alternative with good ergonomics

**Recommendation**: Use `slotmap` crate. It's battle-tested and provides:
- O(1) insert, remove, access
- Safe iteration
- Secondary maps for associating extra data with entities

```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct BodyKey; }

pub struct PhysicsWorld {
    bodies: SlotMap<BodyKey, RigidBody4D>,
}
```

---

## 3. Rapier Physics Patterns

### 3.1 RigidBody vs Collider Separation

Rapier's fundamental architecture separates dynamics from geometry:

```rust
// Rapier pattern: RigidBody handles motion, Collider handles shape
let rigid_body = RigidBodyBuilder::dynamic()
    .translation(vector![0.0, 10.0, 0.0])
    .build();

let collider = ColliderBuilder::ball(0.5)
    .restitution(0.7)
    .friction(0.3)
    .build();

let body_handle = rigid_body_set.insert(rigid_body);
collider_set.insert_with_parent(collider, body_handle, &mut rigid_body_set);
```

**Key benefits**:
- Multiple colliders per body (compound shapes)
- Colliders without bodies (static geometry, sensors)
- Independent material properties per collider

**Rust4D current state**: `RigidBody4D` contains `collider: Collider` directly. This works but limits flexibility.

### 3.2 Static Bodies

Rapier uses body types to distinguish behavior:

```rust
// Rapier body types
RigidBodyBuilder::fixed()      // Never moves (walls, floors)
RigidBodyBuilder::dynamic()    // Full physics simulation
RigidBodyBuilder::kinematic_position_based()  // User-controlled movement
RigidBodyBuilder::kinematic_velocity_based()  // User-controlled velocity
```

**Rust4D comparison**: Currently uses `is_static: bool` flag. This is simpler but covers the main cases.

### 3.3 Friction and Restitution

Rapier stores friction/restitution on **colliders**, not bodies:

```rust
let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0)
    .friction(0.5)           // 0.0 = ice, 1.0 = rubber
    .restitution(0.3)        // 0.0 = no bounce, 1.0 = perfect bounce
    .friction_combine_rule(CoefficientCombineRule::Average)
    .restitution_combine_rule(CoefficientCombineRule::Max)
    .build();
```

**Combine rules** determine how two materials interact:
- `Average`: (a + b) / 2
- `Min`: min(a, b)
- `Max`: max(a, b)
- `Multiply`: a * b

**Recommendation for Rust4D**: Add a `PhysicsMaterial` struct:

```rust
#[derive(Clone, Copy, Debug)]
pub struct PhysicsMaterial {
    pub friction: f32,
    pub restitution: f32,
}

impl PhysicsMaterial {
    pub const ICE: Self = Self { friction: 0.05, restitution: 0.1 };
    pub const RUBBER: Self = Self { friction: 0.9, restitution: 0.8 };
    pub const METAL: Self = Self { friction: 0.3, restitution: 0.3 };

    pub fn combine(&self, other: &Self) -> Self {
        Self {
            friction: (self.friction + other.friction) / 2.0,
            restitution: self.restitution.max(other.restitution),
        }
    }
}
```

### 3.4 Collision Groups

Rapier uses bitmask-based collision filtering:

```rust
// Rapier collision groups
let player_collider = ColliderBuilder::capsule_y(0.5, 0.3)
    .collision_groups(InteractionGroups::new(
        Group::GROUP_1,  // Player is in group 1
        Group::GROUP_2 | Group::GROUP_3,  // Can collide with groups 2 and 3
    ))
    .build();
```

**Rust4D application**: Useful for:
- Player-enemy collision (damage)
- Player-pickup collision (collection)
- Enemy-enemy non-collision (prevent stacking)
- Projectile selective collision

```rust
// Suggested Rust4D pattern
bitflags::bitflags! {
    pub struct CollisionGroup: u32 {
        const PLAYER = 1 << 0;
        const ENEMY = 1 << 1;
        const PROJECTILE = 1 << 2;
        const STATIC = 1 << 3;
        const PICKUP = 1 << 4;
        const ALL = 0xFFFFFFFF;
    }
}

pub struct CollisionFilter {
    pub membership: CollisionGroup,  // What groups this belongs to
    pub mask: CollisionGroup,        // What groups this can collide with
}
```

---

## 4. Collision Events and Callbacks

### 4.1 Rapier Event System

Rapier provides event-based collision notification:

```rust
// Rapier collision events
pub enum CollisionEvent {
    Started(ColliderHandle, ColliderHandle, CollisionEventFlags),
    Stopped(ColliderHandle, ColliderHandle, CollisionEventFlags),
}

// Reading events in Bevy
fn handle_collisions(mut events: EventReader<CollisionEvent>) {
    for event in events.read() {
        match event {
            CollisionEvent::Started(a, b, _) => {
                println!("Collision started between {:?} and {:?}", a, b);
            }
            CollisionEvent::Stopped(a, b, _) => {
                println!("Collision ended between {:?} and {:?}", a, b);
            }
        }
    }
}
```

**Key pattern**: Events are collected during physics step, then processed in game logic systems.

### 4.2 Recommended Pattern for Rust4D

```rust
#[derive(Clone, Debug)]
pub struct CollisionEvent {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub contact_point: Vec4,
    pub normal: Vec4,
    pub penetration: f32,
}

pub struct PhysicsWorld {
    bodies: SlotMap<BodyKey, RigidBody4D>,
    collision_events: Vec<CollisionEvent>,  // Filled during step()
}

impl PhysicsWorld {
    pub fn step(&mut self, dt: f32) {
        self.collision_events.clear();
        // ... physics simulation ...
        // When collision detected:
        self.collision_events.push(CollisionEvent { /* ... */ });
    }

    pub fn drain_collision_events(&mut self) -> impl Iterator<Item = CollisionEvent> + '_ {
        self.collision_events.drain(..)
    }
}
```

---

## 5. Bevy Bundle Pattern

### 5.1 What Are Bundles?

Bundles group related components for convenient spawning:

```rust
#[derive(Bundle)]
struct PlayerBundle {
    position: Position,
    velocity: Velocity,
    health: Health,
    sprite: SpriteBundle,
    marker: Player,
}

// Spawn with bundle
commands.spawn(PlayerBundle {
    position: Position(Vec3::ZERO),
    velocity: Velocity(Vec3::ZERO),
    health: Health(100),
    sprite: SpriteBundle::default(),
    marker: Player,
});
```

### 5.2 Application to Rust4D

Even without full ECS, the bundle concept applies to entity creation:

```rust
// Rust4D entity builder pattern (current style, enhanced)
pub struct EntityBuilder {
    transform: Transform4D,
    shape: Option<ShapeRef>,
    material: Material,
    physics: Option<PhysicsProperties>,
}

pub struct PhysicsProperties {
    pub body_type: BodyType,
    pub physics_material: PhysicsMaterial,
    pub collision_filter: CollisionFilter,
}

pub enum BodyType {
    Static,
    Dynamic { mass: f32, gravity: bool },
    Kinematic,
}

impl EntityBuilder {
    pub fn new() -> Self { /* ... */ }

    pub fn with_shape(mut self, shape: ShapeRef) -> Self { /* ... */ }
    pub fn with_transform(mut self, transform: Transform4D) -> Self { /* ... */ }
    pub fn with_material(mut self, material: Material) -> Self { /* ... */ }
    pub fn with_physics(mut self, props: PhysicsProperties) -> Self { /* ... */ }

    pub fn spawn(self, world: &mut World) -> EntityHandle { /* ... */ }
}
```

---

## 6. Scene Management Patterns

### 6.1 Fyrox Scene Graph

Fyrox uses a hierarchical scene graph with nodes:

```rust
// Fyrox scene structure (conceptual)
scene.graph.add_node(Node::Camera(camera));
scene.graph.add_node(Node::Mesh(mesh));
scene.graph.add_node(Node::Light(light));

// Nodes can have parent-child relationships
let child_handle = scene.graph.link_nodes(child, parent);
```

### 6.2 Bevy's Flat Entity Model

Bevy uses a flat entity model with optional parent-child relationships:

```rust
// Parent-child in Bevy
commands.spawn(Parent).with_children(|parent| {
    parent.spawn(Child);
});
```

### 6.3 Recommendation for Rust4D

Start with a **flat entity registry** (what you have now), but add:

1. **Named entities** for easy lookup:
```rust
pub struct World {
    entities: SlotMap<EntityKey, Entity>,
    names: HashMap<String, EntityKey>,
}

impl World {
    pub fn spawn_named(&mut self, name: &str, entity: Entity) -> EntityKey {
        let key = self.entities.insert(entity);
        self.names.insert(name.to_string(), key);
        key
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Entity> {
        self.names.get(name).and_then(|k| self.entities.get(*k))
    }
}
```

2. **Optional parent-child** for hierarchical transforms (later enhancement):
```rust
pub struct Entity {
    pub transform: Transform4D,
    pub parent: Option<EntityKey>,
    pub children: Vec<EntityKey>,
    // ...
}
```

---

## 7. Concrete Recommendations for Rust4D

### 7.1 Immediate Improvements (Low Effort, High Value)

1. **Add generational handles** using `slotmap`:
   ```toml
   [dependencies]
   slotmap = "1.0"
   ```

2. **Add PhysicsMaterial** to colliders:
   ```rust
   pub struct PhysicsMaterial {
       pub friction: f32,
       pub restitution: f32,
   }
   ```

3. **Collect collision events** during physics step for game logic

### 7.2 Medium-Term Improvements

1. **Separate Collider from RigidBody** - allow collider-only entities for static geometry

2. **Add collision groups/layers** for selective collision detection

3. **Named entity registry** for easy lookup and debugging

### 7.3 Architecture Decision: ECS vs Current

**Stay with current architecture** because:
- Rust4D is a 4D engine - the unique physics/rendering is the complexity, not entity management
- Current entity count is likely manageable without ECS
- ECS adds learning curve and boilerplate

**Consider ECS (hecs) if**:
- Entity count grows to thousands
- Need complex component queries (e.g., "all entities with Health but not Invincible")
- Multiple independent systems need to process entities in parallel

---

## 8. Code Examples Summary

### Generational Handle Implementation

```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! {
    pub struct EntityKey;
    pub struct BodyKey;
}

pub struct World {
    entities: SlotMap<EntityKey, Entity>,
}

pub struct PhysicsWorld {
    bodies: SlotMap<BodyKey, RigidBody4D>,
    collision_events: Vec<CollisionEvent>,
}
```

### Physics Material System

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicsMaterial {
    pub friction: f32,     // 0.0 to 1.0
    pub restitution: f32,  // 0.0 to 1.0
}

impl PhysicsMaterial {
    pub fn combine(&self, other: &Self) -> Self {
        Self {
            friction: (self.friction * other.friction).sqrt(),  // Geometric mean
            restitution: self.restitution.max(other.restitution),
        }
    }
}

pub struct RigidBody4D {
    pub position: Vec4,
    pub velocity: Vec4,
    pub mass: f32,
    pub material: PhysicsMaterial,  // Instead of just restitution
    pub collider: Collider,
    pub is_static: bool,
}
```

### Collision Filter System

```rust
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct CollisionLayer: u32 {
        const DEFAULT = 1 << 0;
        const PLAYER = 1 << 1;
        const ENEMY = 1 << 2;
        const STATIC = 1 << 3;
        const TRIGGER = 1 << 4;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CollisionFilter {
    pub layer: CollisionLayer,
    pub mask: CollisionLayer,
}

impl CollisionFilter {
    pub fn collides_with(&self, other: &Self) -> bool {
        self.mask.contains(other.layer) && other.mask.contains(self.layer)
    }
}
```

---

## Sources

- [Bevy ECS Documentation](https://bevy.org/learn/quick-start/getting-started/ecs/)
- [Bevy Engine GitHub](https://github.com/bevyengine/bevy)
- [Rapier Rigid Bodies](https://rapier.rs/docs/user_guides/rust/rigid_bodies/)
- [Rapier Colliders](https://rapier.rs/docs/user_guides/rust/colliders/)
- [Rapier Collision Groups](https://rapier.rs/docs/user_guides/rust/collider_collision_groups/)
- [Rapier Advanced Collision Detection](https://rapier.rs/docs/user_guides/rust/advanced_collision_detection/)
- [bevy_rapier Physics Events](https://deepwiki.com/dimforge/bevy_rapier/7-physics-events)
- [Fyrox Architecture](https://github.com/FyroxEngine/Fyrox/blob/master/ARCHITECTURE.md)
- [Fyrox 1.0 Release Candidate](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0-rc-1/)
- [hecs GitHub](https://github.com/Ralith/hecs)
- [Legion GitHub](https://github.com/amethyst/legion)
- [generational-arena](https://docs.rs/generational-arena/latest/generational_arena/)
- [slotmap](https://docs.rs/slotmap/)
- [Generational Indices Guide](https://lucassardois.medium.com/generational-indices-guide-8e3c5f7fd594)
- [Arenas in Rust](https://manishearth.github.io/blog/2021/03/15/arenas-in-rust/)
- [Heron PhysicMaterial](https://docs.rs/heron/0.7.0/heron/struct.PhysicMaterial.html)
- [Avian Physics 0.3](https://joonaa.dev/blog/08/avian-0-3)
- [Bevy Bundles Cheat Book](https://bevy-cheatbook.github.io/programming/bundle.html)
- [Making Games in Rust - Floors and Gravity](https://dev.to/sbelzile/making-games-in-rust-part-3-floors-and-gravity-3lag)
- [Collision Detection in Bevy using Rapier](https://rancic.org/blog/collision-detection-in-bevy/)

---

## Appendix: Quick Reference

| Pattern | Library/Engine | Rust4D Applicability |
|---------|----------------|---------------------|
| Full ECS | Bevy, hecs, legion | Low priority - current architecture sufficient |
| Generational Handles | slotmap, generational-arena | **High priority** - prevents stale handle bugs |
| RigidBody/Collider separation | Rapier | Medium priority - enables compound shapes |
| Collision Groups | Rapier | Medium priority - enables selective collision |
| Physics Materials | Rapier, Heron, Avian | **High priority** - enables friction |
| Collision Events | Rapier, bevy_rapier | **High priority** - enables game logic reactions |
| Component Bundles | Bevy | Low priority - builder pattern works |
| Scene Graph | Fyrox | Low priority - flat registry sufficient for now |
