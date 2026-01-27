# Phase 4: Static Colliders

**Status:** Not Started
**Sessions:** 1-2
**Dependencies:** Phase 1 (Handles), Phase 3 (Materials)
**Parallelizable With:** None

---

## Goal

Replace the hardcoded single floor plane with a flexible static collider system. Enable multiple floors, walls, ramps, and platforms.

---

## Problem

Current physics has a single hardcoded floor:
```rust
// rust4d_physics/src/world.rs
pub struct PhysicsWorld {
    bodies: Vec<RigidBody4D>,
    floor: Plane4D,         // <-- Only one, hardcoded
    pub config: PhysicsConfig,
}
```

The floor is also configured via `PhysicsConfig::floor_y`, mixing config with scene data.

---

## Solution

Add a `static_colliders` list to PhysicsWorld:

```rust
pub struct StaticCollider {
    pub collider: Collider,
    pub material: PhysicsMaterial,
}

pub struct PhysicsWorld {
    bodies: SlotMap<BodyKey, RigidBody4D>,
    static_colliders: Vec<StaticCollider>,  // Multiple static shapes
    pub config: PhysicsConfig,
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/rust4d_physics/src/world.rs` | Replace floor with static_colliders list |
| `crates/rust4d_physics/src/body.rs` | Add StaticCollider struct |
| `crates/rust4d_physics/src/collision.rs` | Add plane collision detection |
| `crates/rust4d_core/src/world.rs` | Expose static collider methods |
| `src/main.rs` | Create floor as static collider |

---

## Implementation Steps

### Wave 1: Static Collider Type (Sequential)

1. Add to `body.rs` or new `static_collider.rs`:
   ```rust
   /// A collider that doesn't move (floors, walls, platforms)
   #[derive(Clone, Debug)]
   pub struct StaticCollider {
       pub collider: Collider,
       pub material: PhysicsMaterial,
   }

   impl StaticCollider {
       pub fn plane(normal: Vec4, distance: f32, material: PhysicsMaterial) -> Self {
           Self {
               collider: Collider::Plane(Plane4D::new(normal, distance)),
               material,
           }
       }

       pub fn floor(y: f32, material: PhysicsMaterial) -> Self {
           Self::plane(Vec4::new(0.0, 1.0, 0.0, 0.0), y, material)
       }

       pub fn aabb(center: Vec4, half_extents: Vec4, material: PhysicsMaterial) -> Self {
           Self {
               collider: Collider::AABB(AABB4D::from_center_half_extents(center, half_extents)),
               material,
           }
       }
   }
   ```

2. Ensure `Collider` enum includes `Plane`:
   ```rust
   pub enum Collider {
       Sphere(Sphere4D),
       AABB(AABB4D),
       Plane(Plane4D),  // May already exist
   }
   ```

### Wave 2: PhysicsWorld Changes (Sequential)

1. Update `PhysicsWorld`:
   ```rust
   pub struct PhysicsWorld {
       bodies: SlotMap<BodyKey, RigidBody4D>,
       static_colliders: Vec<StaticCollider>,
       pub config: PhysicsConfig,
   }

   impl PhysicsWorld {
       pub fn new(config: PhysicsConfig) -> Self {
           Self {
               bodies: SlotMap::with_key(),
               static_colliders: Vec::new(),
               config,
           }
       }

       pub fn add_static_collider(&mut self, collider: StaticCollider) {
           self.static_colliders.push(collider);
       }

       pub fn static_colliders(&self) -> &[StaticCollider] {
           &self.static_colliders
       }
   }
   ```

2. Remove `floor: Plane4D` field

3. Remove `floor_y` from `PhysicsConfig`:
   ```rust
   pub struct PhysicsConfig {
       pub gravity: f32,
       pub floor_material: PhysicsMaterial,  // Default material for floors
   }
   ```

### Wave 3: Collision Detection (Sequential)

