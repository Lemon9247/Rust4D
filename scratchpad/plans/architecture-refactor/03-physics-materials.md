# Phase 3: Physics Materials

**Status:** Not Started
**Sessions:** 1
**Dependencies:** None (can start immediately)
**Parallelizable With:** Phase 1, Phase 2

---

## Goal

Add friction to the physics system via a `PhysicsMaterial` struct. Currently objects slide infinitely on surfaces.

---

## Problem

Current physics has no friction:
```rust
// rust4d_physics/src/world.rs
pub struct PhysicsConfig {
    pub gravity: f32,
    pub floor_y: f32,
    pub restitution: f32,  // Only bounce, no friction!
}
```

---

## Solution

Add `PhysicsMaterial` with friction and restitution per-collider:

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicsMaterial {
    pub friction: f32,      // 0.0 = ice, 1.0 = rubber
    pub restitution: f32,   // 0.0 = no bounce, 1.0 = perfect bounce
}

impl PhysicsMaterial {
    pub fn combine(&self, other: &Self) -> Self {
        Self {
            friction: (self.friction * other.friction).sqrt(),
            restitution: self.restitution.max(other.restitution),
        }
    }
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/rust4d_physics/src/lib.rs` | Add `material` module, re-export |
| `crates/rust4d_physics/src/material.rs` | **NEW** - PhysicsMaterial struct |
| `crates/rust4d_physics/src/body.rs` | Add `material` field to RigidBody4D |
| `crates/rust4d_physics/src/world.rs` | Apply friction in collision response |
| `src/main.rs` | Set materials when creating bodies |

---

## Implementation Steps

### Wave 1: Material Definition (Sequential)

1. Create `crates/rust4d_physics/src/material.rs`:
   ```rust
   /// Physical material properties for collision response
   #[derive(Clone, Copy, Debug, PartialEq)]
   pub struct PhysicsMaterial {
       /// Friction coefficient (0.0 = ice, 1.0 = rubber)
       pub friction: f32,
       /// Restitution/bounciness (0.0 = no bounce, 1.0 = perfect bounce)
       pub restitution: f32,
   }

   impl Default for PhysicsMaterial {
       fn default() -> Self {
           Self {
               friction: 0.5,
               restitution: 0.0,
           }
       }
   }

   impl PhysicsMaterial {
       pub const ICE: Self = Self { friction: 0.05, restitution: 0.1 };
       pub const RUBBER: Self = Self { friction: 0.9, restitution: 0.8 };
       pub const METAL: Self = Self { friction: 0.3, restitution: 0.3 };
       pub const WOOD: Self = Self { friction: 0.5, restitution: 0.2 };
       pub const CONCRETE: Self = Self { friction: 0.7, restitution: 0.1 };

       pub fn new(friction: f32, restitution: f32) -> Self {
           Self {
               friction: friction.clamp(0.0, 1.0),
               restitution: restitution.clamp(0.0, 1.0),
           }
       }

       /// Combine two materials for collision response
       /// Uses geometric mean for friction, max for restitution
       pub fn combine(&self, other: &Self) -> Self {
           Self {
               friction: (self.friction * other.friction).sqrt(),
               restitution: self.restitution.max(other.restitution),
           }
       }
   }
   ```

2. Update `lib.rs`:
   ```rust
   mod material;
   pub use material::PhysicsMaterial;
   ```

### Wave 2: Body Integration (Sequential)

1. Update `RigidBody4D` in `body.rs`:
   ```rust
   pub struct RigidBody4D {
       pub position: Vec4,
       pub velocity: Vec4,
       pub mass: f32,
       pub material: PhysicsMaterial,  // NEW - replaces restitution
       pub affected_by_gravity: bool,
       pub collider: Collider,
       pub is_static: bool,
   }
   ```

2. Update constructors:
   ```rust
   impl RigidBody4D {
       pub fn new_sphere(position: Vec4, radius: f32) -> Self {
           Self {
               position,
               velocity: Vec4::ZERO,
               mass: 1.0,
               material: PhysicsMaterial::default(),
               affected_by_gravity: true,
               collider: Collider::Sphere(Sphere4D::new(position, radius)),
               is_static: false,
           }
       }

