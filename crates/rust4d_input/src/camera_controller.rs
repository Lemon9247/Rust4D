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

    // Jump state (for physics-based movement)
    jump_pressed: bool,

    // Mouse state
    mouse_pressed: bool,
    w_rotation_mode: bool,  // Right-click held
    pending_yaw: f32,
    pending_pitch: f32,

    // Input smoothing state
    smooth_yaw: f32,
    smooth_pitch: f32,

    // Configuration
    pub move_speed: f32,
    pub w_move_speed: f32,
    pub mouse_sensitivity: f32,
    pub w_rotation_sensitivity: f32,
    pub smoothing_half_life: f32,  // Exponential smoothing half-life in seconds
    pub smoothing_enabled: bool,
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

            jump_pressed: false,

            mouse_pressed: false,
            w_rotation_mode: false,
            pending_yaw: 0.0,
            pending_pitch: 0.0,

            smooth_yaw: 0.0,
            smooth_pitch: 0.0,

            move_speed: 3.0,
            w_move_speed: 2.0,
            mouse_sensitivity: 0.002,  // Standard FPS sensitivity
            w_rotation_sensitivity: 0.005,
            smoothing_half_life: 0.05,  // 50ms half-life when enabled
            smoothing_enabled: false,   // Disabled by default for responsive FPS feel
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
            KeyCode::Space => {
                self.up = pressed;
                // Also track jump for physics mode
                if pressed {
                    self.jump_pressed = true;
                }
                true
            }
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
    ///
    /// When `cursor_captured` is true, free look is enabled (no click required).
    /// Returns the camera position for debug display.
    pub fn update<C: CameraControl>(&mut self, camera: &mut C, dt: f32, cursor_captured: bool) -> Vec4 {
        // Calculate movement deltas
        let fwd = (self.forward as i32 - self.backward as i32) as f32;
        let rgt = (self.right as i32 - self.left as i32) as f32;
        let up_down = (self.up as i32 - self.down as i32) as f32;
        let w = (self.ana as i32 - self.kata as i32) as f32;

        // Apply movement
        camera.move_local_xz(fwd * self.move_speed * dt, rgt * self.move_speed * dt);
        camera.move_y(up_down * self.move_speed * dt);
        camera.move_w(w * self.w_move_speed * dt);

        // Apply exponential smoothing to mouse input (engine4d-style)
        let (yaw_input, pitch_input) = if self.smoothing_enabled && dt > 0.0 {
            // Exponential smoothing: new = old * factor + input * (1 - factor)
            // factor = 2^(-dt / half_life), so smaller half_life = faster response
            let smooth_factor = 2.0f32.powf(-dt / self.smoothing_half_life);
            self.smooth_yaw = self.smooth_yaw * smooth_factor + self.pending_yaw * (1.0 - smooth_factor);
            self.smooth_pitch = self.smooth_pitch * smooth_factor + self.pending_pitch * (1.0 - smooth_factor);
            (self.smooth_yaw, self.smooth_pitch)
        } else {
            // No smoothing - use raw input
            (self.pending_yaw, self.pending_pitch)
        };

        // Apply rotation
        // Free look when cursor is captured, or when mouse button is pressed
        let can_look = cursor_captured || self.mouse_pressed;
        if can_look || self.w_rotation_mode {
            if self.w_rotation_mode {
                // Right-click: W-rotation mode
                // Horizontal mouse: ZW rotation (roll_w)
                // Vertical mouse: XW rotation (roll_xw)
                camera.rotate_w(yaw_input * self.w_rotation_sensitivity);
                camera.rotate_xw(pitch_input * self.w_rotation_sensitivity);
            } else if can_look {
                // Free look: Standard 3D FPS rotation
                // Mouse right (positive delta_x) should turn camera right (positive yaw)
                // Mouse down (positive delta_y) should look down (negative pitch)
                camera.rotate_3d(
                    yaw_input * self.mouse_sensitivity,
                    -pitch_input * self.mouse_sensitivity,
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

    /// Toggle input smoothing on/off
    pub fn toggle_smoothing(&mut self) -> bool {
        self.smoothing_enabled = !self.smoothing_enabled;
        // Reset smoothing state when toggling
        self.smooth_yaw = 0.0;
        self.smooth_pitch = 0.0;
        self.smoothing_enabled
    }

    /// Check if smoothing is enabled
    pub fn is_smoothing_enabled(&self) -> bool {
        self.smoothing_enabled
    }

    /// Consume the jump input flag
    ///
    /// Returns true if jump was pressed since last consume, then clears the flag.
    /// Use this for physics-based movement where jump should trigger once per press.
    pub fn consume_jump(&mut self) -> bool {
        let was_pressed = self.jump_pressed;
        self.jump_pressed = false;
        was_pressed
    }

    /// Get raw movement input for physics-based movement
    ///
    /// Returns (forward, right) input values in range -1.0 to 1.0.
    /// Forward is positive when W is pressed, negative when S is pressed.
    /// Right is positive when D is pressed, negative when A is pressed.
    pub fn get_movement_input(&self) -> (f32, f32) {
        let forward = (self.forward as i32 - self.backward as i32) as f32;
        let right = (self.right as i32 - self.left as i32) as f32;
        (forward, right)
    }

    /// Get W-axis (ana/kata) movement input
    ///
    /// Returns input value in range -1.0 to 1.0.
    /// Positive when Q is pressed (ana), negative when E is pressed (kata).
    pub fn get_w_input(&self) -> f32 {
        (self.ana as i32 - self.kata as i32) as f32
    }

    /// Builder: set movement speed
    pub fn with_move_speed(mut self, speed: f32) -> Self {
        self.move_speed = speed;
        self
    }

    /// Builder: set W-axis movement speed
    pub fn with_w_move_speed(mut self, speed: f32) -> Self {
        self.w_move_speed = speed;
        self
    }

    /// Builder: set mouse sensitivity
    pub fn with_mouse_sensitivity(mut self, sensitivity: f32) -> Self {
        self.mouse_sensitivity = sensitivity;
        self
    }

    /// Builder: set W-axis rotation sensitivity
    pub fn with_w_rotation_sensitivity(mut self, sensitivity: f32) -> Self {
        self.w_rotation_sensitivity = sensitivity;
        self
    }

    /// Builder: set smoothing half-life (lower = more responsive)
    pub fn with_smoothing_half_life(mut self, half_life: f32) -> Self {
        self.smoothing_half_life = half_life;
        self
    }

    /// Builder: enable or disable smoothing
    pub fn with_smoothing(mut self, enabled: bool) -> Self {
        self.smoothing_enabled = enabled;
        self
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
    fn rotate_xw(&mut self, delta: f32);
    fn position(&self) -> Vec4;
}
