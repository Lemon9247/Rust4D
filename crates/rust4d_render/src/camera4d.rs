//! 4D Camera with 6 degrees of freedom
//!
//! The camera has a 4D position and orientation (using a Rotor4).
//! It supports:
//! - Standard 3D movement (forward/backward, left/right, up/down)
//! - 4D movement along the W axis (ana/kata)
//! - Standard 3D rotation (yaw, pitch)
//! - 4D rotation in the ZW plane (W-roll)

use rust4d_math::{Vec4, Rotor4, RotationPlane};
use rust4d_input::CameraControl;

/// 4D Camera for viewing 4D space
pub struct Camera4D {
    /// 4D position (x, y, z, w)
    pub position: Vec4,
    /// 6-DoF orientation as a rotor
    pub orientation: Rotor4,
    /// Cross-section offset from camera W position
    pub slice_offset: f32,

    // Euler-like angles for incremental control
    pitch: f32,      // XZ plane rotation
    yaw: f32,        // XY plane rotation
    roll_w: f32,     // ZW plane rotation (4D Golf's W-rotation)
}

impl Default for Camera4D {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera4D {
    /// Create a new camera at the default position
    pub fn new() -> Self {
        Self {
            position: Vec4::new(0.0, 0.0, 5.0, 0.0),
            orientation: Rotor4::IDENTITY,
            slice_offset: 0.0,
            pitch: 0.0,
            yaw: 0.0,
            roll_w: 0.0,
        }
    }

    /// Standard 3D mouse look (yaw and pitch)
    pub fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        // Clamp pitch to avoid gimbal lock issues
        self.pitch = (self.pitch + delta_pitch).clamp(-1.5, 1.5);
        self.rebuild_orientation();
    }

    /// 4D W-rotation in the ZW plane
    pub fn rotate_w(&mut self, delta: f32) {
        self.roll_w += delta;
        self.rebuild_orientation();
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

    /// Move along the W axis (ana/kata)
    pub fn move_w(&mut self, delta: f32) {
        self.position.w += delta;
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

    /// Get the 4x4 rotation matrix for the camera orientation
    pub fn rotation_matrix(&self) -> [[f32; 4]; 4] {
        self.orientation.to_matrix()
    }

    /// Rebuild the orientation rotor from the Euler-like angles
    fn rebuild_orientation(&mut self) {
        // Compose rotations: yaw * pitch * roll_w
        let r_yaw = Rotor4::from_plane_angle(RotationPlane::XY, self.yaw);
        let r_pitch = Rotor4::from_plane_angle(RotationPlane::XZ, self.pitch);
        let r_roll_w = Rotor4::from_plane_angle(RotationPlane::ZW, self.roll_w);

        self.orientation = r_yaw.compose(&r_pitch).compose(&r_roll_w).normalize();
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

    fn position(&self) -> Vec4 {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_default_position() {
        let cam = Camera4D::new();
        assert_eq!(cam.position.z, 5.0);
        assert_eq!(cam.position.w, 0.0);
    }

    #[test]
    fn test_camera_move_w() {
        let mut cam = Camera4D::new();
        cam.move_w(1.0);
        assert_eq!(cam.position.w, 1.0);
    }

    #[test]
    fn test_camera_slice_w() {
        let mut cam = Camera4D::new();
        cam.position.w = 2.0;
        cam.slice_offset = 0.5;
        assert_eq!(cam.get_slice_w(), 2.5);
    }
}
