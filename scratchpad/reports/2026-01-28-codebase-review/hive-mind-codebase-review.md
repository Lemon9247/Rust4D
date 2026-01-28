# Hive Mind: Comprehensive Codebase Review

## Task Overview
Perform an extensive codebase review to identify existing shortcomings, dead code, and implementation gaps. This review builds on findings from recent session reports (config consolidation, physics review, wave planning) and aims to produce a definitive inventory of issues across all subsystems.

## Context from Recent Reports

### From Physics Review (2026-01-27)
- 180+ unit tests but previously lacked integration tests (now added)
- `ColliderTemplate` doesn't include `BoundedFloor`
- Tesseract uses sphere collider approximation in plans vs AABB in code

### From Config Consolidation (2026-01-28)
- Connected config values to physics, rendering, camera
- Removed duplicate physics.player_radius
- Open: `camera.pitch_limit` not connected (hardcoded)
- `physics.floor_y` was unused and removed
- `rendering.max_triangles` connected (was 10x mismatch)
- Note: GPU buffer limit clamping added

### From Wave Planning (2026-01-28)
- 319 tests total, 1 failing (test_env_override in config)
- EntityTemplate provides prefab-like functionality
- Phase 3B documentation not started
- Phase 4+ architecture refactoring not started

### From Config Follow-up Fixes (2026-01-28)
- `thickness` field in `Hyperplane4D` is never read (dead code)
- Vulkan validation errors about atomic operations in shaders
- GPU buffer size calculation: max_triangles × 3 × 48 bytes

## Agents

1. **Core Reviewer** - Reviews rust4d_core crate: World, Entity, Transform4D, Scene system
2. **Input Reviewer** - Reviews input handling, mouse/keyboard, controller systems
3. **Math Reviewer** - Reviews rust4d_math crate: vectors, matrices, 4D geometry, hyperplanes
4. **Physics Reviewer** - Reviews rust4d_physics crate: collision, rigid bodies, world simulation
5. **Render Reviewer** - Reviews rust4d_render crate: pipelines, shaders, geometry, GPU buffers
6. **Configuration and Architecture Reviewer** - Reviews config system, main.rs, overall structure
7. **Testing Reviewer** - Reviews test coverage, test quality, missing tests
8. **Roadmap Reviewer** - Reviews roadmap vs implementation, missing features, planned vs done

## Coordination Notes

- Each agent writes findings to this folder as `<agent-name>-report.md`
- Focus on: dead code, unused fields/functions, unconnected config, missing implementations
- Look for: compiler warnings, TODO comments, FIXME comments, unimplemented!() macros
- Cross-reference with other agents if you find issues that span systems
- Add discovered cross-cutting issues to "Questions for Discussion" section below

## Questions for Discussion
(Agents can add questions here - other agents should check this section and respond)

### Input Reviewer Findings (cross-cutting)
1. **Config not connected**: `input.w_rotation_sensitivity` is in config but NOT connected to CameraController (no `with_w_rotation_sensitivity` builder method exists)
2. **Config not connected**: `camera.pitch_limit` is in config but Camera4D uses hardcoded `PITCH_LIMIT` constant
3. **Debug config unused**: `debug.show_overlay`, `debug.show_colliders`, `debug.log_level` are loaded but never used in main.rs

### Roadmap Reviewer Findings (cross-cutting)
1. **Phase 3B status wrong**: Listed as "NOT STARTED" but `docs/` directory exists with all 4 guides complete
2. **Prefab plan vs reality**: Phase 2B describes full `Prefab` struct with overrides that doesn't exist - only EntityTemplate
3. **ColliderTemplate doesn't exist**: Plans reference ColliderTemplate but only ShapeTemplate exists; physics colliders created from tags
4. **README needs update**: "Scene serialization" still listed as "in progress" but is complete

### Configuration and Architecture Reviewer Findings (cross-cutting)
1. **7 unused config values identified**: `camera.pitch_limit`, `window.fullscreen`, `window.vsync`, `input.w_rotation_sensitivity`, `debug.show_overlay`, `debug.log_level`, `debug.show_colliders`
2. **rust4d_render depends on rust4d_input**: Unusual coupling via CameraControl trait - consider moving trait to rust4d_math
3. **ARCHITECTURE.md incomplete**: Doesn't show render->input dependency
4. **test_env_override flaky**: Fails when run with other tests due to shared environment state
5. **main.rs god object**: App struct holds all state, game loop is 300+ lines in single method

### Testing Reviewer Findings (cross-cutting)
1. **326 total tests**: 325 pass reliably, 1 flaky (`test_env_override`)
2. **Zero tests for rust4d_input**: CameraController's input smoothing, key handling, movement calculation all untested
3. **Zero tests for render context**: wgpu initialization code untested (may need mocking)
4. **Test warnings present**: Unused imports/variables in test modules create noise
5. **Soft skip pattern**: `test_load_default_scene_file` silently returns instead of using `#[ignore]`

