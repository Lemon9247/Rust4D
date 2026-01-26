//! 4D Camera with 6 degrees of freedom
//!
//! The camera has a 4D position and orientation (using a Rotor4).
//! It supports:
//! - Standard 3D movement (forward/backward, left/right, up/down)
//! - 4D movement along the W axis (ana/kata)
//! - Standard 3D rotation (yaw, pitch)
//! - 4D rotation in the ZW plane (W-roll)
//!
//! The camera uses an **incremental rotation** approach to avoid gimbal lock.
//! Rotations are applied relative to the camera's current orientation rather
//! than recomposing from absolute Euler angles.

use rust4d_math::{Vec4, Rotor4, RotationPlane};
use rust4d_input::CameraControl;

/// 4D Camera for viewing 4D space
///
/// Uses incremental rotations to avoid gimbal-lock-like issues in 4D.
/// The orientation is stored as a rotor and rotations are applied
/// incrementally in the camera's local frame.
pub struct Camera4D {
    /// 4D position (x, y, z, w)
    pub position: Vec4,
    /// 6-DoF orientation as a rotor (accumulated incrementally)
    pub orientation: Rotor4,
    /// Cross-section offset from camera W position
    pub slice_offset: f32,

    /// Accumulated pitch for clamping (prevents looking past vertical)
    pitch_accumulator: f32,
}

impl Default for Camera4D {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera4D {
    /// Pitch clamp limit: ±89° to avoid gimbal lock (engine4d uses same value)
    const PITCH_LIMIT: f32 = 1.553; // ~89 degrees in radians

    /// Create a new camera at the default position
    pub fn new() -> Self {
        Self {
            position: Vec4::new(0.0, 0.0, 5.0, 0.0),
            orientation: Rotor4::IDENTITY,
            slice_offset: 0.0,
            pitch_accumulator: 0.0,
        }
    }

    /// Standard 3D mouse look (yaw and pitch)
    ///
    /// Uses incremental rotation to avoid gimbal lock:
    /// - Yaw is applied in world space (around world Y axis) to keep horizon level
    /// - Pitch is applied in camera-local space (around camera's right axis)
    pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
        // Clamp pitch to prevent looking past vertical
        let new_pitch = (self.pitch_accumulator + delta_pitch).clamp(-Self::PITCH_LIMIT, Self::PITCH_LIMIT);
        let actual_delta_pitch = new_pitch - self.pitch_accumulator;
        self.pitch_accumulator = new_pitch;

        // Yaw: rotate in world XZ plane (around world Y axis)
        // This keeps the horizon level regardless of current orientation
        if delta_yaw.abs() > 0.0001 {
            let r_yaw = Rotor4::from_plane_angle(RotationPlane::XZ, delta_yaw);
            // Apply yaw in world space: new_orientation = orientation * r_yaw
            self.orientation = self.orientation.compose(&r_yaw).normalize();
        }

        // Pitch: rotate in camera-local YZ plane (around camera's right axis)
        // This is applied in local space so it works correctly after any yaw
        if actual_delta_pitch.abs() > 0.0001 {
            let r_pitch = Rotor4::from_plane_angle(RotationPlane::YZ, actual_delta_pitch);
            // Apply pitch in local space: new_orientation = orientation * r_pitch
            self.orientation = self.orientation.compose(&r_pitch).normalize();
        }
    }

    /// 4D W-rotation in the ZW plane (camera-local)
    ///
    /// Rotates the camera's view into the 4th dimension.
    /// Applied in camera-local space so it works correctly with any orientation.
    pub fn rotate_w(&mut self, delta: f32) {
        if delta.abs() > 0.0001 {
            let r_zw = Rotor4::from_plane_angle(RotationPlane::ZW, delta);
            // Apply in local space
            self.orientation = self.orientation.compose(&r_zw).normalize();
        }
    }

    /// 4D XW rotation (camera-local)
    ///
    /// Rotates the camera in the XW plane (4D tilt).
    /// Applied in camera-local space.
    pub fn rotate_xw(&mut self, delta: f32) {
        if delta.abs() > 0.0001 {
            let r_xw = Rotor4::from_plane_angle(RotationPlane::XW, delta);
            // Apply in local space
            self.orientation = self.orientation.compose(&r_xw).normalize();
        }
    }