1. Update `step()` to check all static colliders:
   ```rust
   fn step(&mut self, dt: f32) {
       // Apply gravity and integrate
       for (_, body) in &mut self.bodies {
           if body.affected_by_gravity && !body.is_static {
               body.velocity.y += self.config.gravity * dt;
           }
           body.position += body.velocity * dt;
       }

       // Check collisions with static colliders
       for (key, body) in &mut self.bodies {
           if body.is_static {
               continue;
           }

           for static_col in &self.static_colliders {
               if let Some(contact) = self.check_collision(&body.collider, &static_col.collider) {
                   self.resolve_static_collision(body, &contact, &static_col.material);
               }
           }
       }

       // Body-body collisions (existing code)
       // ...
   }

   fn check_collision(&self, body_col: &Collider, static_col: &Collider) -> Option<Contact> {
       match (body_col, static_col) {
           (Collider::Sphere(sphere), Collider::Plane(plane)) => {
               sphere_vs_plane(sphere, plane)
           }
           (Collider::AABB(aabb), Collider::Plane(plane)) => {
               aabb_vs_plane(aabb, plane)
           }
           (Collider::Sphere(sphere), Collider::AABB(aabb)) => {
               sphere_vs_aabb(sphere, aabb)
           }
           (Collider::AABB(a), Collider::AABB(b)) => {
               aabb_vs_aabb(a, b)
           }
           _ => None,
       }
   }

   fn resolve_static_collision(
       &self,
       body: &mut RigidBody4D,
       contact: &Contact,
       static_material: &PhysicsMaterial,
   ) {
       // Push out of collision
       body.position += contact.normal * contact.penetration;

       // Update collider position
       body.sync_collider();

       // Combine materials
       let combined = body.material.combine(static_material);

       // Bounce (perpendicular to surface)
       let velocity_along_normal = body.velocity.dot(contact.normal);
       if velocity_along_normal < 0.0 {
           body.velocity -= contact.normal * velocity_along_normal * (1.0 + combined.restitution);
       }

       // Friction (parallel to surface)
       let velocity_tangent = body.velocity - contact.normal * body.velocity.dot(contact.normal);
       let tangent_speed = velocity_tangent.length();
       if tangent_speed > 0.001 {
           let friction_impulse = tangent_speed * combined.friction;
           let friction_direction = velocity_tangent.normalized();
           body.velocity -= friction_direction * friction_impulse.min(tangent_speed);
       }
   }
   ```

### Wave 4: Main Integration (Sequential)

1. Update `main.rs` to create floor as static collider:
   ```rust
   // In App::new()
   let physics_config = PhysicsConfig::new(GRAVITY);
   let mut world = World::with_capacity(2).with_physics(physics_config);

   // Add floor as static collider
   world.physics_mut().unwrap().add_static_collider(
       StaticCollider::floor(FLOOR_Y, PhysicsMaterial::CONCRETE)
   );

   // Optionally add walls
   // world.physics_mut().unwrap().add_static_collider(
   //     StaticCollider::plane(
   //         Vec4::new(1.0, 0.0, 0.0, 0.0),  // Normal pointing +X
   //         -10.0,                           // At X = -10
   //         PhysicsMaterial::CONCRETE,
   //     )
   // );
   ```

2. Update player physics floor handling (if separate):
   ```rust
   // PlayerPhysics should also use static colliders
   // Or: integrate player into PhysicsWorld (Phase 5)
   ```

---

## Commits

1. "Add StaticCollider struct for immovable collision shapes"
2. "Replace hardcoded floor with static_colliders list"
3. "Update collision detection for static colliders"
4. "Create floor as static collider in main.rs"

---

## Verification

1. **Unit tests:**
   ```rust
   #[test]
   fn test_multiple_static_colliders() {
       let mut world = PhysicsWorld::new(PhysicsConfig::new(-10.0));

       // Floor at Y = 0
       world.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));

       // Ceiling at Y = 10
       world.add_static_collider(StaticCollider::plane(
           Vec4::new(0.0, -1.0, 0.0, 0.0),  // Normal pointing down
           -10.0,
           PhysicsMaterial::METAL,
       ));

       // Ball in the middle
       let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
       world.add_body(body);

       // Step simulation - ball should bounce between floor and ceiling
       for _ in 0..1000 {
           world.step(0.016);
       }

       // Ball should still be between 0 and 10
       let ball = world.bodies().next().unwrap();
       assert!(ball.position.y >= 0.0 && ball.position.y <= 10.0);
   }
   ```

2. **Manual test:**
   - Objects land on floor (existing behavior)
   - Try adding a wall, verify collision works

---

## Future Extensions

After this phase, adding new static geometry is trivial:
```rust
// Ramp
world.add_static_collider(StaticCollider::plane(
    Vec4::new(0.0, 0.707, 0.707, 0.0).normalized(),
    0.0,
    PhysicsMaterial::WOOD,
));

// Platform box
world.add_static_collider(StaticCollider::aabb(
    Vec4::new(5.0, 2.0, 0.0, 0.0),
    Vec4::new(2.0, 0.5, 2.0, 2.0),
    PhysicsMaterial::METAL,
));
```

---

## Rollback Plan

If issues arise, temporarily restore the single `floor: Plane4D` field and keep `static_colliders` for future use.
