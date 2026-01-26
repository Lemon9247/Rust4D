# Camera Controls Improvement Plan

**Date**: 2026-01-26
**Status**: Synthesis Report
**Prepared by**: Swarm synthesis of Rust4D Analysis Agent and Engine4D Research Agent findings

---

## Executive Summary

The camera controls are broken due to a **critical bug: rotation planes are swapped**. Additionally, several UX issues make the controls feel uncomfortable compared to standard FPS games and engine4d's implementation.

**Root Cause**: In `camera4d.rs`, yaw uses XY plane and pitch uses XZ plane. This is backwards - should be XZ for yaw (left/right) and YZ for pitch (up/down).

---

## Issues Identified

### Critical (Controls Don't Work)

| Issue | Location | Impact |
|-------|----------|--------|
| **Rotation planes swapped** | `camera4d.rs:126-128` | Looking left/right and up/down are completely wrong |
| **Incorrect rotor4.rs comments** | `rotor4.rs:17-19` | Misleading documentation |

### High Priority (Uncomfortable UX)

| Issue | Location | Impact |
|-------|----------|--------|
| **Click-to-rotate required** | `camera_controller.rs:123` | Can't look around without holding mouse button |
| **No cursor capture** | `main.rs` | Mouse visible and not confined |
| **No input smoothing** | `camera_controller.rs` | Jerky camera movement |

### Medium Priority (Polish)

| Issue | Location | Impact |
|-------|----------|--------|
| **Movement includes Y** | `camera4d.rs:62-71` | Flight-mode behavior when looking up/down |
| **W-rotation only uses X-axis** | `camera_controller.rs:126` | Vertical mouse ignored in W-mode |
| **W-movement is absolute** | `camera4d.rs:74-76` | Doesn't follow camera orientation |

---

## Improvement Plan

### Phase 1: Critical Bug Fix (Session 1)

**Goal**: Fix the rotation planes so looking around works correctly.

#### Task 1.1: Fix Rotation Planes in camera4d.rs

```rust
// Current (WRONG):
let r_yaw = Rotor4::from_plane_angle(RotationPlane::XY, self.yaw);
let r_pitch = Rotor4::from_plane_angle(RotationPlane::XZ, self.pitch);

// Fixed:
let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, self.yaw);    // Around Y axis
let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, self.pitch); // Around X axis
```

#### Task 1.2: Fix Comments in camera4d.rs

```rust
// Current:
pitch: f32,      // XZ plane rotation
yaw: f32,        // XY plane rotation

// Fixed:
pitch: f32,      // YZ plane rotation (around X axis - look up/down)
yaw: f32,        // XZ plane rotation (around Y axis - look left/right)
```

#### Task 1.3: Fix rotor4.rs Documentation

```rust
// Current (WRONG):
/// XY plane - standard yaw (rotation around Z axis in 3D)
/// XZ plane - standard pitch (rotation around Y axis in 3D)

// Fixed:
/// XY plane - roll (rotation around Z axis in 3D) - tilt head sideways
/// XZ plane - yaw (rotation around Y axis in 3D) - look left/right
/// YZ plane - pitch (rotation around X axis in 3D) - look up/down
```

#### Task 1.4: Verify Rotor Composition Order

Current: `r_yaw.compose(&r_pitch).compose(&r_roll_w)`

This applies roll_w first, then pitch, then yaw. Standard FPS order is yaw first (global), then pitch (local).

Consider changing to: `r_pitch.compose(&r_yaw).compose(&r_roll_w)` after verifying rotor compose semantics.

---

### Phase 2: Cursor Capture & Free Look (Session 1-2)

**Goal**: Add cursor capture mode for standard FPS feel.

#### Task 2.1: Add Cursor Grab State to App

```rust
struct App {
    // ... existing fields ...
    cursor_captured: bool,
}
```

#### Task 2.2: Add Tab Toggle for Cursor Capture

In main.rs keyboard handling:
```rust
KeyCode::Tab => {
    self.cursor_captured = !self.cursor_captured;
    if let Some(window) = &self.window {
        if self.cursor_captured {
            let _ = window.set_cursor_grab(CursorGrabMode::Confined);
            window.set_cursor_visible(false);
        } else {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
        }
    }
}
```

#### Task 2.3: Update Camera Controller for Free Look

Modify `camera_controller.rs` to accept a `cursor_captured` parameter:

```rust
pub fn update<C: CameraControl>(&mut self, camera: &mut C, dt: f32, cursor_captured: bool) -> Vec4 {
    // ...

    // Apply rotation when cursor is captured OR when mouse button pressed
    if cursor_captured || self.mouse_pressed || self.w_rotation_mode {
        if self.w_rotation_mode {
            camera.rotate_w(self.pending_yaw * self.w_rotation_sensitivity);
        } else {
            camera.rotate_3d(
                -self.pending_yaw * self.mouse_sensitivity,
                -self.pending_pitch * self.mouse_sensitivity,
            );
        }
    }
    // ...
}
```

---

### Phase 3: Input Smoothing (Session 2)

**Goal**: Add exponential smoothing for smoother camera movement.

#### Task 3.1: Add Smoothing State to CameraController

```rust
pub struct CameraController {
    // ... existing fields ...

    // Smoothed values
    smooth_yaw: f32,
    smooth_pitch: f32,

    // Configuration
    pub smoothing_half_life: f32,  // 0.05 = 50ms (engine4d default)
}
```

#### Task 3.2: Implement Exponential Smoothing

