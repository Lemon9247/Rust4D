//! Player physics for FPS-style movement in 4D
//!
//! Provides player movement with gravity, jumping, and floor collision.

use crate::collision::sphere_vs_plane;
use crate::shapes::{Plane4D, Sphere4D};
use rust4d_math::Vec4;

/// Default player collision radius
pub const DEFAULT_PLAYER_RADIUS: f32 = 0.5;

/// Default jump velocity
pub const DEFAULT_JUMP_VELOCITY: f32 = 8.0;

/// Player physics state
///
/// Handles position, velocity, gravity, jumping, and floor collision.
/// The player is represented as a sphere for collision purposes.
#[derive(Clone, Debug)]
pub struct PlayerPhysics {
    /// Current position (center of player sphere)
    pub position: Vec4,
    /// Current velocity
    pub velocity: Vec4,
    /// Collision radius
    pub radius: f32,
    /// Whether the player is touching the ground
    pub grounded: bool,
    /// Upward velocity applied when jumping
    pub jump_velocity: f32,
}

impl PlayerPhysics {
    /// Create a new player at the given position
    pub fn new(position: Vec4) -> Self {
        Self {
            position,
            velocity: Vec4::ZERO,
            radius: DEFAULT_PLAYER_RADIUS,
            grounded: false,
            jump_velocity: DEFAULT_JUMP_VELOCITY,
        }
    }

    /// Create a new player with custom radius and jump velocity
    pub fn with_config(position: Vec4, radius: f32, jump_velocity: f32) -> Self {
        Self {
            position,
            velocity: Vec4::ZERO,
            radius,
            grounded: false,
            jump_velocity,
        }
    }

    /// Get the player's collision sphere at the current position
    pub fn collider(&self) -> Sphere4D {
        Sphere4D::new(self.position, self.radius)
    }

    /// Apply horizontal movement input to velocity (XZ plane only)
    ///
    /// This sets the horizontal velocity directly based on movement input.
    /// The Y component is ignored to prevent flying via movement input.
    pub fn apply_movement(&mut self, movement: Vec4) {
        // Only apply movement on XZ plane (ignore Y, keep W for 4D movement)
        self.velocity.x = movement.x;
        self.velocity.z = movement.z;
        // Optionally allow W movement for 4D navigation
        self.velocity.w = movement.w;
    }

    /// Attempt to jump if grounded
    ///
    /// Sets vertical velocity to jump_velocity if the player is on the ground.
    pub fn jump(&mut self) {
        if self.grounded {
            self.velocity.y = self.jump_velocity;
            self.grounded = false;
        }
    }

    /// Simulate one physics step
    ///
    /// Applies gravity, integrates velocity, and resolves floor collision.
    ///
    /// # Arguments
    /// * `dt` - Time step in seconds
    /// * `gravity` - Gravity acceleration (typically negative, e.g., -20.0)
    /// * `floor` - The floor plane to collide with
    pub fn step(&mut self, dt: f32, gravity: f32, floor: &Plane4D) {
        // Apply gravity to velocity
        self.velocity.y += gravity * dt;

        // Integrate velocity to update position
        self.position += self.velocity * dt;

        // Check for floor collision using a small margin for ground detection
        // This prevents floating point issues where the player flickers between
        // grounded and airborne states when resting on the floor.
        const GROUND_MARGIN: f32 = 0.01;

        let collider = self.collider();
        let height_above_floor = floor.signed_distance(self.position) - self.radius;

        if let Some(contact) = sphere_vs_plane(&collider, floor) {
            if contact.is_colliding() {
                // Push the player out of the floor
                self.position += contact.normal * contact.penetration;

                // If we hit the floor from above (normal pointing up), we're grounded
                if contact.normal.y > 0.5 {
                    self.grounded = true;
                    // Zero out vertical velocity when landing
                    if self.velocity.y < 0.0 {
                        self.velocity.y = 0.0;
                    }
                }
            }
        } else if height_above_floor <= GROUND_MARGIN && self.velocity.y <= 0.0 {
            // Very close to ground and not moving up - consider grounded
            self.grounded = true;
            // Snap to floor to prevent drift
            self.position.y = floor.distance + self.radius;
            if self.velocity.y < 0.0 {
                self.velocity.y = 0.0;
            }
        } else {
            // Not touching floor and not close enough, we're in the air
            self.grounded = false;
        }
    }

    /// Check if the player is currently in the air (not grounded)
    pub fn is_airborne(&self) -> bool {
        !self.grounded
    }