       // Add builder method
       pub fn with_material(mut self, material: PhysicsMaterial) -> Self {
           self.material = material;
           self
       }
   }
   ```

3. Remove old `restitution` field if it exists separately

### Wave 3: Collision Response (Sequential)

1. Update floor collision in `world.rs`:
   ```rust
   fn resolve_floor_collision(&mut self, body_idx: usize) {
       let body = &mut self.bodies[body_idx];

       // Get combined material (body + floor)
       let combined = body.material.combine(&self.floor_material);

       // Normal collision response (bounce)
       if body.velocity.y < 0.0 {
           body.velocity.y = -body.velocity.y * combined.restitution;
       }

       // Friction - reduce horizontal velocity
       let friction_factor = 1.0 - combined.friction;
       body.velocity.x *= friction_factor;
       body.velocity.z *= friction_factor;
       body.velocity.w *= friction_factor;

       // Clamp small velocities to zero (prevents jitter)
       if body.velocity.x.abs() < 0.01 { body.velocity.x = 0.0; }
       if body.velocity.z.abs() < 0.01 { body.velocity.z = 0.0; }
       if body.velocity.w.abs() < 0.01 { body.velocity.w = 0.0; }
   }
   ```

2. Add floor material to `PhysicsWorld`:
   ```rust
   pub struct PhysicsWorld {
       bodies: Vec<RigidBody4D>,  // Will become SlotMap in Phase 1
       floor: Plane4D,
       floor_material: PhysicsMaterial,  // NEW
       pub config: PhysicsConfig,
   }
   ```

3. Update `PhysicsConfig` to include default floor material:
   ```rust
   pub struct PhysicsConfig {
       pub gravity: f32,
       pub floor_y: f32,
       pub floor_material: PhysicsMaterial,
   }

   impl PhysicsConfig {
       pub fn new(gravity: f32, floor_y: f32) -> Self {
           Self {
               gravity,
               floor_y,
               floor_material: PhysicsMaterial::CONCRETE,
           }
       }

       pub fn with_floor_material(mut self, material: PhysicsMaterial) -> Self {
           self.floor_material = material;
           self
       }
   }
   ```

### Wave 4: Main Integration (Sequential)

1. Update physics config in `main.rs`:
   ```rust
   let physics_config = PhysicsConfig::new(GRAVITY, FLOOR_Y)
       .with_floor_material(PhysicsMaterial::CONCRETE);
   ```

2. Set material when creating tesseract:
   ```rust
   let tesseract_body = RigidBody4D::new_aabb(tesseract_start, half_extents)
       .with_gravity(true)
       .with_mass(10.0)
       .with_material(PhysicsMaterial::WOOD);
   ```

---

## Commits

1. "Add PhysicsMaterial struct with friction and restitution"
2. "Add material field to RigidBody4D"
3. "Apply friction in floor collision response"
4. "Set physics materials in main.rs"

---

## Verification

1. **Unit tests:**
   ```rust
   #[test]
   fn test_material_combine() {
       let ice = PhysicsMaterial::ICE;
       let rubber = PhysicsMaterial::RUBBER;
       let combined = ice.combine(&rubber);

       // Friction: sqrt(0.05 * 0.9) ≈ 0.21
       assert!((combined.friction - 0.21).abs() < 0.01);
       // Restitution: max(0.1, 0.8) = 0.8
       assert_eq!(combined.restitution, 0.8);
   }

   #[test]
   fn test_friction_slows_body() {
       let mut world = PhysicsWorld::new(PhysicsConfig::new(-10.0, 0.0));
       let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.5, 0.0, 0.0), 0.5)
           .with_material(PhysicsMaterial::new(0.5, 0.0));
       body.velocity = Vec4::new(10.0, 0.0, 0.0, 0.0);  // Moving right
       world.add_body(body);

       // Step until on ground
       for _ in 0..100 {
           world.step(0.016);
       }

       let body = world.get_body(BodyKey::from_raw_parts(0, 1)).unwrap();
       // Velocity should have decreased due to friction
       assert!(body.velocity.x < 5.0);
   }
   ```

2. **Manual test:**
   - Push tesseract, watch it slow down (not slide forever)
   - Try different materials (ICE vs RUBBER)

---

## Rollback Plan

If friction causes issues, set all materials to `friction: 0.0` to restore old behavior.
