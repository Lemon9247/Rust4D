# Session Report: Camera Rotation System Overhaul

**Date:** 2026-01-26
**Duration:** ~2 sessions
**Focus:** Fixing 4D rotation mathematics and gimbal-lock-like behavior

---

## Summary

This session addressed two major issues in the 4D camera system:

1. **Rotor rotation formula was incorrect for composed rotors** - The explicit sandwich product formula had errors that caused non-orthogonal results when multiple rotations were composed.

2. **Camera used absolute Euler angles causing gimbal-lock-like behavior** - The system rebuilt orientation from scratch each frame using absolute angles, which caused unexpected behavior when combining 3D and 4D rotations.

---

## Part 1: Rotor4 Rotation Math Fix

### The Problem

The `rotate()` function in `rotor4.rs` used an explicit formula for the sandwich product `R v R̃`. This formula was derived for simple rotors (single plane rotations) but **failed for composed rotors** that have multiple non-zero bivector components.

Tests revealed:
- `test_composed_rotation_orthogonality` failed: X.Y = 0.5 instead of 0
- `test_multiple_rotation_composition` failed: vector length not preserved
- Sequential rotation gave different results than composed rotation

### The Root Cause

The explicit matrix-style formula assumed certain simplifications that don't hold when a rotor has multiple bivector components active simultaneously. The formula treated each bivector independently rather than accounting for their interactions through the trivector terms.

### The Solution

Rewrote `rotate()` to compute the sandwich product step-by-step:

1. **Compute R * v** - This produces both vector parts (e1, e2, e3, e4) and trivector parts (e123, e124, e134, e234)

2. **Compute (R*v) * R̃** - Multiply the intermediate result by the reverse rotor, extracting only the vector components

```rust
// R * v produces vector and trivector parts
let rv_e1 = s * v.x + b12 * v.y + b13 * v.z + b14 * v.w;
let rv_e2 = s * v.y - b12 * v.x + b23 * v.z + b24 * v.w;
let rv_e3 = s * v.z - b13 * v.x - b23 * v.y + b34 * v.w;
let rv_e4 = s * v.w - b14 * v.x - b24 * v.y - b34 * v.z;

let rv_e123 = b12 * v.z - b13 * v.y + b23 * v.x + p * v.w;
let rv_e124 = b12 * v.w - b14 * v.y + b24 * v.x - p * v.z;
let rv_e134 = b13 * v.w - b14 * v.z + b34 * v.x + p * v.y;
let rv_e234 = b23 * v.w - b24 * v.z + b34 * v.y - p * v.x;

// (R*v) * R̃ extracts the vector part
let new_x = rv_e1 * s + rv_e2 * b12 + rv_e3 * b13 + rv_e4 * b14
    + rv_e123 * b23 + rv_e124 * b24 + rv_e134 * b34 - rv_e234 * p;
// ... similar for y, z, w
```

### Dead Code Removal

After fixing the rotation, removed unused functions:
- `to_rotation_matrix()` - explicit formula that was incorrect
- `rotate_explicit()` - old implementation marked with `#[allow(dead_code)]`

The `to_matrix()` function now correctly builds the rotation matrix by calling `rotate()` on each basis vector.

---

## Part 2: Camera Incremental Rotation System

### The Problem

The camera stored 4 Euler-like angles (`yaw`, `pitch`, `roll_w`, `roll_xw`) and rebuilt the orientation rotor from scratch each frame:

```rust
// Old approach - absolute angles recomposed every frame
fn rebuild_orientation(&mut self) {
    let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, self.yaw);
    let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, self.pitch);
    let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, self.roll_w);
    let r_roll_xw = Rotor4::from_plane_angle(RotationPlane::XW, self.roll_xw);

    self.orientation = r_roll_xw.compose(&r_roll_w.compose(&r_pitch.compose(&r_yaw))).normalize();
}
```

This caused gimbal-lock-like issues because:
1. **Yaw was applied in world space before 4D rotations** - After rotating into 4D (roll_w, roll_xw), yaw didn't turn the camera left/right in the expected way
2. **Fixed composition order** - The rotations were always composed in the same order regardless of when they were applied

### The Solution

Changed to **incremental rotation** where rotations are applied relative to the current orientation:

```rust
pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
    // Yaw: rotate in world XZ plane (keeps horizon level)
    if delta_yaw.abs() > 0.0001 {
        let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, delta_yaw);
        // World space: new = r_yaw * orientation
        self.orientation = r_yaw.compose(&self.orientation).normalize();
    }

    // Pitch: rotate in camera-local YZ plane
    if actual_delta_pitch.abs() > 0.0001 {
        let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, actual_delta_pitch);
        // Local space: new = orientation * r_pitch
        self.orientation = self.orientation.compose(&r_pitch).normalize();
    }
}
```

