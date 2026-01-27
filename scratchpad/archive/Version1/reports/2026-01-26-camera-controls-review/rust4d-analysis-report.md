# Rust4D Camera Controls Analysis Report

**Agent**: Rust4D Analysis Agent
**Date**: 2026-01-26
**Files Analyzed**:
- `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_input/src/camera_controller.rs`
- `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_render/src/camera4d.rs`
- `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_math/src/rotor4.rs`
- `/home/lemoneater/Projects/Personal/Rust4D/src/main.rs`

---

## Executive Summary

The camera controls implementation has **several critical issues** that make it uncomfortable to use:

1. **Major Bug**: Yaw and pitch rotation planes are swapped (XY for yaw, XZ for pitch - should be reversed for standard FPS controls)
2. **Movement Direction Issues**: Forward movement uses `-Z` but XZ plane pitch will make this unintuitive
3. **Click-to-rotate UX**: Requiring click-hold for mouse look is non-standard and awkward for exploration
4. **Sensitivity Values**: Default sensitivities are reasonable but may need tuning
5. **Rotor Composition Order**: Current order (yaw * pitch * roll_w) may cause gimbal-like issues

---

## Detailed Analysis

### Issue 1: Rotation Planes Are Wrong (Critical)

**Location**: `camera4d.rs` lines 22-25 and 124-130

```rust
// Current code comments and implementation:
pitch: f32,      // XZ plane rotation  <-- WRONG for pitch
yaw: f32,        // XY plane rotation  <-- WRONG for yaw

fn rebuild_orientation(&mut self) {
    let r_yaw = Rotor4::from_plane_angle(RotationPlane::XY, self.yaw);
    let r_pitch = Rotor4::from_plane_angle(RotationPlane::XZ, self.pitch);
    // ...
}
```

**Problem**: In standard 3D coordinate systems with Y-up:
- **Yaw** (looking left/right) should rotate in the **XZ plane** (around Y axis)
- **Pitch** (looking up/down) should rotate in the **YZ plane** (around X axis)

The current implementation has:
- Yaw in XY plane (rotating around Z axis) - this rotates like a tilted head!
- Pitch in XZ plane (rotating around Y axis) - this rotates like turning left/right!

**These are swapped!** This explains why the controls "don't work right."

**Evidence from rotor4.rs comments** (lines 17-19):
```rust
/// XY plane - standard yaw (rotation around Z axis in 3D)
/// XZ plane - standard pitch (rotation around Y axis in 3D)
```

The comments in rotor4.rs are also incorrect! In a Y-up coordinate system:
- XY plane rotation = rotation around Z axis = **roll** (tilting head sideways)
- XZ plane rotation = rotation around Y axis = **yaw** (turning left/right)
- YZ plane rotation = rotation around X axis = **pitch** (looking up/down)

### Issue 2: Forward Direction and Movement

**Location**: `camera4d.rs` lines 62-71

```rust
pub fn move_local_xz(&mut self, forward: f32, right: f32) {
    // Get forward and right vectors in world space
    let fwd = self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0));
    let rgt = self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0));

    // Project movement onto the XYZ plane (ignore W component for XZ movement)
    self.position.x += fwd.x * forward + rgt.x * right;
    self.position.y += fwd.y * forward + rgt.y * right;  // <-- includes Y
    self.position.z += fwd.z * forward + rgt.z * right;
}
```

**Problem**: The function is named `move_local_xz` but it includes Y component changes. This means:
- If you pitch the camera up and press forward, you'll move upward
- This is "flight mode" behavior, not "FPS mode" behavior