    /// Move in the camera-local XZ plane (forward/backward, left/right)
    pub fn move_local_xz(&mut self, forward: f32, right: f32) {
        // Get forward and right vectors in world space
        let fwd = self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0));
        let rgt = self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0));

        // Project movement onto the XYZ plane (ignore W component for XZ movement)
        self.position.x += fwd.x * forward + rgt.x * right;
        self.position.y += fwd.y * forward + rgt.y * right;
        self.position.z += fwd.z * forward + rgt.z * right;
    }

    /// Move along the camera-local W axis (ana/kata)
    ///
    /// When the camera is rotated in 4D, Q/E moves along the camera's W-axis,
    /// not the world W-axis. This matches engine4d behavior.
    pub fn move_w(&mut self, delta: f32) {
        // Get the W-axis direction in world space
        let w_axis = self.orientation.rotate(Vec4::new(0.0, 0.0, 0.0, 1.0));
        self.position = self.position + w_axis * delta;
    }

    /// Move up/down along Y axis
    pub fn move_y(&mut self, delta: f32) {
        self.position.y += delta;
    }

    /// Get the W-coordinate for cross-section slicing
    pub fn get_slice_w(&self) -> f32 {
        self.position.w + self.slice_offset
    }

    /// Adjust the slice offset
    pub fn adjust_slice_offset(&mut self, delta: f32) {
        self.slice_offset += delta;
    }

    /// Reset camera to the default starting position and orientation
    pub fn reset(&mut self) {
        self.position = Vec4::new(0.0, 0.0, 5.0, 0.0);
        self.orientation = Rotor4::IDENTITY;
        self.slice_offset = 0.0;
        self.pitch_accumulator = 0.0;
    }

    /// Get the forward direction vector
    pub fn forward(&self) -> Vec4 {
        self.orientation.rotate(Vec4::new(0.0, 0.0, -1.0, 0.0))
    }

    /// Get the right direction vector
    pub fn right(&self) -> Vec4 {
        self.orientation.rotate(Vec4::new(1.0, 0.0, 0.0, 0.0))
    }

    /// Get the up direction vector
    pub fn up(&self) -> Vec4 {
        self.orientation.rotate(Vec4::new(0.0, 1.0, 0.0, 0.0))
    }

    /// Get the W (ana) direction vector
    pub fn ana(&self) -> Vec4 {
        self.orientation.rotate(Vec4::new(0.0, 0.0, 0.0, 1.0))
    }

    /// Get the 4x4 rotation matrix for the camera orientation
    pub fn rotation_matrix(&self) -> [[f32; 4]; 4] {
        self.orientation.to_matrix()
    }
}

impl CameraControl for Camera4D {
    fn move_local_xz(&mut self, forward: f32, right: f32) {
        Camera4D::move_local_xz(self, forward, right);
    }

    fn move_y(&mut self, delta: f32) {
        Camera4D::move_y(self, delta);
    }

    fn move_w(&mut self, delta: f32) {
        Camera4D::move_w(self, delta);
    }

    fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
        Camera4D::rotate_3d(self, delta_yaw, delta_pitch);
    }

    fn rotate_w(&mut self, delta: f32) {
        Camera4D::rotate_w(self, delta);
    }

    fn rotate_xw(&mut self, delta: f32) {
        Camera4D::rotate_xw(self, delta);
    }

    fn position(&self) -> Vec4 {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::FRAC_PI_2;

    const EPSILON: f32 = 0.0001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_camera_default_position() {
        let cam = Camera4D::new();
        assert_eq!(cam.position.z, 5.0);
        assert_eq!(cam.position.w, 0.0);
    }

    #[test]
    fn test_camera_move_w() {
        let mut cam = Camera4D::new();
        cam.position = Vec4::ZERO; // Start at origin
        cam.move_w(1.0);
        // With identity orientation, W axis points in +W direction
        assert!(approx_eq(cam.position.w, 1.0), "Expected W=1.0, got W={}", cam.position.w);
    }

    #[test]
    fn test_camera_slice_w() {
        let mut cam = Camera4D::new();
        cam.position.w = 2.0;
        cam.slice_offset = 0.5;
        assert_eq!(cam.get_slice_w(), 2.5);
    }

    #[test]
    fn test_yaw_rotates_in_xz_plane() {
        // Yaw (horizontal turning) should rotate in XZ plane around Y axis
        // Positive yaw = turn right (clockwise from above)
        let mut cam = Camera4D::new();
        cam.rotate_3d(FRAC_PI_2, 0.0);  // 90° yaw right

        let fwd = cam.forward();
        // Initially forward is -Z. After 90° yaw right (positive), forward should be +X
        // (X→Z in XZ plane, so -Z→+X)
        assert!(approx_eq(fwd.x, 1.0), "Expected X=1.0, got X={}", fwd.x);
        assert!(approx_eq(fwd.y, 0.0), "Expected Y=0.0, got Y={}", fwd.y);
        assert!(approx_eq(fwd.z, 0.0), "Expected Z=0.0, got Z={}", fwd.z);
    }

    #[test]
    fn test_pitch_rotates_in_yz_plane() {
        // Pitch (looking up/down) should rotate in YZ plane around X axis
        let mut cam = Camera4D::new();
        cam.rotate_3d(0.0, FRAC_PI_2);  // 90° pitch up (but clamped to ~89°)

        let fwd = cam.forward();
        // After pitch up, forward should point up (+Y) with minimal Z
        assert!(fwd.y > 0.9, "Expected Y>0.9, got Y={}", fwd.y);
        assert!(approx_eq(fwd.x, 0.0), "Expected X~0, got X={}", fwd.x);
    }

    #[test]
    fn test_pitch_clamped_at_89_degrees() {
        let mut cam = Camera4D::new();
        // Try to pitch way past 90 degrees
        cam.rotate_3d(0.0, 10.0);

        // Should be clamped to ~89 degrees (1.553 rad)
        let fwd = cam.forward();
        // At 89°, forward should be mostly up but not perfectly vertical
        assert!(fwd.y > 0.99, "Forward should be nearly vertical, got Y={}", fwd.y);
        assert!(fwd.z < 0.0, "Forward should have slight negative Z, got Z={}", fwd.z);
    }

    #[test]
    fn test_camera_relative_w_movement() {
        // After rotating in ZW plane, W movement should follow camera's W axis
        let mut cam = Camera4D::new();
        cam.position = Vec4::ZERO;

        // Rotate 90° in ZW plane
        cam.rotate_w(FRAC_PI_2);

        // Now move in W direction
        cam.move_w(1.0);

        // After ZW rotation, the camera's W axis should be rotated
        // The W basis vector rotated by 90° in ZW becomes -Z
        assert!(approx_eq(cam.position.z, -1.0), "Expected Z=-1.0, got Z={}", cam.position.z);
        assert!(approx_eq(cam.position.w, 0.0), "Expected W=0.0, got W={}", cam.position.w);
    }

    #[test]
    fn test_reset_clears_all_rotation() {
        let mut cam = Camera4D::new();
        cam.rotate_3d(1.0, 0.5);
        cam.rotate_w(0.3);
        cam.rotate_xw(0.2);

        cam.reset();

        assert!(approx_eq(cam.forward().z, -1.0), "Forward should be -Z after reset");
        assert!(approx_eq(cam.up().y, 1.0), "Up should be +Y after reset");
    }

    #[test]
    fn test_yaw_after_zw_rotation() {
        // Test that yaw still works correctly after 4D rotation
        // With incremental rotation, yaw is in world space so horizon stays level
        let mut cam = Camera4D::new();

        // First rotate 90° in ZW plane (looking into 4D)
        cam.rotate_w(FRAC_PI_2);

        // Now apply yaw - should turn camera right in world XZ plane
        cam.rotate_3d(FRAC_PI_2, 0.0);

        let fwd = cam.forward();
        let up = cam.up();

        println!("After ZW + yaw: forward = ({:.3}, {:.3}, {:.3}, {:.3})", fwd.x, fwd.y, fwd.z, fwd.w);
        println!("After ZW + yaw: up = ({:.3}, {:.3}, {:.3}, {:.3})", up.x, up.y, up.z, up.w);

        // Up should still be +Y because yaw is in world space
        assert!(up.y > 0.9, "Up should still be mostly +Y, got Y={}", up.y);

        // Forward should have moved toward +X in the 3D view
        // (The exact result depends on whether we're viewing into W or not)
    }

    #[test]
    fn test_incremental_rotation_preserves_orthogonality() {
        // Test that incremental rotations preserve orthogonality of basis vectors
        let mut cam = Camera4D::new();

        // Apply a series of rotations
        cam.rotate_3d(0.5, 0.3);
        cam.rotate_w(0.4);
        cam.rotate_3d(-0.2, 0.1);
        cam.rotate_xw(0.25);

        let fwd = cam.forward();
        let right = cam.right();
        let up = cam.up();
        let ana = cam.ana();

        // Check all vectors are unit length (allow 0.5% tolerance for accumulated error)
        assert!((fwd.length() - 1.0).abs() < 0.005, "Forward not unit: {}", fwd.length());
        assert!((right.length() - 1.0).abs() < 0.005, "Right not unit: {}", right.length());
        assert!((up.length() - 1.0).abs() < 0.005, "Up not unit: {}", up.length());
        assert!((ana.length() - 1.0).abs() < 0.005, "Ana not unit: {}", ana.length());

        // Check orthogonality (dot products should be 0)
        assert!(fwd.dot(right).abs() < 0.005, "Forward not orthogonal to Right: {}", fwd.dot(right));
        assert!(fwd.dot(up).abs() < 0.005, "Forward not orthogonal to Up: {}", fwd.dot(up));
        assert!(fwd.dot(ana).abs() < 0.005, "Forward not orthogonal to Ana: {}", fwd.dot(ana));
        assert!(right.dot(up).abs() < 0.005, "Right not orthogonal to Up: {}", right.dot(up));
        assert!(right.dot(ana).abs() < 0.005, "Right not orthogonal to Ana: {}", right.dot(ana));
        assert!(up.dot(ana).abs() < 0.005, "Up not orthogonal to Ana: {}", up.dot(ana));
    }

    #[test]
    fn test_yaw_keeps_horizon_level() {
        // Yaw should not affect the up vector (keeps horizon level)
        let mut cam = Camera4D::new();

        // Apply some pitch first
        cam.rotate_3d(0.0, 0.5);

        let up_before = cam.up();

        // Now yaw
        cam.rotate_3d(1.0, 0.0);

        let up_after = cam.up();

        // Up vector Y component should be preserved (horizon stays level)
        // The up vector rotates with yaw in world XZ plane, so Y stays the same
        assert!(approx_eq(up_before.y, up_after.y),
            "Yaw should preserve up.Y: before={}, after={}", up_before.y, up_after.y);
    }

    #[test]
    fn test_pitch_is_local() {
        // Pitch should be in camera-local space
        let mut cam = Camera4D::new();

        // Yaw 90° right
        cam.rotate_3d(FRAC_PI_2, 0.0);

        // Now pitch up - should pitch relative to camera's current orientation
        cam.rotate_3d(0.0, FRAC_PI_2 * 0.5); // ~45° pitch

        let fwd = cam.forward();

        println!("After yaw + pitch: forward = ({:.3}, {:.3}, {:.3}, {:.3})", fwd.x, fwd.y, fwd.z, fwd.w);

        // After yaw, camera was facing +X
        // After pitch, forward should be between +X and +Y
        assert!(fwd.x > 0.5, "Forward should have positive X");
        assert!(fwd.y > 0.3, "Forward should have positive Y after pitching up");
    }

    #[test]
    fn test_movement_follows_orientation() {
        let mut cam = Camera4D::new();
        cam.position = Vec4::ZERO;

        // Yaw 90° right (now facing +X)
        cam.rotate_3d(FRAC_PI_2, 0.0);

        // Move forward
        cam.move_local_xz(1.0, 0.0);

        // Should have moved in +X direction
        assert!(cam.position.x > 0.9, "Should move in +X after yawing right, got X={}", cam.position.x);
        assert!(cam.position.z.abs() < 0.1, "Should not move in Z after yawing right, got Z={}", cam.position.z);
    }

    #[test]
    fn test_movement_after_4d_rotation() {
        let mut cam = Camera4D::new();
        cam.position = Vec4::ZERO;

        // Rotate in ZW plane
        cam.rotate_w(FRAC_PI_2);

        // Yaw 90° right
        cam.rotate_3d(FRAC_PI_2, 0.0);

        // Move forward
        cam.move_local_xz(1.0, 0.0);

        // Should still have moved primarily in +X direction (yaw is world-space)
        assert!(cam.position.x > 0.5, "Yaw should still work in world XZ plane after ZW rotation, got X={}", cam.position.x);
    }
}
