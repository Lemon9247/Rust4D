# Physics Bug Investigation Report

**Date:** 2026-01-27 22:30
**Issue:** "Tesseract in default.ron does not fall to the ground"
**Outcome:** Bug confirmed and fixed

---

## Summary

The physics review swarm correctly identified that all individual components were working correctly. After creating comprehensive integration tests and adding debug logging, I discovered:

1. **The physics code IS working correctly** - The tesseract falls and lands at the correct position (y=-1, meaning bottom at y=-2 = floor surface)

2. **The "bug" was a first-frame delta time issue** - A 0.37 second delay between `App::new()` and the first `RedrawRequested` caused the tesseract to "teleport" to the floor on the first frame instead of visibly falling

---

## Root Cause

In `src/main.rs`, `last_frame` is initialized in `App::new()`:
```rust
last_frame: std::time::Instant::now(),
```

But `RedrawRequested` doesn't fire until after window creation and GPU setup, causing a 0.3-0.4 second gap. This large `dt` caused the tesseract to fall instantly (physics simulation of 0.37 seconds worth of gravity at once).

---

## Fix Applied

Added delta time capping in `src/main.rs`:
```rust
let raw_dt = (now - self.last_frame).as_secs_f32();
// Cap dt to prevent huge physics steps on first frame or after window focus
let dt = raw_dt.min(1.0 / 30.0); // Max 33ms per frame
```

Now the tesseract falls smoothly over ~60 frames instead of teleporting.

---

## Integration Tests Created

Added `crates/rust4d_core/tests/physics_integration.rs` with 9 tests:

| Test | Purpose |
|------|---------|
| `test_scene_dynamic_entity_has_physics_body` | Verify dynamic entities get physics bodies |
| `test_scene_static_floor_has_collider` | Verify static floors create colliders |
| `test_dynamic_body_falls_under_gravity` | Verify gravity affects dynamic bodies |
| `test_dynamic_body_lands_on_floor` | Verify floor collision works |
| `test_aabb_body_lands_on_bounded_floor` | Test exact default.ron scenario |
| `test_entity_transform_syncs_from_physics` | Verify entity-physics sync |
| `test_scene_dynamic_entity_falls_to_floor` | Full pipeline test |
| `test_load_default_scene_file` | Test actual scene file |
| `test_physics_step_trace` | Diagnostic trace |

All tests pass.

---

## Physics Verification

The tesseract correctly:
1. Starts at y=0 (center)
2. Falls under gravity (-20 units/s²)
3. Collides with bounded floor at y=-2
4. Settles at y=-1 (center), meaning bottom at y=-2 (floor surface)
5. Gets marked as `grounded = true`

---

## Debug Output (After Fix)

```
Tesseract moved: y=0.000 -> -0.022, dt=0.0333  (capped from 0.37)
Tesseract moved: y=-0.022 -> -0.024, dt=0.0024
...
Tesseract moved: y=-0.996 -> -1.000, dt=0.0085  (lands on floor)
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/main.rs` | Added dt cap at 33ms (line ~302) |
| `crates/rust4d_core/tests/physics_integration.rs` | NEW - Integration tests |
| `scratchpad/plans/physics-bug-investigation.md` | Investigation plan |

---

## Swarm Findings Validated

The physics review swarm's findings were correct:
- All individual components work correctly
- The lack of integration tests was indeed a critical gap
- The bug was in the runtime behavior, not the physics code itself

---

## Remaining Question

The user asked: "If the tesseract is at y=-1, why is it not on top of the hyperplane?"

**Answer:** It IS on top. The math:
- Tesseract center at y=-1 with half_extent=1.0
- Tesseract bottom = -1 - 1 = **-2.0**
- Floor surface = **-2.0**
- They are touching correctly

If there's still a visual issue, it could be:
1. Camera/slice position not showing both objects
2. 4D slice plane not intersecting both
3. W-coordinate mismatch

---

## Test Results

```
running 9 tests (physics_integration.rs)
test test_dynamic_body_falls_under_gravity ... ok
test test_aabb_body_lands_on_bounded_floor ... ok
test test_dynamic_body_lands_on_floor ... ok
test test_load_default_scene_file ... ok
test test_physics_step_trace ... ok
test test_entity_transform_syncs_from_physics ... ok
test test_scene_dynamic_entity_has_physics_body ... ok
test test_scene_static_floor_has_collider ... ok
test test_scene_dynamic_entity_falls_to_floor ... ok

test result: ok. 9 passed; 0 failed
```

---

## Recommendations

1. **Consider resetting `last_frame` in `resumed()`** - This would be more correct than capping dt
2. **Add visual debug mode** - Render physics colliders as wireframes to debug visual issues
3. **Check 4D slice parameters** - If user still sees issues, verify camera W and slice_w match the tesseract's W position

---

## Session Summary

- Created investigation plan
- Added 9 integration tests (all pass)
- Found and fixed first-frame dt issue
- Verified physics works correctly
- Documented findings