### Core Reviewer Findings (cross-cutting)
1. **thickness field in Hyperplane4D**: Compiler warning is from rust4d_math. Field IS used in constructor but stored value never accessed after construction. Math Reviewer should verify if accessor is needed.
2. **Orphaned physics bodies**: When `World::remove_entity` is called, associated physics bodies are NOT removed from PhysicsWorld. Physics Reviewer should address.
3. **ShapeTemplate limited**: Only supports Tesseract and Hyperplane. No mechanism for custom shapes or other primitives.
4. **Doc-test ignored**: `scene_manager.rs:9` has an ignored doc-test that should be fixed or removed.

### Physics Reviewer Findings (cross-cutting)
1. **Dead module**: The entire `player.rs` module (PlayerPhysics struct, DEFAULT_PLAYER_RADIUS, DEFAULT_JUMP_VELOCITY) is dead code - player physics was integrated into PhysicsWorld
2. **Collision layers unused in production**: CollisionFilter/CollisionLayer system is fully implemented (7 layers) but only used in tests, not in main.rs or rust4d_core
3. **No ColliderTemplate**: Plans reference ColliderTemplate but it doesn't exist; colliders are created from entity tags which is fragile
4. **Orphaned physics bodies response**: CONFIRMED - World::remove_entity does not clean up PhysicsWorld. This is a bug.

## Status
- [x] Core Reviewer: Complete
- [x] Input Reviewer: Complete
- [x] Math Reviewer: Complete
- [x] Physics Reviewer: Complete
- [x] Render Reviewer: Complete
- [x] Configuration and Architecture Reviewer: Complete
- [x] Testing Reviewer: Complete
- [x] Roadmap Reviewer: Complete
- [x] Final synthesis: Complete

## Reports Generated
- `core-reviewer-report.md` - Core crate review complete
- `input-reviewer-report.md` - Input handling review complete
- `math-reviewer-report.md` - Math crate review complete
- `physics-reviewer-report.md` - Physics crate review complete
- `render-reviewer-report.md` - Render crate review complete
- `config-architecture-reviewer-report.md` - Config and architecture review complete
- `testing-reviewer-report.md` - Test suite review complete
- `roadmap-reviewer-report.md` - Roadmap vs implementation review complete
- `final-synthesis-report.md` - Combined findings and action items

## Key Findings
(Summarize major discoveries as they emerge)

### From Config/Architecture Review
- **7 config values not connected** to code (see report for full list)
- **main.rs is a god object** - App struct owns everything, game loop is monolithic
- **Crate dependency issue**: rust4d_render -> rust4d_input for CameraControl trait
- **Test isolation issue**: test_env_override pollutes shared environment

## Known Issues to Verify
These issues were identified in previous reports - verify current status:

1. **Dead Code**
   - [x] `thickness` field in `Hyperplane4D` (never read) - CONFIRMED, compiler warning present
   - [x] `player.rs` module in rust4d_physics - CONFIRMED, entire module unused (PlayerPhysics replaced by PhysicsWorld player integration)

2. **Shader Issues**
   - [ ] Vulkan validation errors in `slice.wgsl` and `slice_tetra.wgsl` (atomic operations)

3. **Config Gaps**
   - [x] `camera.pitch_limit` not connected to Camera4D - CONFIRMED, hardcoded at 89 degrees
   - [x] `window.fullscreen` not applied on startup - CONFIRMED
   - [x] `window.vsync` not connected to wgpu - CONFIRMED
   - [x] `input.w_rotation_sensitivity` not passed to controller - CONFIRMED
   - [x] `debug.*` values loaded but features not implemented - CONFIRMED

4. **Test Issues**
   - [x] `test_env_override` failing - CONFIRMED (flaky due to env var pollution between parallel tests)
   - [x] Integration test coverage gaps - REVIEWED: 10 integration tests in physics_integration.rs; some tests have soft skips
   - [x] `rust4d_input` crate has ZERO unit tests
   - [x] `rust4d_render/context.rs` has ZERO unit tests
   - [x] 1 doc test ignored without explanation (scene_manager.rs:9)

5. **Template/Serialization Gaps**
   - [x] `ColliderTemplate` doesn't exist at all - CONFIRMED: only ShapeTemplate exists, physics colliders created from tags
   - [x] `BoundedFloor` has no template - CONFIRMED: StaticCollider::floor_bounded() exists but no scene template variant

6. **Physics Cleanup Issues**
   - [x] Orphaned physics bodies - CONFIRMED: World::remove_entity doesn't clean up PhysicsWorld bodies
   - [x] Collision filtering unused - CONFIRMED: 7 collision layers implemented but only DEFAULT and STATIC used in production
