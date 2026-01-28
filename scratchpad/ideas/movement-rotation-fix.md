# Movement System Rotation Fix

**Date:** 2026-01-28
**Status:** Analysis in progress

## Problem Statement

The movement system does not correctly rotate movement directions when the player rotates in 4D.

### Expected Behavior

1) When the player is facing such that the W-axis is rotated orthogonal to their current 3D slice, the movement controls should work as:
   - W/S: Move the player forwards and backwards along x
   - A/D: Move the player left and right along z
   - Q/E: Move the player ana and kata along w

2) When the player is rotated in the 4th dimension, the axes of movement for the player should be similarly rotated. So forwards/left/ana should become a combination of 3D and 4D motion vectors.

### Current Behavior

When rotating the camera and pressing movement keys, the coordinates in the titlebar do not reflect the expected behavior. Instead, pressing each key always affects a specific coordinate (e.g., Q/E always affects W coordinate, even after 4D rotation).

## Analysis

**Analysis Date:** 2026-01-28
**Method:** Swarm analysis with 3 agents (Movement, Camera, Coordinate)

### Root Cause

W-axis movement (Q/E keys) uses hardcoded `Vec4::W` constant instead of the camera's transformed W direction.

**Bug Location:** `src/main.rs` line 324

```rust
// Current (broken):
let move_dir = forward_xz * forward_input + right_xz * right_input
    + Vec4::W * w_input;  // Always (0,0,0,1) regardless of rotation!
```

The XZ movement is correctly transformed using `camera.forward()` and `camera.right()`, but W movement ignores the camera's 4D orientation entirely.

### Why It's Wrong

- `Vec4::W` is always `(0, 0, 0, 1)` - the world W axis
- When the player rotates in 4D, `Vec4::W` doesn't change
- The camera's `ana()` method DOES return the correctly rotated W direction
- But the movement code doesn't use it

## Proposed Solution

Use `camera.ana()` for W-axis movement:

```rust
// Fixed:
let camera_ana = self.camera.ana();
let move_dir = forward_xz * forward_input + right_xz * right_input
    + camera_ana * w_input;  // Now follows camera's 4D rotation!
```

The `ana()` method already exists at `camera4d.rs:215-218` and correctly transforms the W basis vector through the full camera matrix.

### Considerations

1. **Y-component zeroing:** Consider whether `ana()` should have its Y component zeroed like `forward_xz` and `right_xz` to keep W-movement horizontal.

2. **Physics integration:** Currently physics has no player rotation. This fix works by using camera rotation for movement direction, which is the correct approach for now.

**Estimated effort:** 0.5-1 session

**Full analysis:** See `scratchpad/reports/2026-01-28-movement-analysis/synthesis-report.md`
