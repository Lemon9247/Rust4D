# Plan: Re-implement Camera System (Engine4D Style)

**Date:** 2026-01-27
**Estimated Sessions:** 2-3

## Goal

Re-architect the camera and movement system to match Engine4D's approach, where:
1. **Pitch is stored separately** from 4D rotation
2. **4D rotations operate in XZW hyperplane only** (skipping Y)
3. **Movement is transformed by the full camera matrix**
4. **Y axis always remains aligned with gravity/world up**

---

## Phase 1: Core Math - SkipY Implementation

**Session estimate:** 0.5

### Task 1.1: Add SkipY matrix transformation

Add a function to `rust4d_math` that remaps a 4x4 rotation matrix so it operates in XZW, skipping Y:

```rust
// In rust4d_math/src/lib.rs or a new transforms.rs

/// Remap a 4D rotation matrix so it operates in the XZW hyperplane,
/// leaving the Y axis unchanged.
///
/// This maps:
/// - X axis -> X axis (unchanged)
/// - Y axis -> Z axis (in 4D)
/// - Z axis -> W axis (in 4D)
///
/// Used to keep 4D rotations from affecting the gravity direction.
pub fn skip_y_matrix(m: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    // XYZTo(m, 0, 2, 3) equivalent
    // Maps columns: col[0]->col[0], col[1]->col[2], col[2]->col[3]
    // Y column (col[1]) becomes identity
    let mut result = [[0.0; 4]; 4];

    // Set column 0 from source column 0
    result[0][0] = m[0][0];
    result[0][2] = m[0][1];  // Y->Z
    result[0][3] = m[0][2];  // Z->W

    // Column 1 (Y) is identity
    result[1][1] = 1.0;

    // Set column 2 from source column 1 (Y->Z)
    result[2][0] = m[1][0];
    result[2][2] = m[1][1];
    result[2][3] = m[1][2];

    // Set column 3 from source column 2 (Z->W)
    result[3][0] = m[2][0];
    result[3][2] = m[2][1];
    result[3][3] = m[2][2];

    // Also need row remapping...
    // Actually this is more complex - need to study Transform4D.XYZTo carefully

    result
}
```

**Note:** The actual implementation needs to match Engine4D's `XYZTo` function which does both column AND row remapping. Study `Transform4D.cs:297-306` carefully.

### Task 1.2: Add PlaneRotation helper

```rust
/// Create a rotation matrix in a specific 2D plane within 4D space.
/// Equivalent to Engine4D's Transform4D.PlaneRotation
///
/// # Arguments
/// * `angle` - Rotation angle in radians
/// * `p1`, `p2` - Indices of the axes forming the rotation plane (0=X, 1=Y, 2=Z, 3=W)
pub fn plane_rotation_matrix(angle: f32, p1: usize, p2: usize) -> [[f32; 4]; 4] {
    let cs = angle.cos();
    let sn = angle.sin();
    let mut m = [[0.0; 4]; 4];

    // Identity diagonal
    for i in 0..4 {
        m[i][i] = 1.0;
    }

    // Rotation in plane p1-p2
    m[p1][p1] = cs;
    m[p2][p2] = cs;
    m[p1][p2] = sn;
    m[p2][p1] = -sn;

    m
}
```

### Task 1.3: Add matrix multiplication

```rust
/// Multiply two 4x4 matrices
pub fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }
    result
}

/// Transform a Vec4 by a 4x4 matrix
pub fn mat4_transform(m: [[f32; 4]; 4], v: Vec4) -> Vec4 {
    Vec4::new(
        m[0][0]*v.x + m[1][0]*v.y + m[2][0]*v.z + m[3][0]*v.w,
        m[0][1]*v.x + m[1][1]*v.y + m[2][1]*v.z + m[3][1]*v.w,
        m[0][2]*v.x + m[1][2]*v.y + m[2][2]*v.z + m[3][2]*v.w,
        m[0][3]*v.x + m[1][3]*v.y + m[2][3]*v.z + m[3][3]*v.w,
    )
}
```

---

## Phase 2: Camera4D Re-architecture

**Session estimate:** 1

### Task 2.1: Change Camera4D struct

