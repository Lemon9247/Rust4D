# Camera System Comparison: Rust4D vs Engine4D

**Date:** 2026-01-27

This document provides a detailed comparison between our Rust4D camera implementation and the Engine4D reference implementation to understand why movement still feels wrong.

---

## Executive Summary

The **fundamental architectural difference** is that Engine4D **separates pitch from 4D rotations** using a technique called `SkipY`, while Rust4D combines everything into a single rotor. This separation is crucial for intuitive movement because it keeps the Y axis (gravity/world up) decoupled from 4D rotations.

---

## 1. Rotation Representation

### Engine4D
Uses **two separate components**:
```csharp
public Quaternion m1 = Quaternion.identity;  // 4D rotation (in XZW hyperplane)
public float lookYZ = 0.0f;                   // Pitch (separate, clamped ±89°)
```

### Rust4D
Uses a **single unified rotor**:
```rust
pub orientation: Rotor4,        // All rotations combined
pitch_accumulator: f32,         // Only for clamping, not a separate rotation
```

### Analysis
Engine4D's approach keeps pitch conceptually separate from 4D rotations. When you rotate in 4D, you're only affecting the XZW hyperplane. When you look up/down, you're only affecting the YZ plane. These never interfere.

---

## 2. The SkipY Technique (Critical Difference!)

### Engine4D's SkipY
```csharp
// Transform4D.cs line 278
public static Matrix4x4 SkipY(Quaternion q) {
    return XYZTo(Matrix4x4.Rotate(q), 0, 2, 3);  // X->X, Y->Z, Z->W
}
```

This remaps a 3D quaternion rotation so it **operates in the XZW hyperplane, completely skipping the Y axis**:
- X axis → X axis (unchanged)
- Y axis → Z axis (in 4D)
- Z axis → W axis (in 4D)

**Result:** 4D rotations (m1) NEVER affect the Y axis. The Y axis is always aligned with world up (or gravity direction).

### Rust4D
No equivalent. Our 4D rotations (ZW, XW planes) directly affect all four axes, including Y after certain rotation combinations.

### Why This Matters
When you walk forward in Engine4D, the movement is transformed by `camMatrix`, but the Y component of your position change is determined by gravity and pitch only - never by 4D rotations. In Rust4D, if you rotate in ZW and then XW planes, your "forward" direction could have a Y component, making movement feel floaty or disorienting.

---

## 3. Camera Matrix Construction

### Engine4D
```csharp
// CameraControl4D.cs line 312-317
public Matrix4x4 CreateCamMatrix(Quaternion m1Rot, float yz) {
    // mainRot: pitch rotation in YZ plane
    Matrix4x4 mainRot = Transform4D.Slerp(
        Transform4D.PlaneRotation(yz, 1, 2),      // YZ plane (pitch)
        Transform4D.PlaneRotation(90.0f, 1, 3),   // YW plane (volume mode)
        volumeSmooth);

    // Composition order (right-to-left):
    // 1. mainRot (pitch)
    // 2. SkipY(m1) (4D rotation in XZW, Y unchanged)
    // 3. gravityMatrix (align with surface normal)
    return gravityMatrix * Transform4D.SkipY(m1Rot) * mainRot;
}
```

### Rust4D
```rust
// camera4d.rs - just uses the raw rotor
pub fn rotation_matrix(&self) -> [[f32; 4]; 4] {
    self.orientation.to_matrix()
}
```

### Analysis
Engine4D's camera matrix is carefully constructed to:
1. First apply pitch (looking up/down in YZ plane)
2. Then apply 4D rotation (XZW hyperplane only - Y unchanged!)
3. Finally apply gravity alignment

This ensures that no matter what 4D rotation state you're in, vertical (Y) is always handled consistently.

---

## 4. Movement Transformation

### Engine4D
```csharp
// CameraControl4D.cs line 193-194
if (accel != Vector4.zero) {
    accel = camMatrix * accel;  // Transform ALL movement by camera matrix
    ...
}
```

Movement input is transformed by the **full camera matrix**. Because `SkipY` is used, 4D rotations don't send movement into the Y direction unexpectedly.

### Rust4D
```rust
// camera4d.rs line 112-121
pub fn move_local_xz(&mut self, forward: f32, right: f32) {
    let fwd = self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0));
    let rgt = self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0));

    // Manually project to XYZ, ignoring W component
    self.position.x += fwd.x * forward + rgt.x * right;
    self.position.y += fwd.y * forward + rgt.y * right;
    self.position.z += fwd.z * forward + rgt.z * right;
}
```

We transform the forward/right basis vectors by the full rotor, then manually project movement to XYZ. However, because 4D rotations affect Y, the forward vector can have unexpected Y components.

---

## 5. Looking Controls

### Engine4D (Standard Mode)
```csharp
// CameraControl4D.cs lines 146-153
} else {
    // Normal look (no 4D key held)
    if (volumeMode) {
        m1 = m1 * Quaternion.Euler(-smoothAngY, 0.0f, -smoothAngX);
    } else {
        // HORIZONTAL -> 4D rotation (m1)
        // VERTICAL -> pitch (lookYZ, separate!)
        m1 = m1 * Quaternion.Euler(0.0f, 0.0f, -smoothAngX);
        lookYZ += smoothAngY;
        lookYZ = Mathf.Clamp(lookYZ, -89.0f, 89.0f);
    }
}
```

