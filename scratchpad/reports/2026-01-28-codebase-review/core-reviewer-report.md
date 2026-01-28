# Core Reviewer Report

## Summary

The `rust4d_core` crate is well-structured and in good health. It provides foundational types for the Rust4D engine: Transform4D, Entity, World, Scene, and SceneManager. The crate has 90 unit tests and 10 integration tests, all passing. No TODO/FIXME comments or `unimplemented!()` macros were found. The code quality is high with proper error handling, clear abstractions, and good documentation.

Key findings:
- **No dead code within rust4d_core itself** - all public items are used
- **Limited ShapeTemplate variants** - only Tesseract and Hyperplane supported
- **Re-exports from rust4d_physics** - BodyKey, PhysicsConfig, PhysicsWorld, RigidBody4D, StaticCollider are re-exported but usage varies
- The `y` field in `ShapeTemplate::Hyperplane` has a dual-purpose (visual mesh position vs physics collider) that may cause confusion

## Dead Code

| Item | Location | Type | Notes |
|------|----------|------|-------|
| None | - | - | No dead code found in rust4d_core crate |

**Note on thickness field**: The hive-mind mentions `thickness` field in `Hyperplane4D` as dead code. This field is in `rust4d_math`, not `rust4d_core`. I verified the field IS used in the constructor (`y1 = thickness` at line 75 of hyperplane.rs), but the compiler warning suggests it's stored but never read AFTER construction. This is a rust4d_math issue for the Math Reviewer.

## Implementation Gaps

| Item | Location | Description |
|------|----------|-------------|
| Limited ShapeTemplate | `shapes.rs:22-47` | Only Tesseract and Hyperplane variants; no support for custom shapes or other primitives |
| No entity hierarchy | `entity.rs` | Entities have no parent/child relationships for transform inheritance |
| No entity deletion callback | `world.rs:89-100` | When entities are removed, there's no mechanism to clean up associated physics bodies |
| SceneManager doc-test ignored | `scene_manager.rs:9` | One doc-test is marked `ignore` |

## Code Quality Issues

| Issue | Location | Severity | Description |
|-------|----------|----------|-------------|
| Orphaned physics body | `world.rs:89-100` | Medium | `remove_entity` doesn't clean up the physics body reference - orphaned bodies may remain in PhysicsWorld |
| Potential confusion | `shapes.rs:36-46` | Low | The `y` field in Hyperplane template is for physics only, not visual mesh - documented but easy to misunderstand |
| Clone on `PhysicsConfig` | `scene.rs:118` | Low | `default_physics.clone()` in `SceneManager::instantiate` - could be `Copy` instead |

## Detailed Analysis

### Files Reviewed

1. **lib.rs** (35 lines) - Clean module exports and re-exports from rust4d_math and rust4d_physics
2. **transform.rs** (262 lines) - Well-implemented Transform4D with position, rotation (Rotor4), scale. Custom serde for Rotor4.
3. **entity.rs** (519 lines) - Entity struct with DirtyFlags, Material, ShapeRef (shared/owned). EntityTemplate for serialization.
4. **world.rs** (700 lines) - SlotMap-based entity storage with name index, physics integration, dirty tracking
5. **scene.rs** (612 lines) - Scene template loading/saving, ActiveScene runtime instantiation
6. **scene_manager.rs** (473 lines) - Scene stack for overlays, template management
7. **shapes.rs** (130 lines) - ShapeTemplate enum for serializable shape construction

### Test Coverage

- **90 unit tests** in the crate
- **10 integration tests** in `tests/physics_integration.rs`
- All tests passing
- Doc-tests: 1 test marked `ignore` (SceneManager usage example)

### Compiler Warnings

Running `cargo check -p rust4d_core` produces:
```
warning: field `thickness` is never read
  --> crates/rust4d_math/src/hyperplane.rs:33:5
```

This warning is from **rust4d_math**, not rust4d_core. The core crate itself has no compiler warnings.

### Re-exports Analysis

The crate re-exports from rust4d_math and rust4d_physics:

From `rust4d_math`:
- `Vec4, Rotor4, RotationPlane, ConvexShape4D, Tetrahedron` - **Used**
- `Tesseract4D, Hyperplane4D` - **Used**

From `rust4d_physics`:
- `BodyKey` - **Used** (entity.physics_body field)
- `PhysicsConfig` - **Used** (World::with_physics)
- `PhysicsWorld` - **Used** (World.physics_world field)
- `RigidBody4D` - **Used** (ActiveScene::from_template)
- `StaticCollider` - **Used** (ActiveScene::from_template)

All re-exports are actively used.

## Recommendations

1. **Add physics body cleanup on entity removal**
   - When `World::remove_entity` is called, if the entity has a `physics_body`, it should be removed from the PhysicsWorld
   - Currently leaves orphaned physics bodies

2. **Consider extending ShapeTemplate**
   - Add support for more primitive shapes (Sphere4D, etc.) if they exist
   - Or add a "Custom" variant for user-defined shapes

3. **Add entity hierarchy support** (future work)
   - Parent/child relationships for transform inheritance
   - Would enable complex composite objects

4. **Fix doc-test in scene_manager.rs**
   - The example at line 9 is marked `ignore`
   - Should either be made runnable or removed

5. **Document the y field behavior in Hyperplane template**
   - The current comment explains the physics/visual split but it's a common source of confusion
   - Consider renaming to `physics_y` or `collider_y` for clarity

## Cross-Cutting Issues for Other Reviewers

1. **Math Reviewer**: Verify the `thickness` field in `Hyperplane4D` - it's stored but the compiler says it's never read. The field is used during construction but may not be accessible after.

2. **Physics Reviewer**: When entities are removed from World, their physics bodies are NOT automatically removed from PhysicsWorld. This could lead to memory leaks or stale collisions.

3. **Physics Reviewer**: Check if `ColliderTemplate` or similar serialization support is needed for physics colliders.
