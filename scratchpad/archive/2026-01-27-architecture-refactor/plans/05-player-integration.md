# Phase 5: Player Integration

**Status:** Not Started
**Sessions:** 1-2
**Dependencies:** Phase 4 (Static Colliders)
**Parallelizable With:** Phase 6 (Collision Groups)

---

## Goal

Integrate `PlayerPhysics` into `PhysicsWorld` so there's one unified physics system instead of two parallel collision systems.

---

## Problem

Current architecture has two separate physics systems:

```rust
// main.rs App struct
struct App {
    player_physics: PlayerPhysics,     // Separate player physics
    physics_floor: PhysicsPlane,       // Separate floor for player!
    world: World,                      // Contains PhysicsWorld with its own floor
    // ...
}
```

This causes:
- Duplicate floor collision code
- Manual player-entity collision in `main.rs`
- Inconsistent physics behavior
- Hard to add player-world interactions

---

## Solution

Make the player a regular `RigidBody4D` in `PhysicsWorld` with special handling:

```rust
pub struct PhysicsWorld {
    bodies: SlotMap<BodyKey, RigidBody4D>,
    static_colliders: Vec<StaticCollider>,
    player_body: Option<BodyKey>,  // Reference to player, not separate physics
    pub config: PhysicsConfig,
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/rust4d_physics/src/player.rs` | Convert to player body wrapper |
| `crates/rust4d_physics/src/world.rs` | Add player body tracking |
| `crates/rust4d_input/src/camera_controller.rs` | Adjust input handling |
| `src/main.rs` | Remove separate player_physics, use unified system |

---

## Design Decision

**Option A: Player as Regular Body (Recommended)**
- Player is a `RigidBody4D` with special movement handling
- Jump is applied as velocity impulse
- Ground detection uses collision contacts

**Option B: Player Controller Pattern**
- Player has a `CharacterController` component
- Controller queries physics for ground contact
- Controller applies movement directly

We choose **Option A** for simplicity. The player body has:
- `is_kinematic: true` for direct velocity control
- Special ground detection
- Jump velocity application

---

## Implementation Steps

### Wave 1: Body Type Enhancement (Sequential)

1. Add body type enum to `body.rs`:
   ```rust
   #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
   pub enum BodyType {
       #[default]
       Dynamic,    // Full physics simulation
       Static,     // Never moves (redundant with StaticCollider, but useful for entity bodies)
       Kinematic,  // User-controlled velocity, no gravity
   }

   pub struct RigidBody4D {
       pub position: Vec4,
       pub velocity: Vec4,
       pub mass: f32,
       pub material: PhysicsMaterial,
       pub body_type: BodyType,  // Replaces is_static + affected_by_gravity
       pub collider: Collider,
       pub grounded: bool,       // NEW - set by physics step
   }
   ```

2. Update physics step to respect body types:
   ```rust
   fn step(&mut self, dt: f32) {
       for (_, body) in &mut self.bodies {
           match body.body_type {
               BodyType::Dynamic => {
                   // Apply gravity
                   body.velocity.y += self.config.gravity * dt;
                   // Integrate position
                   body.position += body.velocity * dt;
               }
               BodyType::Kinematic => {
                   // Only integrate position (user controls velocity)
                   body.position += body.velocity * dt;
               }
               BodyType::Static => {
                   // Don't move
               }
           }
           body.grounded = false;  // Will be set by collision detection
       }

       // Collision detection...
   }
   ```

### Wave 2: Player Body Tracking (Sequential)

1. Add player tracking to `PhysicsWorld`:
   ```rust
   pub struct PhysicsWorld {
       bodies: SlotMap<BodyKey, RigidBody4D>,
       static_colliders: Vec<StaticCollider>,
       player_body: Option<BodyKey>,
       pub config: PhysicsConfig,
   }

   impl PhysicsWorld {
       pub fn set_player_body(&mut self, key: BodyKey) {
           self.player_body = Some(key);
       }

       pub fn player(&self) -> Option<&RigidBody4D> {
           self.player_body.and_then(|key| self.bodies.get(key))
       }

       pub fn player_mut(&mut self) -> Option<&mut RigidBody4D> {
           self.player_body.and_then(|key| self.bodies.get_mut(key))
       }

       pub fn player_key(&self) -> Option<BodyKey> {
           self.player_body
       }
   }
   ```

2. Add ground detection in collision resolution:
   ```rust
   fn resolve_static_collision(&mut self, key: BodyKey, contact: &Contact, material: &PhysicsMaterial) {
       let body = self.bodies.get_mut(key).unwrap();

       // ... existing resolution code ...

       // Check if this is ground contact (normal pointing up)
       if contact.normal.y > 0.7 {
           body.grounded = true;
       }
   }
   ```

### Wave 3: Player Movement Methods (Sequential)

