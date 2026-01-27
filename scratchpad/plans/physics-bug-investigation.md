# Physics Bug Investigation Plan

**Date:** 2026-01-27
**Issue:** Tesseract in `scenes/default.ron` does not fall to the floor
**Based on:** Physics Review Swarm findings (6 agents, unanimous conclusion)

---

## Executive Summary

The physics review swarm analyzed all components of the pipeline and found **every component individually correct**. The bug wasn't found through static analysis, which means either:

1. The bug exists in component interactions (integration issue)
2. The bug is runtime-only (timing, state, or configuration)
3. The bug doesn't exist (perception issue - camera, speed, etc.)

**Strategy:** Create integration tests that exercise the full pipeline. These tests will either:
- **Pass** → Proving the code works and the bug is elsewhere (camera, perception)
- **Fail** → Revealing exactly where the pipeline breaks

---

## Phase 1: Integration Tests (Critical)

**Goal:** Create tests that exercise the full scene-physics-render pipeline

**Sessions:** 1-2

### Task 1.1: Create Integration Test File

Create `crates/rust4d_core/tests/integration.rs` with the following tests:

```rust
// Test: Dynamic entity with physics body falls under gravity
#[test]
fn test_scene_dynamic_entity_falls_to_floor() {
    // Load default scene
    // Step physics for 120 frames (2 seconds at 60fps)
    // Assert tesseract Y position decreased from 0.0
    // Assert tesseract Y position is near floor (-1.0 to -1.1)
}

// Test: Dynamic entities get physics bodies assigned
#[test]
fn test_scene_dynamic_entity_has_physics_body() {
    // Load scene with "dynamic" tagged entity
    // Assert entity.physics_body.is_some()
    // Assert body type is Dynamic
}

// Test: Entity transform syncs from physics body
#[test]
fn test_entity_transform_syncs_from_physics() {
    // Create entity with physics body
    // Step physics once
    // Assert entity.transform.position == body.position
    // Assert entity dirty flag is set
}

// Test: Bounded floor AABB stops falling body
#[test]
fn test_bounded_floor_stops_falling_body() {
    // Create PhysicsWorld with bounded floor at y=-2
    // Add dynamic AABB body at y=0
    // Step until grounded
    // Assert body.grounded == true
    // Assert body position.y is approximately -1.0 (bottom at -2)
}
```

### Task 1.2: Add Test Helpers

May need to expose some internal APIs for testing:
- `PhysicsWorld::get_body()` - Already exists
- `World::get_entity_by_name()` - May need to add
- `ActiveScene::from_template()` - Already exists

---

## Phase 2: Debug Logging (High Priority)

**Goal:** Add optional tracing to observe runtime behavior

**Sessions:** 0.5-1

### Task 2.1: Physics Step Logging

Add to `crates/rust4d_physics/src/world.rs`:

```rust
// In PhysicsWorld::step()
log::debug!("Physics step: {} bodies, dt={:.4}", self.bodies.len(), dt);
for (key, body) in &self.bodies {
    if body.body_type == BodyType::Dynamic {
        log::trace!("Body {:?}: pos={:?}, vel={:?}, grounded={}",
            key, body.position, body.velocity, body.grounded);
    }
}
```

### Task 2.2: Entity Sync Logging

Add to `crates/rust4d_core/src/world.rs`:

```rust
// In World::update()
log::debug!("Syncing {} entities with physics", count);
// In the sync loop:
log::trace!("Entity {:?}: {:?} -> {:?}",
    entity.name, old_pos, body.position);
```

### Task 2.3: Main Loop Logging

Add to `src/main.rs`:

```rust
// In RedrawRequested handler
log::debug!("Frame: dirty={}, dt={:.4}", has_dirty, dt);
```

---

## Phase 3: Diagnostic Test Run

**Goal:** Run the application with logging to observe actual behavior

**Sessions:** 0.5

### Task 3.1: Run with RUST_LOG=debug

```bash
RUST_LOG=rust4d=debug cargo run
```

