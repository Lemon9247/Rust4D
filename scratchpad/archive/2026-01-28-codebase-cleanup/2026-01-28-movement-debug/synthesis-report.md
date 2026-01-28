# Movement Debug - Synthesis Report

**Date:** 2026-01-28
**Task:** Investigate why using `camera.ana()` for W-axis movement still doesn't rotate with 4D orientation

## Swarm Status

| Agent | Focus | Status | Report |
|-------|-------|--------|--------|
| Camera Matrix Agent | ana() and camera_matrix() | Complete | camera-matrix-agent.md |
| Controller Agent | 4D rotation input handling | Complete | controller-agent.md |
| Movement Flow Agent | Full movement path | Complete | movement-flow-agent.md |
| SkipY Agent | SkipY transformation | Complete | skipy-agent.md |

---

## Executive Summary

**All agents agree the code is mathematically correct.** The SkipY transformation, rotor composition, and ana() method should all work as designed. After a 90° rotation via `rotate_w()`, the `ana()` method should return `(-1, 0, 0, 0)` instead of `(0, 0, 0, 1)`.

**Yet the user reports it doesn't work.** This means either:
1. The rotation isn't being applied correctly at runtime
2. The rotation magnitude is too small to notice
3. There's a bug we haven't found yet
4. The test case doesn't verify what we think it does

---

## Key Finding: Test Coverage Gap

The existing test `test_move_w_follows_camera_orientation` only verifies:
1. Initial W movement goes in +W
2. Y stays unchanged after rotation

**It does NOT verify that the movement direction actually changed!**

```rust
cam.rotate_w(FRAC_PI_2);
cam.move_w(1.0);
// Only checks Y is unchanged - never checks position.x or position.w changed!
assert!(cam.position.y.abs() < EPSILON, ...);
```

---

## Mathematical Verification: SkipY Works Correctly

The SkipY Agent traced through a 90° rotation:

1. `rotate_w(PI/2)` creates XZ plane rotation in rotor
2. SkipY transforms XZ → XW plane
3. Matrix transforms W basis (0,0,0,1) to (-1,0,0,0) or (1,0,0,0)
4. `ana()` returns this transformed direction

**The math is correct.** After rotation, ana() should return a direction with:
- W component ≈ 0
- X component ≈ ±1

---

## Possible Causes (To Investigate)

### 1. Rotor Not Accumulating
The rotor might be getting reset or not properly composing rotations.

### 2. Input Too Small
Mouse sensitivity might mean very small rotation deltas that don't visibly change direction.

### 3. Normalization Issue
The projection `Vec4::new(ana.x, 0.0, ana.z, ana.w).normalized()` might have edge cases.

### 4. Physics Override
Something in the physics system might be overriding or ignoring the W component.

---

## Investigation Plan

### Phase 1: Add Debug Output (0.5 sessions)

Add temporary debug output to main.rs to print:
1. `self.camera.ana()` value
2. `rotation_4d` rotor components
3. `ana_xzw` after projection
4. Final `move_dir` vector

This will reveal exactly what values are being computed at runtime.

### Phase 2: Add Comprehensive Test (0.5 sessions)

Create a test that verifies:
```rust
#[test]
fn test_ana_changes_after_4d_rotation() {
    let mut cam = Camera4D::new();

    let ana_before = cam.ana();
    assert!(ana_before.w > 0.9, "Initial ana should be ~(0,0,0,1)");

    cam.rotate_w(FRAC_PI_2);

    let ana_after = cam.ana();
    // THIS is what was missing - verify the direction actually changed!
    assert!(ana_after.w.abs() < 0.1, "After 90° rotation, W component should be ~0");
    assert!(ana_after.x.abs() > 0.9, "After 90° rotation, X component should be ~±1");
}
```

### Phase 3: Fix Based on Findings

Once we know what's actually happening at runtime, we can fix the root cause.

---

## Files Modified by Previous Fix

| File | Change |
|------|--------|
| `src/main.rs:322-328` | Added `camera.ana()` for W-axis movement |
| `scratchpad/ideas/physics-body-rotation.md` | Documented future enhancement |

---

## Recommendation

Before writing more code, **add debug output** to see what's actually happening at runtime:

```rust
// In main.rs, after line 324:
eprintln!("DEBUG: ana={:?}, ana_xzw={:?}", camera_ana, ana_xzw);
```

Run the game, perform a 4D rotation (right-click drag), then press Q/E and observe the debug output.

This will definitively show whether:
1. `ana()` is returning correct values
2. The projection is working
3. The direction is being applied correctly

---

## Conclusion

The code review suggests the implementation is correct, but the test coverage has a gap. We need runtime debugging to identify why the fix isn't working in practice.
