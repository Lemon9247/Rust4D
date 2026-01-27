# Engine4D Camera & Input Analysis Report

**Agent**: Engine4D Research Agent
**Date**: 2026-01-26
**Task**: Analyze camera controls and input handling in HackerPoet/Engine4D

---

## Executive Summary

Engine4D uses a **matrix-based approach** for 4D orientation (specifically Unity's `Quaternion` embedded in 4x4 matrices), while Rust4D uses **geometric algebra rotors**. Both are mathematically equivalent for rotations but have different trade-offs. Engine4D's camera system is tightly integrated with its physics system and has sophisticated modal controls for W-rotation.

Key findings:
1. **Mouse look is always active** - no click required for standard 3D rotation
2. **W-rotation uses a dedicated key** (`Look4D` keybind, likely a held modifier)
3. **Movement is camera-relative** and supports full 4D (XYZW) movement
4. **Volume mode** completely changes the control scheme for volumetric rendering

---

## 1. Camera System Architecture

### Engine4D Structure

```
BasicCamera4D (Physical4D)
    |
    +-- position4D: Vector4
    +-- camMatrix: Matrix4x4
    +-- LateUpdate() - sends transforms to shaders
    |
    +--- CameraControl4D (extends BasicCamera4D)
            |
            +-- m1: Quaternion (primary 3D orientation)
            +-- lookYZ: float (pitch accumulator, clamped -89 to +89)
            +-- smoothAngX/Y: float (smoothed mouse input)
            +-- volumeSmooth: float (interpolant for volume mode)
```

### Key Differences from Rust4D

| Aspect | Engine4D | Rust4D |
|--------|----------|--------|
| **Orientation storage** | Quaternion (3D) + lookYZ float | Rotor4 (full 4D) |
| **Position** | Vector4 | Vec4 |
| **Pitch handling** | Separate float, clamped | Part of rotor composition |
| **W-rotation** | Modifies m1 quaternion via Euler | ZW plane rotation via rotor |
| **Matrix generation** | `FromQuaternion()` embedding | `Rotor4::to_matrix()` |

### Engine4D's Clever Pitch Hack

Engine4D avoids gimbal lock by **not composing pitch into the quaternion directly**:

```csharp
// Standard mode (no modifier)
m1 = m1 * Quaternion.Euler(0.0f, 0.0f, -smoothAngX);  // Yaw only in quaternion
lookYZ += smoothAngY;                                   // Pitch tracked separately
lookYZ = Mathf.Clamp(lookYZ, -89.0f, 89.0f);           // Clamped
```

The final view matrix is then constructed by combining `m1` with a pitch rotation. This prevents the classic FPS pitch/yaw interaction issues.

**Rust4D does this differently** by composing all rotations through the rotor:
```rust
let r_yaw = Rotor4::from_plane_angle(RotationPlane::XY, self.yaw);
let r_pitch = Rotor4::from_plane_angle(RotationPlane::XZ, self.pitch);
let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, self.roll_w);
self.orientation = r_yaw.compose(&r_pitch).compose(&r_roll_w).normalize();
```

---

## 2. Input Handling Approach

### Engine4D Input System

Engine4D has a comprehensive `InputManager` static class:

```csharp
// Sensitivity constants
public static float CAM_SMOOTHING = 0.05f;      // Seconds to half-speed
public static float LOOK_SENSITIVITY = 1.0f;
public static float PUTT_SENSITIVITY = 1.0f;
public static float DEADZONE = 0.2f;
```

### Key Bindings (KeyBind enum)

```csharp
// Movement
Left = 0, Right = 1, Forward = 2, Backward = 3,
Kata = 4, Ana = 5,           // 4D movement
Sursum = 6, Deorsum = 7,     // 5D movement

// Rotation modifiers
Look4D = 8,     // Hold for W-rotation mode
Look5D = 9,     // Hold for 5D rotation
LookSpin = 10,  // Hold for spin rotation

// Actions
Putt = 11, Reset = 12, VolumeView = 13,
Run = 20,
```

### Axis Bindings (AxisBind enum)

```csharp
LookHorizontal = 0, LookVertical = 1,      // Mouse/stick look
MoveLeftRight = 2, MoveForwardBack = 3,    // XZ movement
MoveAnaKata = 4, MoveSursumDeorsum = 5,    // WV movement (4D/5D)
Zoom = 6,
```

### Mouse Smoothing Implementation

```csharp
public void HandleLooking() {
    float mouseSmooth = Mathf.Pow(2.0f, -Time.deltaTime / InputManager.CAM_SMOOTHING);
    float angX = InputManager.GetAxis(InputManager.AxisBind.LookHorizontal);
    float angY = InputManager.GetAxis(InputManager.AxisBind.LookVertical);

    // Exponential smoothing
    smoothAngX = smoothAngX * mouseSmooth + angX * (1.0f - mouseSmooth);
    smoothAngY = smoothAngY * mouseSmooth + angY * (1.0f - mouseSmooth);

    // ...apply rotation
}
```

This is **exponential smoothing** with a half-life of `CAM_SMOOTHING` seconds (0.05s = 50ms).

**Rust4D currently uses no smoothing** - mouse delta is applied directly.

---

## 3. 4D Rotation Implementation

### W-Rotation Mode (Engine4D)

When the `Look4D` key is held:

```csharp
if (InputManager.GetKey(InputManager.KeyBind.Look4D)) {
    if (volumeMode) {
        // Volume mode: only yaw
        m1 = m1 * Quaternion.Euler(0.0f, smoothAngX, 0.0f);
    } else {
        // Slice mode: pitch and yaw affect 4D view
        m1 = m1 * Quaternion.Euler(-smoothAngY, smoothAngX, 0.0f);
    }
}
```

In **slice mode**, holding Look4D makes mouse control pitch AND yaw (both axes rotate the 4D view plane), rather than the normal "yaw only in quaternion, pitch separate" approach.

### Default Mode (No Modifier)

```csharp
} else {
    if (volumeMode) {
        m1 = m1 * Quaternion.Euler(-smoothAngY, 0.0f, -smoothAngX);
    } else {
        m1 = m1 * Quaternion.Euler(0.0f, 0.0f, -smoothAngX);  // Roll from horizontal mouse
        lookYZ += smoothAngY;                                   // Pitch from vertical mouse
        lookYZ = Mathf.Clamp(lookYZ, -89.0f, 89.0f);
    }
}
```

**Important insight**: In default slice mode, horizontal mouse controls **roll** (rotation in the XY plane as seen from the camera), not yaw. The `Look4D` modifier enables actual yaw control for 4D navigation.

### Rust4D's Current Approach

```rust
pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
    self.yaw += delta_yaw;
    self.pitch = (self.pitch + delta_pitch).clamp(-1.5, 1.5);
    self.rebuild_orientation();
}

pub fn rotate_w(&mut self, delta: f32) {
    self.roll_w += delta;
    self.rebuild_orientation();
}
```

Rust4D uses a simpler model:
- Standard mouse drag = yaw (XY plane) + pitch (XZ plane)
- Right-click drag = ZW plane rotation (W-roll)

This is **more intuitive for newcomers** but less flexible than Engine4D's modal system.

---

## 4. Movement System

### Engine4D Movement (Camera-Relative)

```csharp
public Vector4 HandleMoving() {
    Vector4 accel = Vector4.zero;

    // Read input axes
    accel.x = InputManager.GetAxis(InputManager.AxisBind.MoveLeftRight);
    // ... key overrides ...
    accel.z = InputManager.GetAxis(InputManager.AxisBind.MoveForwardBack);
    accel.w = InputManager.GetAxis(InputManager.AxisBind.MoveAnaKata);

    // Volume mode interpolation: W movement partially becomes Y
    accel.y = volumeSmooth * accel.w;
    accel.w = (volumeSmooth - 1.0f) * accel.w;

    if (accel != Vector4.zero) {
        // Transform by camera matrix
        accel = camMatrix * accel;

        float mag = Mathf.Min(accel.magnitude, 1.0f);

        // Remove gravity-direction component for ground movement
        if (useGravity) {
            accel -= smoothGravityDirection * Vector4.Dot(smoothGravityDirection, accel);
        }

        accel = accel.normalized * mag;

        if (InputManager.GetKey(InputManager.KeyBind.Run)) {
            accel *= runMultiplier;
        }
    }
    return accel;
}
```

**Key details**:
1. Movement is transformed by `camMatrix` - all movement is camera-relative
2. Gravity projection removes the gravity component for proper ground movement
3. Analog input is normalized but capped at magnitude 1.0
4. Volume mode interpolates W-input between W and Y axes

### Rust4D Movement

```rust
pub fn move_local_xz(&mut self, forward: f32, right: f32) {
    let fwd = self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0));
    let rgt = self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0));

    // Project onto XYZ, ignoring W
    self.position.x += fwd.x * forward + rgt.x * right;
    self.position.y += fwd.y * forward + rgt.y * right;
    self.position.z += fwd.z * forward + rgt.z * right;
}

pub fn move_w(&mut self, delta: f32) {
    self.position.w += delta;
}
```

Rust4D's approach:
1. Forward/right are camera-relative in XYZ
2. W movement is **absolute** (not camera-relative)
3. No gravity handling or ground projection yet

---

## 5. Sensitivity and Speed Values

### Engine4D Constants

```csharp
// CameraControl4D.cs
public const float MOVE_SPEED = 60.0f;        // Units/sec acceleration
public const float JUMP_SPEED = 6.0f;         // Initial jump velocity
public const float PLAYER_RADIUS = 0.3f;      // Collision sphere
public const float CAM_HEIGHT = 1.62f;        // Eye height from feet
public const float GRAVITY_RATE = 90.0f;      // Degrees/sec gravity alignment
public const float ZOOM_RATE = 1.1f;          // Multiplier per input
public const float ZOOM_MAX = 8.0f;
public const float ZOOM_MIN = 0.3f;

// InputManager.cs
public static float CAM_SMOOTHING = 0.05f;    // Smoothing half-life (seconds)
public static float LOOK_SENSITIVITY = 1.0f;  // Mouse multiplier
public static float DEADZONE = 0.2f;          // Joystick deadzone
```

### Rust4D Constants

```rust
// camera_controller.rs
pub move_speed: f32,              // 3.0
pub w_move_speed: f32,            // 2.0
pub mouse_sensitivity: f32,       // 0.003
pub w_rotation_sensitivity: f32,  // 0.005
```

**Comparison**:
- Engine4D uses much higher raw values (60.0 vs 3.0) but these are affected by Unity's deltaTime and frame-independent physics
- Engine4D has more configurable options (zoom, gravity alignment rate)
- Rust4D's values are likely appropriate for its current frame-dependent movement

---

## 6. Volume Mode

Engine4D has a **completely separate control scheme** for volumetric rendering:

```csharp
float volumeSmooth = 0.0f;  // 0 = slice mode, 1 = volume mode

// Rotation changes in volume mode:
if (volumeMode) {
    m1 = m1 * Quaternion.Euler(-smoothAngY, 0.0f, -smoothAngX);
} else {
    m1 = m1 * Quaternion.Euler(0.0f, 0.0f, -smoothAngX);
    lookYZ += smoothAngY;
}

// Movement changes in volume mode:
accel.y = volumeSmooth * accel.w;       // W-input goes to Y
accel.w = (volumeSmooth - 1.0f) * accel.w;  // W-input reduced proportionally
```

In volume mode:
- W-axis input becomes Y-axis movement (since the projection shows W instead of Y)
- Mouse control affects pitch differently
- The transition is smoothed over `VOLUME_TIME` (0.75 seconds)

**Rust4D doesn't have volume mode yet** but this is useful reference for when it does.

---

## 7. Transform4D Plane Rotation

Engine4D constructs rotation matrices for arbitrary planes:

```csharp
public static Matrix4x4 PlaneRotation(float angle, int p1, int p2) {
    float cs = Mathf.Cos(angle * Mathf.Deg2Rad);
    float sn = Mathf.Sin(angle * Mathf.Deg2Rad);

    // Snap exact values to avoid floating-point drift
    if (Mathf.Abs(angle) == 90.0f || angle == 180.0f || angle == 0.0f) {
        cs = Mathf.Round(cs);
        sn = Mathf.Round(sn);
    }

    Matrix4x4 result = Matrix4x4.identity;
    result[p1, p1] = cs;
    result[p2, p2] = cs;
    result[p1, p2] = sn;
    result[p2, p1] = -sn;
    return result;
}
```

This is equivalent to Rust4D's `Rotor4::from_plane_angle()` but in matrix form. The indices `p1` and `p2` select which coordinate axes define the rotation plane.

---

## 8. Key Differences Summary

| Feature | Engine4D | Rust4D |
|---------|----------|--------|
| **Rotation representation** | Quaternion (3D) in Matrix4x4 | Rotor4 (native 4D) |
| **Mouse look** | Always active | Click-to-rotate |
| **W-rotation trigger** | Hold `Look4D` key | Right-click hold |
| **Pitch handling** | Separate float with clamping | Part of rotor, clamped in controller |
| **Input smoothing** | Exponential (50ms half-life) | None |
| **Movement** | Physics-based acceleration | Direct position change |
| **Volume mode** | Full modal switch | Not implemented |
| **Gravity** | Projects movement off gravity vector | Not implemented |

---

## 9. Lessons for Rust4D

### Should Consider

1. **Input smoothing**: Add exponential smoothing for mouse look
   ```rust
   let smooth_factor = 2.0f32.powf(-dt / CAM_SMOOTHING);
   smooth_yaw = smooth_yaw * smooth_factor + raw_yaw * (1.0 - smooth_factor);
   ```

2. **Separate pitch tracking**: Consider extracting pitch to avoid gimbal lock interactions
   ```rust
   // Instead of composing all into rotor
   fn rebuild_orientation(&mut self) {
       let r_yaw = Rotor4::from_plane_angle(RotationPlane::XY, self.yaw);
       let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, self.roll_w);
       self.base_orientation = r_yaw.compose(&r_roll_w).normalize();
       // Apply pitch separately when generating view matrix
   }
   ```

3. **Always-active mouse look**: FPS games typically don't require clicking to look around. Consider making mouse look always active and using a modifier for W-rotation.

4. **Camera-relative W movement**: When the camera is rotated in 4D, W-movement should follow the camera's W-axis, not world W.
   ```rust
   pub fn move_w(&mut self, delta: f32) {
       let w_axis = self.orientation.rotate(Vec4::W);
       self.position = self.position + w_axis * delta;
   }
   ```

### Keep Current Approach

1. **Rotor4 for orientation**: Native 4D representation is mathematically cleaner than Engine4D's 3D quaternion hack

2. **Right-click for W-rotation**: Simpler than a separate keybind for casual users

3. **Direct position changes**: Fine for a free-fly camera; physics-based movement can come later

### Future Considerations

1. **Volume mode**: Plan for a mode switch that changes both input mapping and movement axes

2. **Configurable sensitivity**: Add runtime-adjustable sensitivity values

3. **Controller support**: Engine4D's input system supports gamepads with axis deadzones

---

## 10. Code Snippets Reference

### Engine4D's Full HandleLooking()

```csharp
public void HandleLooking() {
    float mouseSmooth = Mathf.Pow(2.0f, -Time.deltaTime / InputManager.CAM_SMOOTHING);
    float angX = InputManager.GetAxis(InputManager.AxisBind.LookHorizontal);
    float angY = InputManager.GetAxis(InputManager.AxisBind.LookVertical);

    if (Time.deltaTime == 0.0f) {
        smoothAngX = 0.0f;
        smoothAngY = 0.0f;
    } else {
        smoothAngX = smoothAngX * mouseSmooth + angX * (1.0f - mouseSmooth);
        smoothAngY = smoothAngY * mouseSmooth + angY * (1.0f - mouseSmooth);
    }

    if (InputManager.GetKey(InputManager.KeyBind.Look4D)) {
        if (volumeMode) {
            m1 = m1 * Quaternion.Euler(0.0f, smoothAngX, 0.0f);
        } else {
            m1 = m1 * Quaternion.Euler(-smoothAngY, smoothAngX, 0.0f);
        }
    } else {
        if (volumeMode) {
            m1 = m1 * Quaternion.Euler(-smoothAngY, 0.0f, -smoothAngX);
        } else {
            m1 = m1 * Quaternion.Euler(0.0f, 0.0f, -smoothAngX);
            lookYZ += smoothAngY;
            lookYZ = Mathf.Clamp(lookYZ, -89.0f, 89.0f);
        }
    }
}
```

### Engine4D's Transform Matrix Construction

```csharp
public static Matrix4x4 FromQuaternion(Quaternion q) {
    Matrix4x4 r = Matrix4x4.Rotate(q);
    Matrix4x4 matrix = Matrix4x4.identity;
    matrix.SetColumn(0, (Vector4)r.GetColumn(0));
    matrix.SetColumn(1, (Vector4)r.GetColumn(1));
    matrix.SetColumn(2, (Vector4)r.GetColumn(2));
    // Column 3 (W axis) remains identity [0,0,0,1]
    return matrix;
}
```

---

## Appendix: Sources

- [HackerPoet/Engine4D](https://github.com/HackerPoet/Engine4D) GitHub repository
- Files analyzed:
  - `Assets/Scripts/CameraControl4D.cs`
  - `Assets/Scripts/BasicCamera4D.cs`
  - `Assets/Scripts/InputManager.cs`
  - `Assets/Scripts/Transform4D.cs`
  - `Assets/Scripts/Physical4D.cs`

---

*Report generated by Engine4D Research Agent for Rust4D camera controls review swarm*