```rust
/// 4D Camera using Engine4D-style architecture
///
/// Pitch (YZ plane) is stored separately from 4D rotation.
/// 4D rotations operate in the XZW hyperplane only (SkipY),
/// keeping Y aligned with world up.
pub struct Camera4D {
    /// 4D position (x, y, z, w)
    pub position: Vec4,

    /// Pitch angle in radians (YZ plane rotation, clamped ±89°)
    /// This is the "lookYZ" from Engine4D
    pitch: f32,

    /// 4D rotation quaternion-equivalent (operates in XZW hyperplane only)
    /// This is the "m1" from Engine4D - stored as a 3x3 rotation
    /// (we use Rotor4 but only for XZW rotations)
    rotation_4d: Rotor4,

    /// Cross-section offset from camera W position
    pub slice_offset: f32,
}
```

### Task 2.2: Implement camera matrix construction

```rust
impl Camera4D {
    /// Build the camera transformation matrix (Engine4D style)
    ///
    /// Composition: skip_y(rotation_4d) * pitch_rotation
    /// This ensures pitch is applied first (local), then 4D rotation (XZW only)
    pub fn camera_matrix(&self) -> [[f32; 4]; 4] {
        // 1. Build pitch rotation in YZ plane
        let pitch_mat = plane_rotation_matrix(self.pitch, 1, 2);

        // 2. Build 4D rotation matrix and apply SkipY
        let rot_4d_raw = self.rotation_4d.to_matrix();
        let rot_4d_skip_y = skip_y_matrix(rot_4d_raw);

        // 3. Combine: 4D rotation * pitch
        // (right-to-left: pitch applied first)
        mat4_mul(rot_4d_skip_y, pitch_mat)
    }

    /// Get forward direction (transformed by camera matrix)
    pub fn forward(&self) -> Vec4 {
        let cam_mat = self.camera_matrix();
        mat4_transform(cam_mat, Vec4::new(0.0, 0.0, -1.0, 0.0))
    }

    /// Get right direction
    pub fn right(&self) -> Vec4 {
        let cam_mat = self.camera_matrix();
        mat4_transform(cam_mat, Vec4::new(1.0, 0.0, 0.0, 0.0))
    }

    /// Get up direction
    pub fn up(&self) -> Vec4 {
        let cam_mat = self.camera_matrix();
        mat4_transform(cam_mat, Vec4::new(0.0, 1.0, 0.0, 0.0))
    }

    /// Get ana (W) direction
    pub fn ana(&self) -> Vec4 {
        let cam_mat = self.camera_matrix();
        mat4_transform(cam_mat, Vec4::new(0.0, 0.0, 0.0, 1.0))
    }
}
```

### Task 2.3: Implement looking controls

```rust
impl Camera4D {
    const PITCH_LIMIT: f32 = 1.553; // ~89 degrees

    /// Standard 3D mouse look (yaw and pitch)
    ///
    /// Engine4D style:
    /// - Horizontal (yaw): Applied to rotation_4d as Z rotation (becomes ZW via SkipY)
    /// - Vertical (pitch): Applied to separate pitch variable
    pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
        // Yaw: modify rotation_4d with Z-axis rotation
        // After SkipY, this becomes a rotation in the ZW plane
        if delta_yaw.abs() > 0.0001 {
            let r_z = Rotor4::from_euler_xyz(0.0, 0.0, -delta_yaw);
            self.rotation_4d = self.rotation_4d.compose(&r_z).normalize();
        }

        // Pitch: modify separate pitch variable (Engine4D's lookYZ)
        self.pitch = (self.pitch + delta_pitch).clamp(-Self::PITCH_LIMIT, Self::PITCH_LIMIT);
    }

    /// 4D look mode (when holding 4D look key)
    /// Both horizontal and vertical modify rotation_4d
    pub fn rotate_4d_look(&mut self, delta_x: f32, delta_y: f32) {
        // Apply as Euler angles to rotation_4d
        // (This matches Engine4D's m1 = m1 * Quaternion.Euler(-smoothAngY, smoothAngX, 0.0f))
        if delta_x.abs() > 0.0001 || delta_y.abs() > 0.0001 {
            let r = Rotor4::from_euler_xyz(-delta_y, delta_x, 0.0);
            self.rotation_4d = self.rotation_4d.compose(&r).normalize();
        }
    }

    /// 4D ZW rotation (Q/E keys typically)
    pub fn rotate_zw(&mut self, delta: f32) {
        // This is a Z rotation which becomes ZW after SkipY
        if delta.abs() > 0.0001 {
            let r = Rotor4::from_euler_xyz(0.0, 0.0, delta);
            self.rotation_4d = self.rotation_4d.compose(&r).normalize();
        }
    }

    /// 4D XW rotation
    pub fn rotate_xw(&mut self, delta: f32) {
        // This is an X rotation which becomes XW after SkipY
        if delta.abs() > 0.0001 {
            let r = Rotor4::from_euler_xyz(delta, 0.0, 0.0);
            self.rotation_4d = self.rotation_4d.compose(&r).normalize();
        }
    }
}
```