1. Add player-specific methods to `PhysicsWorld`:
   ```rust
   impl PhysicsWorld {
       /// Apply movement input to player (horizontal only)
       pub fn apply_player_movement(&mut self, direction: Vec4, speed: f32) {
           if let Some(body) = self.player_mut() {
               // Zero out Y component for horizontal movement
               let horizontal = Vec4::new(direction.x, 0.0, direction.z, direction.w).normalized();

               // Set horizontal velocity (don't affect vertical for jumping)
               body.velocity.x = horizontal.x * speed;
               body.velocity.z = horizontal.z * speed;
               body.velocity.w = horizontal.w * speed;
           }
       }

       /// Make player jump if grounded
       pub fn player_jump(&mut self, jump_velocity: f32) -> bool {
           if let Some(body) = self.player_mut() {
               if body.grounded {
                   body.velocity.y = jump_velocity;
                   body.grounded = false;
                   return true;
               }
           }
           false
       }

       /// Get player position for camera sync
       pub fn player_position(&self) -> Option<Vec4> {
           self.player().map(|b| b.position)
       }

       /// Check if player is on ground
       pub fn player_is_grounded(&self) -> bool {
           self.player().map(|b| b.grounded).unwrap_or(false)
       }
   }
   ```

### Wave 4: Main Integration (Sequential)

1. Remove old player physics from `App`:
   ```rust
   struct App {
       // REMOVE: player_physics: PlayerPhysics,
       // REMOVE: physics_floor: PhysicsPlane,

       world: World,
       player_body: BodyKey,  // Just store the key
       // ...
   }
   ```

2. Create player as physics body:
   ```rust
   fn new() -> Self {
       let mut world = World::with_capacity(3).with_physics(physics_config);

       // Create player body
       let player_body = RigidBody4D::new_sphere(player_start, PLAYER_RADIUS)
           .with_body_type(BodyType::Kinematic)
           .with_material(PhysicsMaterial::RUBBER);
       let player_key = world.physics_mut().unwrap().add_body(player_body);
       world.physics_mut().unwrap().set_player_body(player_key);

       // Create tesseract (as before)
       // ...

       Self {
           world,
           player_body: player_key,
           // ...
       }
   }
   ```

3. Update game loop:
   ```rust
   // In RedrawRequested handler:

   // Get movement input
   let (forward, right) = self.controller.get_movement_input();
   let camera_forward = self.camera.forward().with_y(0.0).normalized();
   let camera_right = self.camera.right().with_y(0.0).normalized();
   let move_dir = camera_forward * forward + camera_right * right;

   // Apply movement
   let physics = self.world.physics_mut().unwrap();
   physics.apply_player_movement(move_dir, MOVE_SPEED);

   // Handle jump
   if self.controller.consume_jump() {
       physics.player_jump(JUMP_VELOCITY);
   }

   // Step physics (handles ALL bodies including player)
   physics.step(dt);

   // Sync camera to player
   if let Some(pos) = physics.player_position() {
       self.camera.set_position(pos);
   }
   ```

---

## Commits

1. "Add BodyType enum (Dynamic/Static/Kinematic)"
2. "Add player body tracking to PhysicsWorld"
3. "Add player movement and jump methods"
4. "Remove separate PlayerPhysics, use unified system"

---

## Verification

1. **Unit tests:**
   ```rust
   #[test]
   fn test_player_grounded_detection() {
       let mut world = PhysicsWorld::new(PhysicsConfig::new(-10.0));
       world.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));

       let player = RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
           .with_body_type(BodyType::Kinematic);
       let key = world.add_body(player);
       world.set_player_body(key);

       // Initially not grounded
       assert!(!world.player_is_grounded());

       // Step until player lands
       for _ in 0..100 {
           world.step(0.016);
       }

       // Now should be grounded
       assert!(world.player_is_grounded());
   }

   #[test]
   fn test_player_jump() {
       let mut world = PhysicsWorld::new(PhysicsConfig::new(-10.0));
       world.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));

       let player = RigidBody4D::new_sphere(Vec4::new(0.0, 0.5, 0.0, 0.0), 0.5)
           .with_body_type(BodyType::Kinematic);
       let key = world.add_body(player);
       world.set_player_body(key);

       // Land the player
       for _ in 0..10 {
           world.step(0.016);
       }

       // Jump should succeed when grounded
       assert!(world.player_jump(10.0));
       assert!(world.player().unwrap().velocity.y > 0.0);

       // Jump should fail when airborne
       assert!(!world.player_jump(10.0));
   }
   ```

2. **Manual test:**
   - Walk around (WASD)
   - Jump (Space)
   - Push tesseract
   - Land on floor

---

## Migration Notes

After this phase:
- `PlayerPhysics` struct can be removed (or kept for legacy)
- Player-entity collisions happen automatically in `PhysicsWorld`
- Push mechanics work through body-body collision in physics step

---

## Rollback Plan

Keep `PlayerPhysics` as fallback. The new system is additive and can be disabled by not calling the new methods.
