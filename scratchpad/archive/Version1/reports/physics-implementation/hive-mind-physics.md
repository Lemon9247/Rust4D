# Hive Mind Coordination: Physics Implementation

## Shared Decisions

### BodyHandle Type
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) usize);
```

### PhysicsConfig Defaults
```rust
pub struct PhysicsConfig {
    pub gravity: f32,      // -20.0 (Y-axis, negative = down)
    pub floor_y: f32,      // 0.0
    pub restitution: f32,  // 0.0 (no bounce by default)
}
```

### Player Physics Values
- Player radius: 0.5
- Jump velocity: 8.0
- Move speed: 5.0 (handled by CameraController)

### Contact Struct (already implemented in collision.rs)
```rust
pub struct Contact {
    pub point: Vec4,
    pub normal: Vec4,
    pub penetration: f32,
}
```

## Agent Status

### Wave 2 - COMPLETE

### Agent A (Physics World)
- [x] body.rs - RigidBody4D, BodyHandle
- [x] world.rs - PhysicsWorld, PhysicsConfig, step()
- [x] Unit tests

### Agent B (Player Physics)
- [x] player.rs - PlayerPhysics
- [x] camera_controller.rs - Add jump input
- [x] Unit tests

---

### Wave 3 - IN PROGRESS

### Agent C (Core Integration)
- [ ] entity.rs - Add `physics_body: Option<BodyHandle>`
- [ ] world.rs - Add PhysicsWorld, transform sync
- [ ] Cargo.toml - Add rust4d_physics dependency

### Agent D (Main Integration)
- [ ] main.rs - Add PlayerPhysics to App
- [ ] main.rs - Create PhysicsWorld with tesseract body
- [ ] main.rs - Modify game loop for physics
- [ ] main.rs - Handle push interaction

## Wave 3 Coordination

**Agent C creates these types that Agent D depends on:**
- `World::with_physics(config)` - enables physics on World
- `World::physics_mut()` - returns `Option<&mut PhysicsWorld>`
- `Entity.physics_body: Option<BodyHandle>` - links entity to physics

**If Agent D finishes before Agent C:**
Agent D should manage PhysicsWorld separately in App struct and manually sync transforms, rather than using World's physics integration.

**Floor Configuration:**
- Hyperplane floor Y: -2.0 (visual floor in rust4d_core)
- PhysicsConfig floor_y: 0.0 (default) - BUT should be changed to -2.0 to match
- Player physics floor: Plane4D::floor(-2.0)

**Tesseract Setup:**
- Tesseract size: 2.0 (half-extents = 1.0)
- Starting position: Vec4::new(0.0, 0.0, 0.0, 0.0) or slightly above floor
- Should be dynamic body with gravity enabled

**Player Setup:**
- Starting position: Vec4::new(0.0, 0.0, 5.0, 0.0) - behind tesseract
- Player radius: 0.5
- Player needs to be above floor Y=-2.0, so start at Y=0.0 or higher

## Questions / Notes

**AGENTS: This file is your shared communication channel!**
- Read this file periodically to check for updates from other agents
- Write questions, blockers, or status updates here
- If you need something from another agent, ask here
- Update your status when you complete tasks

### Agent B Notes

**Completed Implementation:**

1. **player.rs** - Full PlayerPhysics implementation with:
   - `new(position)` and `with_config(position, radius, jump_velocity)` constructors
   - `collider()` returns Sphere4D at current position
   - `apply_movement(movement)` applies XZ/W movement, preserves Y velocity
   - `jump()` applies jump_velocity if grounded
   - `step(dt, gravity, floor)` integrates gravity, moves, and resolves floor collision
   - Added ground margin (0.01) to prevent grounded/airborne flickering at rest
   - 13 unit tests all passing

2. **camera_controller.rs** - Added jump input handling:
   - `jump_pressed: bool` field to track Space presses
   - `consume_jump()` - returns and clears jump flag (for physics mode)
   - `get_movement_input()` - returns (forward, right) in -1.0 to 1.0 range
   - `get_w_input()` - returns ana/kata input for 4D movement
   - Existing Space behavior kept for fly mode compatibility

3. **lib.rs** - Exports player module and constants:
   - `PlayerPhysics`
   - `DEFAULT_PLAYER_RADIUS` (0.5)
   - `DEFAULT_JUMP_VELOCITY` (8.0)

**Note:** There's a failing test in world.rs (`test_body_body_collision_sphere_vs_static_aabb`) but that's Agent A's scope.

### Agent A Notes

**Completed Implementation:**

1. **body.rs** - Full RigidBody4D and BodyHandle implementation:
   - `BodyHandle(usize)` - lightweight handle for body lookup, Copy + Eq + Hash
   - `RigidBody4D` with position, velocity, mass, restitution, affected_by_gravity, collider, is_static
   - `new_sphere(position, radius)` and `new_aabb(position, half_extents)` constructors
   - `new_static_aabb(position, half_extents)` for static obstacle bodies
   - Builder methods: `with_velocity`, `with_mass`, `with_restitution`, `with_gravity`, `with_static`
   - `set_position(position)` and `apply_correction(delta)` keep collider synced
   - 10 unit tests passing

2. **world.rs** - Full PhysicsWorld and PhysicsConfig implementation:
   - `PhysicsConfig` with gravity (-20.0), floor_y (0.0), restitution (0.0) defaults
   - `PhysicsWorld::new()` and `with_config(config)` constructors
   - `add_body(body) -> BodyHandle` adds bodies and returns handles
   - `get_body(handle)` and `get_body_mut(handle)` for body access
   - `step(dt)` implements full simulation:
     1. Gravity application to non-static, gravity-enabled bodies
     2. Velocity integration into position (collider synced)
     3. Floor collision detection and resolution (push out + velocity response with restitution)
     4. Body-body collision detection (sphere-sphere, sphere-AABB, AABB-AABB)
     5. Body-body collision resolution (mass-based push, velocity response)
   - 16 unit tests passing

3. **collision.rs bugfix** - Fixed `sphere_vs_plane` penetration calculation:
   - Old: `penetration = radius - signed_dist.abs()` (incorrect for deep penetration)
   - New: `penetration = radius - signed_dist` (handles both above and below plane cases correctly)
   - All existing collision tests still pass

4. **lib.rs** - Updated exports:
   - `BodyHandle`, `RigidBody4D` from body module
   - `PhysicsConfig`, `PhysicsWorld` from world module

**All 55 tests in rust4d_physics pass.**