### Task 2.4: Implement movement

```rust
impl Camera4D {
    /// Move using camera matrix transformation (Engine4D style)
    ///
    /// Input is in camera-local space:
    /// - forward/back on Z axis
    /// - left/right on X axis
    /// - up/down on Y axis
    /// - ana/kata on W axis
    ///
    /// The camera matrix transforms this to world space.
    pub fn move_camera(&mut self, forward: f32, right: f32, up: f32, ana: f32) {
        if forward.abs() < 0.0001 && right.abs() < 0.0001 && up.abs() < 0.0001 && ana.abs() < 0.0001 {
            return;
        }

        // Build input vector in camera space
        let input = Vec4::new(right, up, -forward, ana);

        // Transform by camera matrix
        let cam_mat = self.camera_matrix();
        let world_movement = mat4_transform(cam_mat, input);

        // Apply movement (Engine4D would also handle gravity here)
        self.position = self.position + world_movement;
    }

    // For backwards compatibility, keep XZ movement separate
    pub fn move_local_xz(&mut self, forward: f32, right: f32) {
        self.move_camera(forward, right, 0.0, 0.0);
    }

    pub fn move_y(&mut self, delta: f32) {
        self.position.y += delta;
    }

    pub fn move_w(&mut self, delta: f32) {
        self.move_camera(0.0, 0.0, 0.0, delta);
    }
}
```

---

## Phase 3: Rotor4 Extensions

**Session estimate:** 0.5

### Task 3.1: Add Euler angle constructor

Engine4D uses quaternion Euler angles. We need an equivalent:

```rust
impl Rotor4 {
    /// Create a rotor from Euler angles (XYZ order)
    /// This creates a 3D rotation that will be remapped via SkipY
    pub fn from_euler_xyz(x: f32, y: f32, z: f32) -> Self {
        let rx = Self::from_plane_angle(RotationPlane::YZ, x);  // X rotation = YZ plane
        let ry = Self::from_plane_angle(RotationPlane::XZ, y);  // Y rotation = XZ plane
        let rz = Self::from_plane_angle(RotationPlane::XY, z);  // Z rotation = XY plane

        // Compose in XYZ order
        rz.compose(&ry.compose(&rx))
    }
}
```

---

## Phase 4: CameraController Updates

**Session estimate:** 0.5

### Task 4.1: Add 4D look mode

```rust
pub struct CameraController {
    // ... existing fields ...

    /// Whether 4D look mode is active (middle mouse or specific key)
    look_4d_active: bool,
}

impl CameraController {
    pub fn process_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        match button {
            MouseButton::Middle => {
                self.look_4d_active = state == ElementState::Pressed;
            }
            // ... existing handling ...
        }
    }

    pub fn update(&mut self, camera: &mut impl CameraControl, dt: f32, cursor_captured: bool) {
        // ... existing smoothing ...

        if self.look_4d_active {
            // 4D look: both axes go to rotation_4d
            camera.rotate_4d_look(smooth_yaw, smooth_pitch);
        } else {
            // Standard look: yaw to rotation_4d, pitch separate
            camera.rotate_3d(smooth_yaw, smooth_pitch);
        }

        // ... rest of update ...
    }
}
```

