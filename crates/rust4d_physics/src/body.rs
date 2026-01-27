//! Rigid body types for 4D physics simulation

use crate::collision::CollisionFilter;
use crate::material::PhysicsMaterial;
use crate::shapes::{Collider, Plane4D};
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

/// Type of rigid body that determines how it's simulated
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BodyType {
    /// Full physics simulation with gravity and collision response
    #[default]
    Dynamic,
    /// Never moves, used for floors, walls, platforms
    Static,
    /// User-controlled velocity, no gravity (ideal for player characters)
    Kinematic,
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
    /// Physical material properties (friction and restitution)
    pub material: PhysicsMaterial,
    /// The collision shape for this body (stores absolute world position)
    pub collider: Collider,
    /// Type of body (Dynamic, Static, or Kinematic)
    pub body_type: BodyType,
    /// Whether this body is touching the ground (set by physics step)
    pub grounded: bool,
    /// Collision filter (layer membership and collision mask)
    pub filter: CollisionFilter,
}

impl RigidBody4D {
    /// Check if this body is affected by gravity
    #[inline]
    pub fn affected_by_gravity(&self) -> bool {
        self.body_type == BodyType::Dynamic
    }

    /// Check if this body is static (never moves)
    #[inline]
    pub fn is_static(&self) -> bool {
        self.body_type == BodyType::Static
    }

    /// Check if this body is kinematic (user-controlled, no gravity)
    #[inline]
    pub fn is_kinematic(&self) -> bool {
        self.body_type == BodyType::Kinematic
    }
}

// Additional RigidBody4D constructors and builder methods
impl RigidBody4D {
    /// Create a new rigid body with a sphere collider
    pub fn new_sphere(position: Vec4, radius: f32) -> Self {
        use crate::shapes::Sphere4D;
        Self {
            position,
            velocity: Vec4::ZERO,
            mass: 1.0,
            material: PhysicsMaterial::default(),
            collider: Collider::Sphere(Sphere4D::new(position, radius)),
            body_type: BodyType::Dynamic,
            grounded: false,
            filter: CollisionFilter::default(),
        }
    }

    /// Create a new rigid body with an AABB collider
    pub fn new_aabb(position: Vec4, half_extents: Vec4) -> Self {
        use crate::shapes::AABB4D;
        Self {
            position,
            velocity: Vec4::ZERO,
            mass: 1.0,
            material: PhysicsMaterial::default(),
            collider: Collider::AABB(AABB4D::from_center_half_extents(position, half_extents)),
            body_type: BodyType::Dynamic,
            grounded: false,
            filter: CollisionFilter::default(),
        }
    }

    /// Create a static body that doesn't move
    pub fn new_static_aabb(position: Vec4, half_extents: Vec4) -> Self {
        Self::new_aabb(position, half_extents).with_body_type(BodyType::Static)
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

    /// Set the physics material for this body
    pub fn with_material(mut self, material: PhysicsMaterial) -> Self {
        self.material = material;
        self
    }

    /// Set the restitution (bounciness) of this body
    ///
    /// This is a convenience method that updates the material's restitution.
    /// For full control over friction and restitution, use `with_material()`.
    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.material.restitution = restitution.clamp(0.0, 1.0);
        self
    }

    /// Set the body type (Dynamic, Static, or Kinematic)
    pub fn with_body_type(mut self, body_type: BodyType) -> Self {
        self.body_type = body_type;
        self
    }

    /// Set whether this body is affected by gravity (legacy API)
    ///
    /// Sets body_type to Dynamic if gravity is enabled, otherwise keeps current type.
    /// For new code, prefer `with_body_type()`.
    pub fn with_gravity(mut self, affected: bool) -> Self {
        if !affected && self.body_type == BodyType::Dynamic {
            // If disabling gravity on a dynamic body, it becomes kinematic
            self.body_type = BodyType::Kinematic;
        } else if affected && self.body_type == BodyType::Kinematic {
            // If enabling gravity on a kinematic body, it becomes dynamic
            self.body_type = BodyType::Dynamic;
        }
        self
    }