In standard mode:
- **Horizontal mouse** modifies `m1` with Euler rotation `(0, 0, -angX)` - this is a Z-axis rotation in 3D space, which becomes a ZW rotation after `SkipY`
- **Vertical mouse** modifies `lookYZ` directly (completely separate from m1)

### Engine4D (4D Look Mode - holding left mouse)
```csharp
// CameraControl4D.cs lines 132-137
if (InputManager.GetKey(InputManager.KeyBind.Look4D)) {
    if (volumeMode) {
        m1 = m1 * Quaternion.Euler(0.0f, smoothAngX, 0.0f);
    } else {
        // BOTH horizontal and vertical modify m1
        m1 = m1 * Quaternion.Euler(-smoothAngY, smoothAngX, 0.0f);
    }
}
```

Only when holding the 4D look key does vertical mouse movement go into the 4D rotation.

### Rust4D
```rust
// camera4d.rs lines 60-85
pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
    // Yaw: rotate in (forward, right) plane (view-relative)
    let r_yaw = Rotor4::from_plane_vectors(forward, right, delta_yaw);
    self.orientation = r_yaw.compose(&self.orientation).normalize();

    // Pitch: local YZ plane
    let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, actual_delta_pitch);
    self.orientation = self.orientation.compose(&r_pitch).normalize();
}
```

All looking goes directly into the unified rotor. Pitch is applied in local space, but because it's part of the same rotor as 4D rotations, they can interfere.

---

## 6. Key Behavioral Differences

| Behavior | Engine4D | Rust4D |
|----------|----------|--------|
| Y axis after 4D rotation | Always aligned with gravity | Can be tilted away from world up |
| Pitch accumulation | Separate float, unaffected by 4D | Part of rotor, can be overwritten |
| Forward direction Y component | Only from pitch | From pitch AND 4D rotation |
| Horizontal movement | Always in XZ plane relative to gravity | Can drift into Y after 4D rotation |
| Walking on slopes | Handled by gravity matrix | Not implemented |

---

## 7. Recommended Fixes

### Option A: Match Engine4D Architecture
Separate pitch from 4D rotations:
1. Store `pitch: f32` separately (like Engine4D's `lookYZ`)
2. Store `rotation_4d: Rotor4` for XZW rotations only (like Engine4D's `m1`)
3. Build camera matrix as: `pitch_rotation * skipY(rotation_4d)`
4. Movement is transformed by this combined matrix

**Implementation sketch:**
```rust
pub struct Camera4D {
    pub position: Vec4,
    pub pitch: f32,           // Separate pitch angle
    pub rotation_4d: Rotor4,  // 4D rotation (XZW only)
    slice_offset: f32,
}

impl Camera4D {
    fn camera_matrix(&self) -> [[f32; 4]; 4] {
        // Build pitch rotation in YZ plane
        let pitch_mat = make_yz_rotation(self.pitch);
        // Build 4D rotation in XZW (skip Y)
        let rot_4d_mat = skip_y(self.rotation_4d.to_matrix());
        // Combine
        mat_mul(rot_4d_mat, pitch_mat)
    }
}
```

### Option B: Constrain Movement
Keep unified rotor but constrain movement:
1. Always project XZ movement onto the gravity plane (world XZ)
2. Only allow Y movement explicitly (Space/Shift)
3. This is simpler but doesn't match Engine4D's feel

### Option C: Gravity Matrix
Implement a gravity matrix like Engine4D:
1. Track a "gravity direction" (default: -Y)
2. Smooth gravity transitions when walking on surfaces
3. Use gravity direction to constrain movement

---

## 8. Specific Code Locations

### Engine4D Key Files
- `CameraControl4D.cs:312-318` - Camera matrix construction
- `CameraControl4D.cs:93-156` - Looking input handling
- `CameraControl4D.cs:158-206` - Movement handling
- `Transform4D.cs:278-280` - SkipY implementation

### Rust4D Key Files
- `crates/rust4d_render/src/camera4d.rs:60-85` - rotate_3d
- `crates/rust4d_render/src/camera4d.rs:112-131` - move_local_xz, move_w
- `crates/rust4d_math/src/rotor4.rs` - Rotor implementation

---

## 9. Conclusion

The core issue is **architectural**: Engine4D deliberately keeps Y-axis (gravity) decoupled from 4D rotations using the `SkipY` technique. Our unified rotor approach is mathematically elegant but produces unintuitive movement because 4D rotations can affect the Y axis, making the camera feel like it's "orbiting" or moving in unexpected directions.

**Recommendation:** Implement Option A (match Engine4D architecture) for the most intuitive camera feel. This requires:
1. Separating pitch storage from 4D rotation
2. Implementing a `skip_y` function for matrices
3. Rebuilding camera matrix construction to match Engine4D's composition order
4. Updating movement to use the combined camera matrix