### Task 4.2: Update CameraControl trait

```rust
pub trait CameraControl {
    fn move_local_xz(&mut self, forward: f32, right: f32);
    fn move_y(&mut self, delta: f32);
    fn move_w(&mut self, delta: f32);
    fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32);
    fn rotate_4d_look(&mut self, delta_x: f32, delta_y: f32);  // NEW
    fn rotate_w(&mut self, delta: f32);
    fn rotate_xw(&mut self, delta: f32);
    fn position(&self) -> Vec4;
}
```

---

## Phase 5: Testing

**Session estimate:** 0.5

### Task 5.1: Test SkipY behavior

```rust
#[test]
fn test_skip_y_preserves_y_axis() {
    // After SkipY, transforming (0,1,0,0) should return (0,1,0,0)
    let r = Rotor4::from_euler_xyz(0.5, 0.3, 0.7);
    let mat = r.to_matrix();
    let skip_mat = skip_y_matrix(mat);

    let y_axis = Vec4::new(0.0, 1.0, 0.0, 0.0);
    let result = mat4_transform(skip_mat, y_axis);

    assert!((result.y - 1.0).abs() < 0.001);
    assert!(result.x.abs() < 0.001);
    assert!(result.z.abs() < 0.001);
    assert!(result.w.abs() < 0.001);
}

#[test]
fn test_pitch_separated_from_4d_rotation() {
    let mut cam = Camera4D::new();

    // Apply 4D rotation
    cam.rotate_zw(std::f32::consts::FRAC_PI_2);

    // Up should still be +Y (4D rotation doesn't affect Y)
    let up = cam.up();
    assert!(up.y > 0.99, "Up should be +Y after 4D rotation, got {:?}", up);

    // Now apply pitch
    cam.rotate_3d(0.0, std::f32::consts::FRAC_PI_4);

    // Up should be tilted but forward should still be in XZW plane
    let fwd = cam.forward();
    // Forward Y comes only from pitch, not from 4D rotation
}
```

### Task 5.2: Test movement stays in XZ plane

```rust
#[test]
fn test_movement_stays_horizontal() {
    let mut cam = Camera4D::new();
    cam.position = Vec4::ZERO;

    // Apply some 4D rotation
    cam.rotate_zw(std::f32::consts::FRAC_PI_4);
    cam.rotate_xw(0.3);

    // Move forward
    cam.move_local_xz(1.0, 0.0);

    // Y should be unchanged (movement in XZW plane only)
    assert!(cam.position.y.abs() < 0.001,
        "Forward movement should not affect Y after 4D rotation, got Y={}", cam.position.y);
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/rust4d_math/src/lib.rs` | Add `skip_y_matrix`, `plane_rotation_matrix`, `mat4_mul`, `mat4_transform` |
| `crates/rust4d_math/src/rotor4.rs` | Add `from_euler_xyz` |
| `crates/rust4d_render/src/camera4d.rs` | Complete rewrite with separate pitch storage |
| `crates/rust4d_input/src/camera_controller.rs` | Add 4D look mode, update trait |
| `crates/rust4d_input/src/lib.rs` | Update `CameraControl` trait |

---

## Validation Checklist

- [ ] After 4D rotation, up vector is still aligned with world Y
- [ ] Horizontal mouse movement creates horizontal view rotation
- [ ] Vertical mouse movement only affects pitch
- [ ] Forward movement stays in XZ plane (no Y drift)
- [ ] All existing tests pass (or are updated appropriately)
- [ ] Camera feels like Engine4D

---

## Notes

1. **Why not use quaternions?** We could, but Rotor4 is more general for 4D. The key insight is that Engine4D's "m1 quaternion" only represents 3D rotations that get remapped to 4D via SkipY. Our rotor can do the same if we only use 3D rotation planes (XY, YZ, XZ) for rotation_4d.

2. **Gravity matrix:** Engine4D also has a `gravityMatrix` for walking on slopes. This is out of scope for now but the architecture supports adding it later.

3. **Volume mode:** Engine4D has a "volume mode" toggle that changes camera behavior. We can add this later.

4. **Backwards compatibility:** The old camera tests may need updates since the behavior is fundamentally different.