For standard FPS-style controls, forward/backward movement should be projected onto the XZ plane only (horizontal movement regardless of where you're looking).

**Recommendation**: Either:
1. Rename to `move_local` and document the flight behavior, or
2. Project the forward vector onto XZ plane: `forward_xz = normalize(forward.x, 0, forward.z)`

### Issue 3: Click-to-Rotate Input Scheme

**Location**: `camera_controller.rs` lines 123-134

```rust
// Apply rotation
if self.mouse_pressed || self.w_rotation_mode {
    if self.w_rotation_mode {
        // Right-click: W-rotation mode
        camera.rotate_w(self.pending_yaw * self.w_rotation_sensitivity);
    } else {
        // Left-click: Standard 3D rotation
        camera.rotate_3d(
            -self.pending_yaw * self.mouse_sensitivity,
            -self.pending_pitch * self.mouse_sensitivity,
        );
    }
}
```

**Problem**: Mouse look only works when holding a mouse button. This is:
- Unusual for first-person exploration (most expect free mouse look)
- Makes it hard to look around while not holding the mouse
- May conflict with click-to-interact mechanics later

**Note**: The negative signs on yaw and pitch are correct for inverting mouse movement direction.

**Recommendation**: Consider adding a "mouse capture" mode toggle (e.g., press Tab) that locks the cursor and enables free look without clicking.

### Issue 4: W-Rotation Uses Only Horizontal Mouse Movement

**Location**: `camera_controller.rs` line 126

```rust
camera.rotate_w(self.pending_yaw * self.w_rotation_sensitivity);
```

**Problem**: Right-click W-rotation only uses the X-axis (yaw) mouse movement. The Y-axis (pitch) movement is ignored. This means:
- Only horizontal mouse movement affects W-rotation
- Vertical mouse movement does nothing in W-mode
- Could feel "broken" to users who expect both axes to do something

**Recommendation**: Either:
1. Use pending_pitch instead for a different W-rotation plane, or
2. Use both axes for two different W-plane rotations, or
3. Document clearly that W-rotation is horizontal-only

### Issue 5: Rotor Composition Order

**Location**: `camera4d.rs` line 130

```rust
self.orientation = r_yaw.compose(&r_pitch).compose(&r_roll_w).normalize();
```

The composition order is: `yaw * pitch * roll_w`

With rotor composition, `a.compose(&b)` applies `b` first, then `a`. So this applies:
1. roll_w first
2. pitch second
3. yaw last

**Potential Issue**: When yaw is applied after pitch, pitch will always be in the local rotated frame. This can cause unexpected behavior when combined with the plane swapping bug.

**Standard FPS order** should be: yaw first (global Y rotation), then pitch (local X rotation). This would be:
```rust
r_pitch.compose(&r_yaw)  // for standard 3D FPS
```

### Issue 6: Pitch Clamping Is Correct

**Location**: `camera4d.rs` line 51

```rust
self.pitch = (self.pitch + delta_pitch).clamp(-1.5, 1.5);
```

This clamps pitch to approximately +/- 86 degrees, which is good. This prevents gimbal lock at +/- 90 degrees.

### Issue 7: Missing Mouse Capture / Cursor Lock

**Location**: `main.rs`

There's no cursor grabbing or hiding. For a 3D exploration tool, users typically expect:
- Cursor to be hidden during free look
- Cursor to be captured (confined to window)
- A way to release the cursor (Escape or Tab)

---

## Summary of Issues by Severity

### Critical (Controls Don't Work Correctly)
1. **Rotation planes swapped**: XY used for yaw, XZ for pitch - both wrong
2. **Misleading comments in rotor4.rs**: Documentation doesn't match standard conventions

### High (Uncomfortable to Use)
3. **Click-to-rotate required**: No free mouse look mode
4. **Movement includes Y**: Forward movement changes altitude when looking up/down

### Medium (Polish Issues)
5. **W-rotation only uses X-axis**: Vertical mouse ignored in W-mode
6. **No cursor capture**: Mouse visible and free during gameplay
7. **Rotor composition order**: May cause unintuitive compound rotations

### Low (Tuning)
8. **Sensitivity values**: Current defaults may need adjustment after fixing rotation

---

## Recommended Fixes

### Fix 1: Correct the Rotation Planes (Critical)

Change `camera4d.rs`:
```rust
// Comments should be:
pitch: f32,      // YZ plane rotation (around X axis - look up/down)
yaw: f32,        // XZ plane rotation (around Y axis - look left/right)

fn rebuild_orientation(&mut self) {
    // Yaw in XZ plane (around Y axis)
    let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, self.yaw);
    // Pitch in YZ plane (around X axis)
    let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, self.pitch);
    let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, self.roll_w);

    // Apply yaw first (global), then pitch (local), then W-roll
    self.orientation = r_pitch.compose(&r_yaw).compose(&r_roll_w).normalize();
}
```

### Fix 2: Update rotor4.rs Comments

```rust
/// XY plane - roll (rotation around Z axis in 3D)
/// XZ plane - yaw (rotation around Y axis in 3D) - LEFT/RIGHT look
/// YZ plane - pitch (rotation around X axis in 3D) - UP/DOWN look
```

### Fix 3: Add Mouse Capture Toggle

Add to main.rs keyboard handling:
```rust
KeyCode::Tab => {
    // Toggle mouse capture
    if let Some(window) = &self.window {
        let grabbed = window.cursor_grab_mode() != CursorGrabMode::None;
        if grabbed {
            window.set_cursor_grab(CursorGrabMode::None).ok();
            window.set_cursor_visible(true);
        } else {
            window.set_cursor_grab(CursorGrabMode::Confined).ok();
            window.set_cursor_visible(false);
        }
    }
}
```

And update camera_controller to always process rotation when cursor is grabbed (not just on click).

### Fix 4: FPS-Style Horizontal Movement (Optional)

If FPS-style movement is desired, modify `move_local_xz`:
```rust
pub fn move_local_xz(&mut self, forward: f32, right: f32) {
    let fwd = self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0));
    let rgt = self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0));

    // Project onto XZ plane (horizontal only)
    let fwd_xz_len = (fwd.x * fwd.x + fwd.z * fwd.z).sqrt();
    let rgt_xz_len = (rgt.x * rgt.x + rgt.z * rgt.z).sqrt();

    if fwd_xz_len > 0.001 {
        self.position.x += (fwd.x / fwd_xz_len) * forward;
        self.position.z += (fwd.z / fwd_xz_len) * forward;
    }
    if rgt_xz_len > 0.001 {
        self.position.x += (rgt.x / rgt_xz_len) * right;
        self.position.z += (rgt.z / rgt_xz_len) * right;
    }
}
```

---

## Conclusion

The primary cause of the "uncomfortable" and "doesn't work right" feedback is the **swapped rotation planes**. The code uses XY for yaw and XZ for pitch, but standard Y-up 3D conventions require XZ for yaw and YZ for pitch. This makes looking left/right and up/down feel completely wrong.

Fixing the rotation planes is the highest priority. The other issues (click-to-rotate, cursor capture, movement projection) are quality-of-life improvements that will make the controls feel more polished.
