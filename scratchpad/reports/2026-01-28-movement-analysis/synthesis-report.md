# Movement System Analysis - Synthesis Report

**Date:** 2026-01-28
**Task:** Analyze why 4D movement doesn't rotate with player orientation
**Status:** Analysis complete, fix plan ready

## Swarm Status

| Agent | Focus | Status |
|-------|-------|--------|
| Movement Agent | Input handling, movement vectors | Complete |
| Camera Agent | Rotation system, 4D orientation | Complete |
| Coordinate Agent | Coordinate spaces, transforms | Complete |

---

## Executive Summary

All three agents converged on the same root cause: **W-axis movement (Q/E keys) uses a hardcoded world vector instead of the camera's transformed W direction**.

The fix is straightforward - use `camera.ana()` instead of `Vec4::W` at line 324 of `src/main.rs`.

---

## Root Cause Analysis

### The Bug Location

**File:** `src/main.rs`, line 324

```rust
// Current (BROKEN):
let move_dir = forward_xz * forward_input + right_xz * right_input
    + Vec4::W * w_input;  // Always world +W/-W regardless of rotation!
```

### Why This Is Wrong

The code correctly transforms XZ movement:
- `forward_xz` is derived from `camera.forward()` which IS transformed by the camera matrix
- `right_xz` is derived from `camera.right()` which IS transformed by the camera matrix

But W movement uses a constant:
- `Vec4::W` is always `(0, 0, 0, 1)` - the world W axis
- When the player rotates in 4D, the world W axis doesn't change
- The camera's "ana" direction DOES change with 4D rotation

### The Fix

```rust
// Fixed:
let camera_ana = self.camera.ana();  // W direction transformed by camera matrix
let ana_xzw = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();

let move_dir = forward_xz * forward_input + right_xz * right_input
    + ana_xzw * w_input;  // Now rotates with camera!
```

Or more simply, since `ana()` already returns the correctly transformed direction:
```rust
let move_dir = forward_xz * forward_input + right_xz * right_input
    + self.camera.ana() * w_input;
```

---

## Agent Findings Summary

### Movement Agent Key Finding

> "The W-axis input is NOT transformed by camera orientation... `Vec4::W` is always `(0, 0, 0, 1)` - it's the world W axis, not the camera's W axis. When the camera rotates in 4D, the camera's local W direction changes, but the input still uses the world W."

**Files Analyzed:**
- `src/main.rs` lines 309-330 (movement vector construction)
- `crates/rust4d_input/src/camera_controller.rs` lines 87-108 (input handling)
- `crates/rust4d_physics/src/world.rs` lines 149-156 (physics application)

### Camera Agent Key Finding

> "The camera has a fully functional `ana()` method (camera4d.rs line 216) that correctly applies the camera matrix to the W basis vector. This method already exists and can be used to fix the movement issue."

**Architecture Documented:**
- Rotor4 for 4D rotations (geometric algebra)
- SkipY transformation preserves Y axis (gravity)
- Camera matrix correctly combines pitch + 4D rotation
- `forward()`, `right()`, `up()`, `ana()` all transform correctly

### Coordinate Agent Key Finding

> "The engine's math is sound... Camera has full orientation info, but it's not integrated into the physics system. The mathematics for the fix already exist in the codebase - they just need to be applied."

**Also Noted:**
- `RigidBody4D` lacks rotation field (potential future enhancement)
- Physics movement applied directly without transform
- One-way sync (position only, not rotation)

---

## The Existing `ana()` Method

From `crates/rust4d_render/src/camera4d.rs` lines 215-218:

```rust
/// Get the W (ana) direction vector
pub fn ana(&self) -> Vec4 {
    mat4::transform(self.camera_matrix(), Vec4::new(0.0, 0.0, 0.0, 1.0))
}
```

This method:
1. Gets the full camera matrix (includes SkipY and pitch transforms)
2. Transforms the W basis vector `(0, 0, 0, 1)` by this matrix
3. Returns the camera-relative "ana" direction

This is **exactly what we need** for W-axis movement.

---

## Fix Plan

### Phase 1: Minimal Fix (1 session)

**Change in `src/main.rs` line 322-324:**

```rust
// Before:
let move_dir = forward_xz * forward_input + right_xz * right_input
    + Vec4::W * w_input;

// After:
let camera_ana = self.camera.ana();
let move_dir = forward_xz * forward_input + right_xz * right_input
    + camera_ana * w_input;
```

**Why This Works:**
- `camera.ana()` returns the W direction vector transformed by the camera's full rotation
- When the player rotates in 4D, `ana()` returns a different direction
- Movement will now follow the rotated W axis

### Phase 2: Consistency Check (Optional, 0.5 sessions)

Consider whether `ana()` needs Y-component zeroing like `forward_xz` and `right_xz`:

```rust
let ana_horizontal = Vec4::new(camera_ana.x, 0.0, camera_ana.z, camera_ana.w).normalized();
```

This depends on the desired behavior:
- **With Y zeroing:** W-movement stays horizontal even when looking up/down
- **Without Y zeroing:** W-movement follows the full camera orientation

The current forward/right behavior zeroes Y, so for consistency, W should probably do the same.

### Phase 3: Testing

Verify behavior:
1. Default orientation: Q/E moves along world +W/-W (unchanged)
2. After 4D rotation: Q/E moves along rotated W axis (fixed!)
3. Looking up/down: Q/E stays horizontal (if Y zeroing applied)

---

## Architecture Insights

The codebase follows **Engine4D-style architecture** with SkipY:

```
Camera Orientation = SkipY(rotation_4d) × pitch_rotation

Where:
- rotation_4d operates in XZW hyperplane
- SkipY remaps: X→X, Y→Z, Z→W (leaves Y as identity)
- pitch operates in YZ plane (separate)

Result: Y axis ALWAYS preserved (gravity-aligned)
```

This design is intentional and correct. The fix maintains this architecture.

---

## Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `src/main.rs` | 322-324 | **THE BUG** - W movement uses hardcoded axis |
| `crates/rust4d_render/src/camera4d.rs` | 215-218 | `ana()` method - the solution |
| `crates/rust4d_render/src/camera4d.rs` | 70-81 | Camera matrix construction |
| `crates/rust4d_math/src/mat4.rs` | 49-106 | SkipY transformation |
| `crates/rust4d_input/src/camera_controller.rs` | 218-235 | W input handling |

---

## Conclusion

This is a **one-line fix** once you understand the architecture. The camera already has the correct `ana()` method that returns the transformed W direction. The movement code just isn't using it.

**Estimated effort:** 0.5-1 session to implement and test.
