//! Rigid body types for 4D physics simulation

use crate::shapes::Collider;
use rust4d_math::Vec4;
use slotmap::new_key_type;

// Define generational key type for rigid bodies
new_key_type! {
    /// Key to a rigid body in the physics world
    ///
    /// Uses generational indexing to prevent the ABA problem where a handle
    /// could point to a reused slot. If a body is removed and its slot reused,
    /// old keys will return None instead of pointing to the wrong body.
    pub struct BodyKey;
}

/// A 4D rigid body with position, velocity, and collision shape
#[derive(Clone, Debug)]
pub struct RigidBody4D {
    /// Position in 4D space (world coordinates)
    pub position: Vec4,
    /// Velocity in 4D space (units per second)
    pub velocity: Vec4,
    /// Mass of the body (used for push calculations)
    pub mass: f32,
    /// Coefficient of restitution (bounciness, 0.0 = no bounce, 1.0 = perfect bounce)
    pub restitution: f32,
    /// Whether this body is affected by gravity
    pub affected_by_gravity: bool,
    /// The collision shape for this body (stores absolute world position)
    pub collider: Collider,
    /// Whether this body is static (static bodies don't move)
    pub is_static: bool,
}

impl RigidBody4D {
    /// Create a new rigid body with a sphere collider
    pub fn new_sphere(position: Vec4, radius: f32) -> Self {
        use crate::shapes::Sphere4D;
        Self {
            position,
            velocity: Vec4::ZERO,
            mass: 1.0,
            restitution: 0.0,
            affected_by_gravity: true,
            collider: Collider::Sphere(Sphere4D::new(position, radius)),
            is_static: false,
        }
    }

    /// Create a new rigid body with an AABB collider
    pub fn new_aabb(position: Vec4, half_extents: Vec4) -> Self {
        use crate::shapes::AABB4D;
        Self {
            position,
            velocity: Vec4::ZERO,
            mass: 1.0,
            restitution: 0.0,
            affected_by_gravity: true,
            collider: Collider::AABB(AABB4D::from_center_half_extents(position, half_extents)),
            is_static: false,
        }
    }

    /// Create a static body that doesn't move
    pub fn new_static_aabb(position: Vec4, half_extents: Vec4) -> Self {
        let mut body = Self::new_aabb(position, half_extents);
        body.is_static = true;
        body.affected_by_gravity = false;
        body
    }

    /// Set the velocity of this body
    pub fn with_velocity(mut self, velocity: Vec4) -> Self {
        self.velocity = velocity;
        self
    }

    /// Set the mass of this body
    pub fn with_mass(mut self, mass: f32) -> Self {
        self.mass = mass;
        self
    }

    /// Set the restitution (bounciness) of this body
    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution.clamp(0.0, 1.0);
        self
    }

    /// Set whether this body is affected by gravity
    pub fn with_gravity(mut self, affected: bool) -> Self {
        self.affected_by_gravity = affected;
        self
    }

    /// Set whether this body is static
    pub fn with_static(mut self, is_static: bool) -> Self {
        self.is_static = is_static;
        if is_static {
            self.affected_by_gravity = false;
        }
        self
    }

    /// Update the position and sync the collider
    pub fn set_position(&mut self, position: Vec4) {
        let delta = position - self.position;
        self.position = position;
        self.collider = self.collider.translated(delta);
    }

    /// Apply a positional correction (e.g., from collision resolution)
    pub fn apply_correction(&mut self, correction: Vec4) {
        self.position = self.position + correction;
        self.collider = self.collider.translated(correction);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sphere_body() {
        let pos = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let body = RigidBody4D::new_sphere(pos, 0.5);

        assert_eq!(body.position, pos);
        assert_eq!(body.velocity, Vec4::ZERO);
        assert_eq!(body.mass, 1.0);
        assert_eq!(body.restitution, 0.0);
        assert!(body.affected_by_gravity);
        assert!(!body.is_static);

        // Check collider is properly set
        assert_eq!(body.collider.center(), pos);
    }

    #[test]
    fn test_new_aabb_body() {
        let pos = Vec4::new(1.0, 2.0, 3.0, 4.0);
        let half_extents = Vec4::new(0.5, 1.0, 0.5, 0.5);
        let body = RigidBody4D::new_aabb(pos, half_extents);

        assert_eq!(body.position, pos);
        assert_eq!(body.collider.center(), pos);
    }

    #[test]
    fn test_static_body() {
        let pos = Vec4::new(0.0, 0.0, 0.0, 0.0);
        let body = RigidBody4D::new_static_aabb(pos, Vec4::new(1.0, 1.0, 1.0, 1.0));

        assert!(body.is_static);
        assert!(!body.affected_by_gravity);
    }

    #[test]
    fn test_builder_methods() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_velocity(Vec4::new(1.0, 2.0, 0.0, 0.0))
            .with_mass(5.0)
            .with_restitution(0.8)
            .with_gravity(false);

        assert_eq!(body.velocity, Vec4::new(1.0, 2.0, 0.0, 0.0));
        assert_eq!(body.mass, 5.0);
        assert_eq!(body.restitution, 0.8);
        assert!(!body.affected_by_gravity);
    }

    #[test]
    fn test_restitution_clamping() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_restitution(1.5);
        assert_eq!(body.restitution, 1.0);

        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_restitution(-0.5);
        assert_eq!(body.restitution, 0.0);
    }

    #[test]
    fn test_set_position() {
        let mut body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0);
        let new_pos = Vec4::new(5.0, 10.0, 3.0, 0.0);

        body.set_position(new_pos);

        assert_eq!(body.position, new_pos);
        assert_eq!(body.collider.center(), new_pos);
    }

    #[test]
    fn test_apply_correction() {
        let mut body = RigidBody4D::new_sphere(Vec4::new(1.0, 0.0, 0.0, 0.0), 1.0);
        let correction = Vec4::new(0.0, 0.5, 0.0, 0.0);

        body.apply_correction(correction);

        assert_eq!(body.position, Vec4::new(1.0, 0.5, 0.0, 0.0));
        assert_eq!(body.collider.center(), Vec4::new(1.0, 0.5, 0.0, 0.0));
    }

    #[test]
    fn test_with_static_disables_gravity() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_gravity(true)
            .with_static(true);

        assert!(body.is_static);
        assert!(!body.affected_by_gravity);
    }
}
