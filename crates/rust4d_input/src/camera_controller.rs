//! Camera controller for 4D Golf-style input handling
//!
//! Controls:
//! - W/S: Forward/backward (Z)
//! - A/D: Left/right strafe (X)
//! - Q/E: Ana/kata movement (W)
//! - Space/Shift: Up/down (Y)
//! - Mouse drag: 3D camera rotation
//! - Right-click + drag: W-axis rotation

use rust4d_math::Vec4;
use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

/// Camera controller for handling input
pub struct CameraController {
    // Movement state
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    ana: bool,     // Q - move toward +W (ana)
    kata: bool,    // E - move toward -W (kata)

    // Mouse state
    mouse_pressed: bool,
    w_rotation_mode: bool,  // Right-click held
    pending_yaw: f32,
    pending_pitch: f32,

    // Configuration
    pub move_speed: f32,
    pub w_move_speed: f32,
    pub mouse_sensitivity: f32,
    pub w_rotation_sensitivity: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraController {
    pub fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            ana: false,
            kata: false,

            mouse_pressed: false,
            w_rotation_mode: false,
            pending_yaw: 0.0,
            pending_pitch: 0.0,

            move_speed: 3.0,
            w_move_speed: 2.0,
            mouse_sensitivity: 0.003,
            w_rotation_sensitivity: 0.005,
        }
    }

    /// Process keyboard input
    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;

        match key {
            KeyCode::KeyW => { self.forward = pressed; true }
            KeyCode::KeyS => { self.backward = pressed; true }
            KeyCode::KeyA => { self.left = pressed; true }
            KeyCode::KeyD => { self.right = pressed; true }
            KeyCode::KeyQ => { self.ana = pressed; true }
            KeyCode::KeyE => { self.kata = pressed; true }
            KeyCode::Space => { self.up = pressed; true }
            KeyCode::ShiftLeft | KeyCode::ShiftRight => { self.down = pressed; true }
            _ => false,
        }
    }

    /// Process mouse button input
    pub fn process_mouse_button(&mut self, button: MouseButton, state: ElementState) {
        let pressed = state == ElementState::Pressed;

        match button {
            MouseButton::Left => {
                self.mouse_pressed = pressed;
            }
            MouseButton::Right => {
                self.w_rotation_mode = pressed;
            }
            _ => {}
        }
    }

    /// Process mouse movement
    pub fn process_mouse_motion(&mut self, delta_x: f64, delta_y: f64) {
        self.pending_yaw += delta_x as f32;
        self.pending_pitch += delta_y as f32;
    }

    /// Update the camera based on accumulated input
    /// Returns the camera position for debug display
    pub fn update<C: CameraControl>(&mut self, camera: &mut C, dt: f32) -> Vec4 {
        // Calculate movement deltas
        let fwd = (self.forward as i32 - self.backward as i32) as f32;
        let rgt = (self.right as i32 - self.left as i32) as f32;
        let up_down = (self.up as i32 - self.down as i32) as f32;
        let w = (self.ana as i32 - self.kata as i32) as f32;

        // Apply movement
        camera.move_local_xz(fwd * self.move_speed * dt, rgt * self.move_speed * dt);
        camera.move_y(up_down * self.move_speed * dt);
        camera.move_w(w * self.w_move_speed * dt);

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

        // Reset pending mouse movement
        self.pending_yaw = 0.0;
        self.pending_pitch = 0.0;

        camera.position()
    }

    /// Check if any movement keys are pressed
    pub fn is_moving(&self) -> bool {
        self.forward || self.backward || self.left || self.right
            || self.up || self.down || self.ana || self.kata
    }
}

/// Trait for camera control
/// Allows the controller to work with different camera implementations
pub trait CameraControl {
    fn move_local_xz(&mut self, forward: f32, right: f32);
    fn move_y(&mut self, delta: f32);
    fn move_w(&mut self, delta: f32);
    fn rotate_3d(&mut self, delta_yaw: f32, delta_pitch: f32);
    fn rotate_w(&mut self, delta: f32);
    fn position(&self) -> Vec4;
}