Observe:
- [ ] "Physics step: N bodies" appears each frame
- [ ] Tesseract body position Y decreasing
- [ ] Entity sync messages showing position changes
- [ ] "dirty=true" appearing when tesseract moves

### Task 3.2: Check for Specific Failure Modes

| Symptom | Likely Cause |
|---------|--------------|
| "Physics step: 0 bodies" | Physics bodies not created |
| "Physics step: 1 body" but no velocity | Gravity not being applied |
| Body falling but entity not syncing | Sync code not running |
| Entity syncing but geometry not rebuilding | Dirty flag not triggering rebuild |
| Geometry rebuilding but no visual change | Camera position or rendering issue |

---

## Phase 4: Hypothesis Testing

Based on swarm findings, test these hypotheses:

### Hypothesis A: First Frame Issue

The tesseract might be at the floor already but the initial geometry was built before physics stepped.

**Test:** Add a small delay or force rebuild after first physics step.

### Hypothesis B: Delta Time Issue

If `dt` is 0 or very small on early frames, nothing moves.

**Test:** Log `dt` values, verify they are reasonable (~0.016 for 60fps).

### Hypothesis C: Scene Not Loading

A different scene or no scene might be loading.

**Test:** Add log when scene loads, verify "default.ron" is being loaded.

### Hypothesis D: Perception Issue

The tesseract might be falling correctly but:
- Too fast to see
- Camera not positioned to see it
- Falls through floor due to tunneling

**Test:** Start tesseract at y=5 instead of y=0, observe for longer.

---

## Phase 5: Fix Implementation

Once the failing test or log reveals the issue, implement the fix.

**Sessions:** 1-2 (depends on what's found)

### If physics bodies not created:
- Check tag detection in `from_template()`
- Verify `is_dynamic` branch is reached

### If gravity not applied:
- Check `affected_by_gravity()` return value
- Check `self.config.gravity` value

### If sync not working:
- Check `entity.physics_body` is `Some`
- Check body key is valid

### If dirty flag not set:
- Check position comparison logic
- Add epsilon-based comparison

### If geometry not rebuilding:
- Check `has_dirty_entities()` implementation
- Verify `build_geometry()` uses updated transforms

---

## File Changes Summary

| File | Change | Phase |
|------|--------|-------|
| `crates/rust4d_core/tests/integration.rs` (NEW) | Integration tests | 1 |
| `crates/rust4d_physics/src/world.rs` | Debug logging | 2 |
| `crates/rust4d_core/src/world.rs` | Debug logging | 2 |
| `src/main.rs` | Frame logging | 2 |
| TBD based on findings | Bug fix | 5 |

---

## Execution Order

```
Wave 1 (Sequential - Tests First)
├── Task 1.1: Create integration test file
└── Task 1.2: Add test helpers if needed

Wave 2 (Parallel - Diagnostics)
├── Agent 1: Add physics logging (Task 2.1)
├── Agent 2: Add entity sync logging (Task 2.2)
└── Agent 3: Add main loop logging (Task 2.3)

Wave 3 (Sequential - Investigation)
└── Task 3.1-3.2: Run diagnostics, analyze output

Wave 4 (Sequential - Fix)
└── Task 5.x: Implement fix based on findings
```

---

## Success Criteria

1. All new integration tests pass
2. Tesseract visually falls from y=0 to rest at y=-1 (bottom at floor y=-2)
3. No console errors or warnings during simulation
4. Frame rate remains stable (60fps)

---

## Notes

- The swarm verified all individual components work correctly
- The bug is almost certainly in the **integration** between components
- Integration tests are the fastest way to find the exact failure point
- Once a test fails, we have a reproducible case to debug

---

## Estimated Sessions

| Phase | Sessions | Notes |
|-------|----------|-------|
| 1: Integration Tests | 1-2 | May reveal bug immediately |
| 2: Debug Logging | 0.5-1 | Quick additions |
| 3: Diagnostic Run | 0.5 | Observe behavior |
| 4: Hypothesis Testing | 0.5-1 | If needed |
| 5: Fix Implementation | 1-2 | Depends on what's found |
| **Total** | **3-6** | Could be faster if tests reveal issue quickly |
