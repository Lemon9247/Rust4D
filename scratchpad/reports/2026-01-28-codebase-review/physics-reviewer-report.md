# Physics Reviewer Report

## Summary

The `rust4d_physics` crate is well-implemented with comprehensive collision detection, rigid body dynamics, and material systems. All 110 tests pass. However, there is one significant dead code issue: the `PlayerPhysics` module is completely unused now that player physics has been integrated into `PhysicsWorld`. Additionally, the collision filtering system (layers/masks) is implemented but not used in actual game logic.

## Dead Code

| Item | Location | Type | Notes |
|------|----------|------|-------|
| `PlayerPhysics` struct | `crates/rust4d_physics/src/player.rs` | Entire module | Was replaced by player integration into PhysicsWorld. Not used in main.rs, examples, or core crate |
| `DEFAULT_PLAYER_RADIUS` | `crates/rust4d_physics/src/player.rs:10` | Constant | Only used within dead PlayerPhysics module |
| `DEFAULT_JUMP_VELOCITY` | `crates/rust4d_physics/src/player.rs:13` | Constant | Only used within dead PlayerPhysics module |
| `pub mod player` | `crates/rust4d_physics/src/lib.rs:12` | Module export | Exports dead module |
| `pub use player::*` | `crates/rust4d_physics/src/lib.rs:20` | Re-export | Re-exports dead types |

### Evidence

- `grep -r "PlayerPhysics" src/` - No matches
- `grep -r "PlayerPhysics" crates/rust4d_core/` - No matches
- `grep -r "PlayerPhysics" examples/` - No matches
- Only references are in:
  - The player.rs module itself (tests)
  - lib.rs (re-exports)
  - Archive scratchpad reports documenting its original implementation

### Historical Context

The `PlayerPhysics` module was originally a separate player physics system. It was superseded when player physics was integrated into `PhysicsWorld` via:
- `PhysicsWorld::set_player_body()`
- `PhysicsWorld::player_jump()`
- `PhysicsWorld::apply_player_movement()`
- `PhysicsWorld::player_is_grounded()`

The old module should be removed or deprecated.

## Implementation Gaps

| Feature | Status | Description |
|---------|--------|-------------|
| Collision Filtering Usage | Implemented but not used | `CollisionFilter` and `CollisionLayer` are fully implemented (7 layers: DEFAULT, PLAYER, ENEMY, STATIC, TRIGGER, PROJECTILE, PICKUP) but not used anywhere in main.rs or rust4d_core |
| ColliderTemplate | Not implemented | Plans mention `ColliderTemplate` enum but it doesn't exist. Physics colliders are created from entity tags ("static" + Hyperplane = floor) |
| BoundedFloor in templates | Gap | `StaticCollider::floor_bounded()` exists but no corresponding scene template variant |
| Sphere collider for entities | Not connected | `RigidBody4D::new_sphere()` exists but scene loading only creates AABB colliders from Tesseract shapes |

### Collision Filter Layer Usage

The following layers are defined but never used in production code:
- `CollisionLayer::PLAYER` - Only in tests
- `CollisionLayer::ENEMY` - Only in tests
- `CollisionLayer::TRIGGER` - Only in tests
- `CollisionLayer::PROJECTILE` - Only in tests
- `CollisionLayer::PICKUP` - Only in tests

All production physics uses `CollisionFilter::default()` (DEFAULT layer, ALL mask) and `CollisionFilter::static_world()` (STATIC layer, ALL mask).

## Code Quality Issues

| Issue | Location | Severity | Description |
|-------|----------|----------|-------------|
| No TODO/FIXME markers | Throughout | Info | Codebase is clean with no unfinished markers |
| No compiler warnings | Throughout | Info | All code compiles cleanly (warning from rust4d_math is separate) |
| Comprehensive tests | Throughout | Good | 110 tests covering all modules |
| Documentation complete | Throughout | Good | All public APIs have doc comments |

### Positive Findings

1. **Clean implementation**: No TODOs, FIXMEs, or `unimplemented!()` macros found
2. **Strong test coverage**: 110 tests across all modules
3. **Well-structured collision system**: Sphere, AABB, and Plane colliders with all pairwise collision detection implemented
4. **Material system complete**: Friction and restitution with geometric mean/max combination
5. **Collision layers ready**: Full layer/mask filtering system ready for future gameplay features
6. **Edge falling works**: Bounded floors properly handle players falling off 4D edges

## PhysicsConfig Analysis

Current fields:
- `gravity: f32` - Connected and used
- `jump_velocity: f32` - Connected and used

No missing config fields identified. The config is minimal but complete for current needs.

## Recommendations

### High Priority
1. **Remove or deprecate `player.rs` module** - The entire module is dead code. Either:
   - Remove it completely (breaking change for any external users)
   - Mark with `#[deprecated]` attributes and remove re-exports from lib.rs

### Medium Priority
2. **Utilize collision filtering** - The layer/mask system is fully implemented but unused. Future gameplay features (enemies, projectiles, pickups) should use it
3. **Add ColliderTemplate to scene system** - Currently physics colliders are created from tags, which is fragile

### Low Priority
4. **Consider sphere colliders for entities** - Currently all dynamic entities use AABB colliders. Sphere colliders might be more appropriate for some shapes
5. **Document collision layer usage** - Add examples in user-guide.md showing how to use collision layers for gameplay

## Cross-Cutting Issues

### For Testing Reviewer
- Physics tests are well-isolated with 110 passing tests
- Integration tests in `rust4d_core/tests/physics_integration.rs` are comprehensive

### For Core Reviewer
- Scene loading creates physics colliders from entity tags (fragile coupling)
- No ColliderTemplate in scene system - physics shape is inferred from visual shape

### For Roadmap Reviewer
- Phase 1 plans mention `ColliderTemplate` that doesn't exist
- `BoundedFloor` collider type exists but not in any template system

## Files Reviewed

- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/lib.rs` - Module exports
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/world.rs` - PhysicsWorld, PhysicsConfig
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/body.rs` - RigidBody4D, StaticCollider, BodyType
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/collision.rs` - Collision detection, filters
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/shapes.rs` - Sphere4D, AABB4D, Plane4D, Collider enum
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/material.rs` - PhysicsMaterial
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_physics/src/player.rs` - DEAD CODE: PlayerPhysics
