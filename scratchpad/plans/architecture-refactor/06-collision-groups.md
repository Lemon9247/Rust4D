# Phase 6: Collision Groups

**Status:** Not Started
**Sessions:** 1
**Dependencies:** Phase 4 (Static Colliders)
**Parallelizable With:** Phase 5 (Player Integration)

---

## Goal

Add collision filtering via bitmask groups, allowing selective collision between entities (e.g., player projectiles don't hit player).

---

## Problem

Currently everything collides with everything:
- Can't have trigger zones that detect but don't push
- Can't filter projectiles to hit only enemies
- Can't have ghost/no-clip modes

---

## Solution

Use Rapier-style collision groups with bitmasks:

```rust
bitflags! {
    pub struct CollisionLayer: u32 {
        const DEFAULT   = 1 << 0;
        const PLAYER    = 1 << 1;
        const ENEMY     = 1 << 2;
        const STATIC    = 1 << 3;
        const TRIGGER   = 1 << 4;
        const PROJECTILE = 1 << 5;
    }
}

pub struct CollisionFilter {
    pub layer: CollisionLayer,  // What I am
    pub mask: CollisionLayer,   // What I collide with
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/rust4d_physics/Cargo.toml` | Add bitflags dependency |
| `crates/rust4d_physics/src/collision.rs` | Add CollisionLayer, CollisionFilter |
| `crates/rust4d_physics/src/body.rs` | Add filter field to RigidBody4D |
| `crates/rust4d_physics/src/world.rs` | Check filter before collision |
| `src/main.rs` | Set collision filters on entities |

---

## Implementation Steps

### Wave 1: Add Dependencies (Sequential)

1. Add to workspace `Cargo.toml`:
   ```toml
   [workspace.dependencies]
   bitflags = "2.4"
   ```

2. Add to `crates/rust4d_physics/Cargo.toml`:
   ```toml
   bitflags.workspace = true
   ```

### Wave 2: Collision Filter Types (Sequential)

1. Add to `collision.rs`:
   ```rust
   use bitflags::bitflags;

   bitflags! {
       /// Collision layers for filtering interactions
       #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
       pub struct CollisionLayer: u32 {
           /// Default layer for most objects
           const DEFAULT = 1 << 0;
           /// Player character
           const PLAYER = 1 << 1;
           /// Enemy entities
           const ENEMY = 1 << 2;
           /// Static world geometry
           const STATIC = 1 << 3;
           /// Trigger zones (detect but don't push)
           const TRIGGER = 1 << 4;
           /// Projectiles
           const PROJECTILE = 1 << 5;
           /// Pickups/collectibles
           const PICKUP = 1 << 6;

           /// Everything collides
           const ALL = 0xFFFFFFFF;
       }
   }

   /// Filter that determines what a body can collide with
   #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   pub struct CollisionFilter {
       /// What layers this body belongs to
       pub layer: CollisionLayer,
       /// What layers this body can collide with
       pub mask: CollisionLayer,
   }

   impl Default for CollisionFilter {
       fn default() -> Self {
           Self {
               layer: CollisionLayer::DEFAULT,
               mask: CollisionLayer::ALL,
           }
       }
   }

   impl CollisionFilter {
       pub fn new(layer: CollisionLayer, mask: CollisionLayer) -> Self {
           Self { layer, mask }
       }

       /// Check if two filters allow collision between their bodies
       pub fn collides_with(&self, other: &Self) -> bool {
           // Both must accept each other
           self.mask.contains(other.layer) && other.mask.contains(self.layer)
       }

       // Preset filters for common cases

       /// Player: collides with static, enemies, pickups
       pub fn player() -> Self {
           Self {
               layer: CollisionLayer::PLAYER,
               mask: CollisionLayer::STATIC | CollisionLayer::ENEMY | CollisionLayer::PICKUP | CollisionLayer::DEFAULT,
           }
       }

       /// Enemy: collides with static, player, projectiles
       pub fn enemy() -> Self {
           Self {
               layer: CollisionLayer::ENEMY,
               mask: CollisionLayer::STATIC | CollisionLayer::PLAYER | CollisionLayer::PROJECTILE | CollisionLayer::DEFAULT,
           }
       }

       /// Static: everything can collide with it
       pub fn static_world() -> Self {
           Self {
               layer: CollisionLayer::STATIC,
               mask: CollisionLayer::ALL,
           }
       }

       /// Trigger: detects but doesn't physically collide
       pub fn trigger(detects: CollisionLayer) -> Self {
           Self {
               layer: CollisionLayer::TRIGGER,
               mask: detects,
           }
       }

       /// Projectile: hits enemies, static world
       pub fn player_projectile() -> Self {
           Self {
               layer: CollisionLayer::PROJECTILE,
               mask: CollisionLayer::ENEMY | CollisionLayer::STATIC,
           }
       }
   }
   ```

2. Export from `lib.rs`:
   ```rust
   pub use collision::{CollisionLayer, CollisionFilter, Contact, /* existing */};
   ```

### Wave 3: Body and Collider Integration (Sequential)

1. Add filter to `RigidBody4D`:
   ```rust
   pub struct RigidBody4D {
       pub position: Vec4,
       pub velocity: Vec4,
       pub mass: f32,
       pub material: PhysicsMaterial,
       pub body_type: BodyType,
       pub collider: Collider,
       pub filter: CollisionFilter,  // NEW
       pub grounded: bool,
   }

   impl RigidBody4D {
       pub fn with_filter(mut self, filter: CollisionFilter) -> Self {
           self.filter = filter;
           self
       }

       pub fn with_layer(mut self, layer: CollisionLayer) -> Self {
           self.filter.layer = layer;
           self
       }

       pub fn with_mask(mut self, mask: CollisionLayer) -> Self {
           self.filter.mask = mask;
           self
       }
   }
   ```

2. Add filter to `StaticCollider`:
   ```rust
   pub struct StaticCollider {
       pub collider: Collider,
       pub material: PhysicsMaterial,
       pub filter: CollisionFilter,  // NEW
   }

   impl StaticCollider {
       pub fn floor(y: f32, material: PhysicsMaterial) -> Self {
           Self {
               collider: Collider::Plane(Plane4D::new(Vec4::Y, y)),
               material,
               filter: CollisionFilter::static_world(),
           }
       }

       pub fn with_filter(mut self, filter: CollisionFilter) -> Self {
           self.filter = filter;
           self
       }
   }
   ```

### Wave 4: Collision Detection Filter (Sequential)

1. Update `step()` to check filters:
   ```rust
   fn step(&mut self, dt: f32) {
       // ... velocity/position integration ...

       // Body vs static colliders
       for (key, body) in &mut self.bodies {
           for static_col in &self.static_colliders {
               // Check filter first (cheap)
               if !body.filter.collides_with(&static_col.filter) {
                   continue;
               }

               if let Some(contact) = self.check_collision(&body.collider, &static_col.collider) {
                   self.resolve_static_collision(key, &contact, &static_col.material);
               }
           }
       }

       // Body vs body collisions
       let keys: Vec<_> = self.bodies.keys().collect();
       for i in 0..keys.len() {
           for j in (i+1)..keys.len() {
               let key_a = keys[i];
               let key_b = keys[j];

               let (body_a, body_b) = self.bodies.get2_mut(key_a, key_b).unwrap();

               // Check filter first
               if !body_a.filter.collides_with(&body_b.filter) {
                   continue;
               }

               if let Some(contact) = Self::check_body_collision(body_a, body_b) {
                   Self::resolve_body_collision(body_a, body_b, &contact);
               }
           }
       }
   }
   ```

### Wave 5: Collision Events (Sequential)

1. Add collision event struct:
   ```rust
   #[derive(Clone, Debug)]
   pub struct CollisionEvent {
       pub body_a: BodyKey,
       pub body_b: Option<BodyKey>,  // None for static collider
       pub contact: Contact,
       pub is_trigger: bool,
   }

   pub struct PhysicsWorld {
       // ...
       collision_events: Vec<CollisionEvent>,
   }

   impl PhysicsWorld {
       pub fn drain_collision_events(&mut self) -> impl Iterator<Item = CollisionEvent> + '_ {
           self.collision_events.drain(..)
       }
   }
   ```

2. Collect events during collision detection:
   ```rust
   // For trigger collisions, don't resolve physics, just record event
   if body.filter.layer.contains(CollisionLayer::TRIGGER) ||
      static_col.filter.layer.contains(CollisionLayer::TRIGGER) {
       self.collision_events.push(CollisionEvent {
           body_a: key,
           body_b: None,
           contact,
           is_trigger: true,
       });
       continue;  // Don't apply physics response
   }
   ```

### Wave 6: Main Integration (Sequential)

1. Set filters when creating bodies:
   ```rust
   // Player
   let player_body = RigidBody4D::new_sphere(player_start, PLAYER_RADIUS)
       .with_body_type(BodyType::Kinematic)
       .with_filter(CollisionFilter::player());

   // Tesseract (dynamic object)
   let tesseract_body = RigidBody4D::new_aabb(tesseract_start, half_extents)
       .with_gravity(true)
       .with_filter(CollisionFilter::default());  // Collides with everything

   // Floor (static)
   world.physics_mut().unwrap().add_static_collider(
       StaticCollider::floor(FLOOR_Y, PhysicsMaterial::CONCRETE)
           // Uses static_world() filter by default
   );
   ```

2. Handle collision events in game loop:
   ```rust
   // After physics step
   for event in self.world.physics_mut().unwrap().drain_collision_events() {
       if event.is_trigger {
           // Handle trigger (e.g., pickup collection)
           println!("Trigger collision: {:?}", event);
       }
   }
   ```

---

## Commits

1. "Add bitflags dependency"
2. "Add CollisionLayer and CollisionFilter types"
3. "Add collision filter to RigidBody4D and StaticCollider"
4. "Filter collisions based on layer masks"
5. "Add collision events for triggers"
6. "Set collision filters in main.rs"

---

## Verification

1. **Unit tests:**
   ```rust
   #[test]
   fn test_collision_filter() {
       let player = CollisionFilter::player();
       let enemy = CollisionFilter::enemy();
       let static_world = CollisionFilter::static_world();

       // Player collides with static
       assert!(player.collides_with(&static_world));

       // Player collides with enemy
       assert!(player.collides_with(&enemy));

       // Two projectiles don't collide
       let proj_a = CollisionFilter::player_projectile();
       let proj_b = CollisionFilter::player_projectile();
       assert!(!proj_a.collides_with(&proj_b));
   }

   #[test]
   fn test_trigger_no_physics_response() {
       let mut world = PhysicsWorld::new(PhysicsConfig::new(-10.0));

       // Trigger zone
       world.add_static_collider(StaticCollider::aabb(
           Vec4::new(0.0, 1.0, 0.0, 0.0),
           Vec4::new(2.0, 2.0, 2.0, 2.0),
           PhysicsMaterial::default(),
       ).with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER)));

       // Player passes through trigger
       let player = RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
           .with_filter(CollisionFilter::player());
       player.velocity = Vec4::new(1.0, 0.0, 0.0, 0.0);
       world.add_body(player);

       world.step(0.1);

       // Player should have moved (not stopped by trigger)
       let events: Vec<_> = world.drain_collision_events().collect();
       assert!(events.iter().any(|e| e.is_trigger));
   }
   ```

2. **Manual test:**
   - Existing collision still works
   - Add debug trigger zone, verify event is raised

---

## Future Use Cases

- **Pickups:** Player touches pickup, pickup disappears
- **Damage zones:** Player/enemy enters zone, takes damage
- **Checkpoints:** Player passes through, checkpoint saved
- **Portals:** Player enters, teleported to destination
- **One-way platforms:** Player can jump through from below

---

## Rollback Plan

Set all filters to `CollisionFilter::default()` (collide with everything) to restore old behavior.