```rust
pub fn update<C: CameraControl>(&mut self, camera: &mut C, dt: f32, cursor_captured: bool) -> Vec4 {
    // Exponential smoothing (engine4d style)
    let smooth_factor = if dt > 0.0 {
        2.0f32.powf(-dt / self.smoothing_half_life)
    } else {
        0.0
    };

    self.smooth_yaw = self.smooth_yaw * smooth_factor + self.pending_yaw * (1.0 - smooth_factor);
    self.smooth_pitch = self.smooth_pitch * smooth_factor + self.pending_pitch * (1.0 - smooth_factor);

    // Use smooth values for rotation
    if cursor_captured || self.mouse_pressed || self.w_rotation_mode {
        // ...use smooth_yaw and smooth_pitch instead of pending_*...
    }

    // Reset pending after smoothing
    self.pending_yaw = 0.0;
    self.pending_pitch = 0.0;

    // ...
}
```

---

### Phase 4: Movement Improvements (Session 2-3)

**Goal**: Improve movement feel with optional FPS-style horizontal movement.

#### Task 4.1: Add Movement Mode Option

```rust
pub enum MovementMode {
    Flight,  // Current behavior - forward moves in look direction including Y
    FPS,     // Forward stays horizontal, separate Y control
}

impl CameraController {
    pub movement_mode: MovementMode,
}
```

#### Task 4.2: Implement FPS-Style Movement

```rust
pub fn move_local_xz(&mut self, forward: f32, right: f32) {
    let fwd = self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0));
    let rgt = self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0));

    match self.movement_mode {
        MovementMode::Flight => {
            // Current behavior - includes Y
            self.position.x += fwd.x * forward + rgt.x * right;
            self.position.y += fwd.y * forward + rgt.y * right;
            self.position.z += fwd.z * forward + rgt.z * right;
        }
        MovementMode::FPS => {
            // Project forward onto XZ plane
            let fwd_xz = (fwd.x * fwd.x + fwd.z * fwd.z).sqrt();
            if fwd_xz > 0.001 {
                self.position.x += (fwd.x / fwd_xz) * forward;
                self.position.z += (fwd.z / fwd_xz) * forward;
            }
            // Right is already mostly horizontal
            self.position.x += rgt.x * right;
            self.position.z += rgt.z * right;
        }
    }
}
```

#### Task 4.3: Make W-Movement Camera-Relative (Optional)

```rust
pub fn move_w(&mut self, delta: f32) {
    // Camera-relative W movement
    let w_axis = self.orientation.rotate(Vec4::new(0.0, 0.0, 0.0, 1.0));
    self.position.x += w_axis.x * delta;
    self.position.y += w_axis.y * delta;
    self.position.z += w_axis.z * delta;
    self.position.w += w_axis.w * delta;
}
```

---

### Phase 5: W-Rotation Improvements (Session 3)

**Goal**: Improve 4D rotation controls.

#### Task 5.1: Use Vertical Mouse for Additional W-Plane

```rust
if self.w_rotation_mode {
    // Horizontal mouse: ZW rotation (current)
    camera.rotate_w(self.smooth_yaw * self.w_rotation_sensitivity);
    // Vertical mouse: XW rotation (new)
    camera.rotate_xw(self.smooth_pitch * self.w_rotation_sensitivity);
}
```

#### Task 5.2: Add XW Rotation to Camera4D

```rust
roll_xw: f32,  // New field

pub fn rotate_xw(&mut self, delta: f32) {
    self.roll_xw += delta;
    self.rebuild_orientation();
}

fn rebuild_orientation(&mut self) {
    let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, self.yaw);
    let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, self.pitch);
    let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, self.roll_w);
    let r_roll_xw = Rotor4::from_plane_angle(RotationPlane::XW, self.roll_xw);

    self.orientation = r_pitch.compose(&r_yaw).compose(&r_roll_w).compose(&r_roll_xw).normalize();
}
```

---

## Session Estimates

| Phase | Sessions | Priority |
|-------|----------|----------|
| Phase 1: Critical Bug Fix | 0.5 | **Critical** |
| Phase 2: Cursor Capture | 0.5-1 | High |
| Phase 3: Input Smoothing | 0.5 | Medium |
| Phase 4: Movement | 1 | Medium |
| Phase 5: W-Rotation | 0.5 | Low |

**Total**: 3-3.5 sessions for full implementation

**Recommended approach**: Implement Phase 1 + Phase 2 together as they're tightly coupled. Test thoroughly. Then Phase 3, 4, 5 as separate polish passes.

---

## Testing Checklist

After implementing each phase:

- [ ] **Phase 1**: Mouse left/right turns camera left/right, mouse up/down looks up/down
- [ ] **Phase 2**: Tab captures cursor, free mouse look works, Tab releases cursor
- [ ] **Phase 3**: Camera movement feels smooth, not jerky
- [ ] **Phase 4**: Forward movement stays horizontal (FPS mode) or follows look direction (Flight mode)
- [ ] **Phase 5**: Right-click W-rotation uses both mouse axes

---

## Notes

1. **Keep rotor-based approach**: It's mathematically cleaner than engine4d's quaternion hack
2. **Right-click for W-rotation is good**: Simpler than a dedicated keybind for casual users
3. **Consider making smoothing optional**: Some users prefer raw input for precision
4. **Movement mode could be runtime toggle**: Let users switch between Flight and FPS modes

---

*This plan synthesizes findings from the Rust4D Analysis Agent and Engine4D Research Agent reports.*