    /// Set whether this body is static (legacy API)
    ///
    /// For new code, prefer `with_body_type()`.
    pub fn with_static(mut self, is_static: bool) -> Self {
        if is_static {
            self.body_type = BodyType::Static;
        } else if self.body_type == BodyType::Static {
            self.body_type = BodyType::Dynamic;
        }
        self
    }

    /// Set the collision filter for this body
    pub fn with_filter(mut self, filter: CollisionFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Set the collision layer (which layer this body belongs to)
    pub fn with_layer(mut self, layer: crate::collision::CollisionLayer) -> Self {
        self.filter.layer = layer;
        self
    }

    /// Set the collision mask (which layers this body can collide with)
    pub fn with_mask(mut self, mask: crate::collision::CollisionLayer) -> Self {
        self.filter.mask = mask;
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

/// A collider that doesn't move (floors, walls, platforms)
///
/// Static colliders are checked for collision with all dynamic bodies
/// but never move themselves.
#[derive(Clone, Debug)]
pub struct StaticCollider {
    /// The collision shape
    pub collider: Collider,
    /// Physics material (friction and restitution)
    pub material: PhysicsMaterial,
    /// Collision filter (layer membership and collision mask)
    pub filter: CollisionFilter,
}

impl StaticCollider {
    /// Create a new static collider with the given shape and material
    pub fn new(collider: Collider, material: PhysicsMaterial) -> Self {
        Self {
            collider,
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create a plane collider
    pub fn plane(normal: Vec4, distance: f32, material: PhysicsMaterial) -> Self {
        Self {
            collider: Collider::Plane(Plane4D::new(normal, distance)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create a horizontal floor plane at the given Y height
    pub fn floor(y: f32, material: PhysicsMaterial) -> Self {
        Self {
            collider: Collider::Plane(Plane4D::floor(y)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create a bounded floor platform using AABB collision
    ///
    /// Objects can fall off the edges of this platform.
    /// The floor surface is at Y height `y`, with the collider extending downward.
    ///
    /// # Parameters
    /// - `y`: Y height of floor surface (top of AABB)
    /// - `half_size_xz`: Half-extent in X and Z dimensions
    /// - `half_size_w`: Half-extent in W dimension
    /// - `thickness`: Thickness in Y (minimum 0.1 enforced to prevent tunneling)
    /// - `material`: Physics material for friction and restitution
    pub fn floor_bounded(
        y: f32,
        half_size_xz: f32,
        half_size_w: f32,
        thickness: f32,
        material: PhysicsMaterial,
    ) -> Self {
        use crate::shapes::AABB4D;

        let actual_thickness = thickness.max(0.1);
        let half_thickness = actual_thickness / 2.0;

        // Position AABB so top surface is at y
        let center = Vec4::new(0.0, y - half_thickness, 0.0, 0.0);
        let half_extents = Vec4::new(half_size_xz, half_thickness, half_size_xz, half_size_w);

        Self {
            collider: Collider::AABB(AABB4D::from_center_half_extents(center, half_extents)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Create an AABB static collider
    pub fn aabb(center: Vec4, half_extents: Vec4, material: PhysicsMaterial) -> Self {
        use crate::shapes::AABB4D;
        Self {
            collider: Collider::AABB(AABB4D::from_center_half_extents(center, half_extents)),
            material,
            filter: CollisionFilter::static_world(),
        }
    }

    /// Set the collision filter for this static collider
    pub fn with_filter(mut self, filter: CollisionFilter) -> Self {
        self.filter = filter;
        self
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
        assert_eq!(body.material, PhysicsMaterial::default());
        assert!(body.affected_by_gravity());
        assert!(!body.is_static());

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

        assert!(body.is_static());
        assert!(!body.affected_by_gravity());
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
        assert_eq!(body.material.restitution, 0.8);
        assert!(!body.affected_by_gravity());
    }

    #[test]
    fn test_restitution_clamping() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_restitution(1.5);
        assert_eq!(body.material.restitution, 1.0);

        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0).with_restitution(-0.5);
        assert_eq!(body.material.restitution, 0.0);
    }

    #[test]
    fn test_with_material() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_material(PhysicsMaterial::RUBBER);

        assert_eq!(body.material, PhysicsMaterial::RUBBER);
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

        assert!(body.is_static());
        assert!(!body.affected_by_gravity());
    }

    // ===== Collision Filter Tests =====

    #[test]
    fn test_default_filter() {
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0);
        assert_eq!(body.filter, CollisionFilter::default());
    }

    #[test]
    fn test_with_filter() {
        use crate::collision::CollisionLayer;
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_filter(CollisionFilter::player());

        assert_eq!(body.filter.layer, CollisionLayer::PLAYER);
    }

    #[test]
    fn test_with_layer() {
        use crate::collision::CollisionLayer;
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_layer(CollisionLayer::ENEMY);

        assert_eq!(body.filter.layer, CollisionLayer::ENEMY);
    }

    #[test]
    fn test_with_mask() {
        use crate::collision::CollisionLayer;
        let body = RigidBody4D::new_sphere(Vec4::ZERO, 1.0)
            .with_mask(CollisionLayer::STATIC | CollisionLayer::ENEMY);

        assert!(body.filter.mask.contains(CollisionLayer::STATIC));
        assert!(body.filter.mask.contains(CollisionLayer::ENEMY));
        assert!(!body.filter.mask.contains(CollisionLayer::PLAYER));
    }

    #[test]
    fn test_static_collider_default_filter() {
        let collider = StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE);
        assert_eq!(collider.filter, CollisionFilter::static_world());
    }

    #[test]
    fn test_static_collider_with_filter() {
        use crate::collision::CollisionLayer;
        let collider = StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE)
            .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));

        assert_eq!(collider.filter.layer, CollisionLayer::TRIGGER);
    }

    // ===== Bounded Floor Tests =====

    #[test]
    fn test_floor_bounded_creates_aabb() {
        use crate::shapes::Collider;
        let collider = StaticCollider::floor_bounded(
            0.0,   // y: floor surface at y=0
            10.0,  // half_size_xz
            5.0,   // half_size_w
            1.0,   // thickness
            PhysicsMaterial::CONCRETE,
        );

        // Should be an AABB, not a plane
        match &collider.collider {
            Collider::AABB(aabb) => {
                // Top surface should be at y=0
                assert_eq!(aabb.max.y, 0.0);
                // Bottom should be at y=-1.0 (thickness=1.0)
                assert_eq!(aabb.min.y, -1.0);
                // X/Z extents should be -10 to +10
                assert_eq!(aabb.min.x, -10.0);
                assert_eq!(aabb.max.x, 10.0);
                assert_eq!(aabb.min.z, -10.0);
                assert_eq!(aabb.max.z, 10.0);
                // W extent should be -5 to +5
                assert_eq!(aabb.min.w, -5.0);
                assert_eq!(aabb.max.w, 5.0);
            }
            _ => panic!("Expected AABB collider from floor_bounded"),
        }

        assert_eq!(collider.filter, CollisionFilter::static_world());
    }

    #[test]
    fn test_floor_bounded_minimum_thickness() {
        use crate::shapes::Collider;
        // Thickness below 0.1 should be clamped to 0.1
        let collider = StaticCollider::floor_bounded(
            5.0,   // y: floor surface at y=5
            1.0,   // half_size_xz
            1.0,   // half_size_w
            0.01,  // thickness (too thin, should be clamped to 0.1)
            PhysicsMaterial::RUBBER,
        );

        match &collider.collider {
            Collider::AABB(aabb) => {
                // Top surface at y=5
                assert_eq!(aabb.max.y, 5.0);
                // Bottom should be at y=4.9 (minimum thickness 0.1)
                assert!((aabb.min.y - 4.9).abs() < 0.001);
            }
            _ => panic!("Expected AABB collider"),
        }
    }
}