    /// Get the player's current height above the floor
    pub fn height_above_floor(&self, floor: &Plane4D) -> f32 {
        floor.signed_distance(self.position) - self.radius
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAVITY: f32 = -20.0;
    const EPSILON: f32 = 0.0001;

    fn floor() -> Plane4D {
        Plane4D::floor(0.0)
    }

    #[test]
    fn test_new_player() {
        let pos = Vec4::new(0.0, 5.0, 0.0, 0.0);
        let player = PlayerPhysics::new(pos);

        assert_eq!(player.position, pos);
        assert_eq!(player.velocity, Vec4::ZERO);
        assert_eq!(player.radius, DEFAULT_PLAYER_RADIUS);
        assert!(!player.grounded);
        assert_eq!(player.jump_velocity, DEFAULT_JUMP_VELOCITY);
    }

    #[test]
    fn test_collider() {
        let pos = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let player = PlayerPhysics::new(pos);
        let collider = player.collider();

        assert_eq!(collider.center, pos);
        assert_eq!(collider.radius, DEFAULT_PLAYER_RADIUS);
    }

    #[test]
    fn test_apply_movement_xz_only() {
        let mut player = PlayerPhysics::new(Vec4::ZERO);
        player.velocity.y = 5.0; // Existing vertical velocity

        // Apply movement with Y component (should be ignored for X/Z)
        player.apply_movement(Vec4::new(3.0, 10.0, 4.0, 1.0));

        assert_eq!(player.velocity.x, 3.0);
        // Y velocity should remain unchanged by apply_movement
        // (apply_movement only sets X, Z, W)
        assert_eq!(player.velocity.z, 4.0);
        assert_eq!(player.velocity.w, 1.0);
    }

    #[test]
    fn test_jump_when_grounded() {
        let mut player = PlayerPhysics::new(Vec4::new(0.0, 0.5, 0.0, 0.0));
        player.grounded = true;

        player.jump();

        assert_eq!(player.velocity.y, DEFAULT_JUMP_VELOCITY);
        assert!(!player.grounded);
    }

    #[test]
    fn test_jump_when_airborne() {
        let mut player = PlayerPhysics::new(Vec4::new(0.0, 5.0, 0.0, 0.0));
        player.grounded = false;
        player.velocity.y = -2.0; // Falling

        player.jump();

        // Should not jump when airborne
        assert_eq!(player.velocity.y, -2.0);
        assert!(!player.grounded);
    }

    #[test]
    fn test_gravity_applied() {
        let mut player = PlayerPhysics::new(Vec4::new(0.0, 10.0, 0.0, 0.0));
        let floor = floor();

        let initial_y = player.position.y;
        player.step(0.1, GRAVITY, &floor);

        // Gravity should have been applied
        assert!(player.velocity.y < 0.0);
        // Should have moved down
        assert!(player.position.y < initial_y);
    }

    #[test]
    fn test_floor_collision() {
        // Player starting just above floor
        let mut player = PlayerPhysics::new(Vec4::new(0.0, 0.6, 0.0, 0.0));
        player.velocity.y = -5.0; // Falling fast
        let floor = floor();

        // Take several steps to fall and hit floor
        for _ in 0..10 {
            player.step(0.1, GRAVITY, &floor);
        }

        // Should be grounded and not below floor
        assert!(player.grounded);
        // Position should be at or above the floor + radius
        assert!(player.position.y >= player.radius - EPSILON);
        // Vertical velocity should be zero or positive
        assert!(player.velocity.y >= 0.0);
    }

    #[test]
    fn test_resting_on_floor() {
        // Player exactly on floor
        let mut player = PlayerPhysics::new(Vec4::new(0.0, 0.5, 0.0, 0.0));
        player.grounded = true;
        let floor = floor();

        // Step should keep player grounded
        player.step(0.016, GRAVITY, &floor);

        assert!(player.grounded);
        // Should not fall through floor
        assert!(player.position.y >= player.radius - EPSILON);
    }

    #[test]
    fn test_horizontal_movement_preserved() {
        let mut player = PlayerPhysics::new(Vec4::new(0.0, 0.5, 0.0, 0.0));
        player.grounded = true;
        player.apply_movement(Vec4::new(5.0, 0.0, 3.0, 0.0));
        let floor = floor();

        let initial_x = player.position.x;
        let initial_z = player.position.z;
        player.step(0.1, GRAVITY, &floor);

        // Should have moved horizontally
        assert!((player.position.x - (initial_x + 0.5)).abs() < EPSILON);
        assert!((player.position.z - (initial_z + 0.3)).abs() < EPSILON);
    }

    #[test]
    fn test_jump_and_land() {
        let mut player = PlayerPhysics::new(Vec4::new(0.0, 0.5, 0.0, 0.0));
        player.grounded = true;
        let floor = floor();

        // Jump
        player.jump();
        assert!(!player.grounded);
        assert_eq!(player.velocity.y, DEFAULT_JUMP_VELOCITY);

        // Simulate until landing
        for _ in 0..100 {
            player.step(0.016, GRAVITY, &floor);
            if player.grounded {
                break;
            }
        }

        // Should have landed
        assert!(player.grounded);
        assert!(player.position.y >= player.radius - EPSILON);
    }

    #[test]
    fn test_height_above_floor() {
        let player = PlayerPhysics::new(Vec4::new(0.0, 5.0, 0.0, 0.0));
        let floor = floor();

        let height = player.height_above_floor(&floor);
        // Height should be position.y - radius = 5.0 - 0.5 = 4.5
        assert!((height - 4.5).abs() < EPSILON);
    }

    #[test]
    fn test_is_airborne() {
        let mut player = PlayerPhysics::new(Vec4::ZERO);

        player.grounded = false;
        assert!(player.is_airborne());

        player.grounded = true;
        assert!(!player.is_airborne());
    }

    #[test]
    fn test_with_config() {
        let pos = Vec4::new(1.0, 2.0, 3.0, 0.0);
        let player = PlayerPhysics::with_config(pos, 1.0, 10.0);

        assert_eq!(player.position, pos);
        assert_eq!(player.radius, 1.0);
        assert_eq!(player.jump_velocity, 10.0);
    }
}