Key design decisions:
- **Yaw in world space** - `r_yaw.compose(&orientation)` - Keeps horizon level regardless of 4D orientation
- **Pitch in local space** - `orientation.compose(&r_pitch)` - Works correctly after any amount of yaw
- **4D rotations in local space** - `orientation.compose(&r_4d)` - Applied relative to current view

### Pitch Clamping

Still track accumulated pitch to prevent looking past vertical:
```rust
pitch_accumulator: f32,  // Track total pitch for clamping

let new_pitch = (self.pitch_accumulator + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
let actual_delta_pitch = new_pitch - self.pitch_accumulator;
self.pitch_accumulator = new_pitch;
```

---

## Part 3: Camera Controller Improvements

### Input Smoothing

Added exponential smoothing for mouse input (engine4d-style):

```rust
let smooth_factor = 2.0f32.powf(-dt / self.smoothing_half_life);
self.smooth_yaw = self.smooth_yaw * smooth_factor + self.pending_yaw * (1.0 - smooth_factor);
self.smooth_pitch = self.smooth_pitch * smooth_factor + self.pending_pitch * (1.0 - smooth_factor);
```

- Configurable via `smoothing_half_life` (default 0.05s = 50ms)
- Toggle with G key
- Disabled by default for responsive FPS feel

### Cursor Capture Support

Updated controller to accept `cursor_captured` parameter:
- When cursor captured: free look enabled (no click required)
- When cursor not captured: need left-click to rotate

---

## Part 4: Main Application FPS Controls

### Click-to-Capture Pattern

Implemented standard FPS-style mouse capture:
- Click window to capture cursor
- Escape releases cursor (press again to exit)
- Uses `CursorGrabMode::Locked` for true FPS feel

### View Matrix Fix

Fixed view matrix to use camera's actual up vector:
```rust
// Before (wrong): fixed world up
let view_matrix = look_at_matrix(eye, target, [0.0, 1.0, 0.0]);

// After (correct): camera's up vector
let up = self.camera.up();
let view_matrix = look_at_matrix(eye, target, [up.x, up.y, up.z]);
```

This prevents weird distortion when the camera is rotated in 4D.

---

## Test Results

All 83 tests pass:
- 29 tests in rust4d_math (including comprehensive rotor composition tests)
- 54 tests in rust4d_render (including 12 camera tests)
- 0 compilation warnings

### New Camera Tests Added

- `test_yaw_after_zw_rotation` - Verifies yaw works after 4D rotation
- `test_incremental_rotation_preserves_orthogonality` - Basis vectors stay orthonormal
- `test_yaw_keeps_horizon_level` - Up vector Y preserved during yaw
- `test_pitch_is_local` - Pitch works correctly after yaw

---

## Files Modified

| File | Changes |
|------|---------|
| `crates/rust4d_math/src/rotor4.rs` | Fixed rotate(), removed dead code, updated to_matrix() |
| `crates/rust4d_render/src/camera4d.rs` | Incremental rotation system, new tests |
| `crates/rust4d_input/src/camera_controller.rs` | Smoothing, cursor capture support |
| `src/main.rs` | FPS controls, click-to-capture, view matrix fix |

---

## Lessons Learned

1. **Explicit formulas vs step-by-step computation** - For complex algebraic operations like the sandwich product, computing step-by-step is more reliable than deriving explicit formulas, especially when the formula needs to handle general cases.

2. **Euler angles in 4D are even more problematic** - In 3D, Euler angles have known gimbal lock issues. In 4D with 6 rotation planes, the problem is worse. Incremental rotation with rotors is the right approach.

3. **World vs local space rotations** - Choosing which rotations to apply in world space vs local space is crucial for intuitive controls. Yaw in world space keeps horizon level; pitch in local space follows the view direction.

4. **Test composed rotations thoroughly** - Simple single-rotation tests may pass even with buggy code. Always test composed rotations with orthogonality and length preservation checks.

---

## Future Considerations

1. **Additional 4D rotation controls** - Currently only ZW and XW rotations are exposed. YW rotation could be added for completeness.

2. **Quaternion-style interpolation** - For smooth camera animations, implementing rotor SLERP would be valuable.

3. **Numerical drift** - Over very long sessions, accumulated floating-point error could cause the rotor to drift from unit length. Periodic renormalization could help.
