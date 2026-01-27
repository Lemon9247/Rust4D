//! Physics world and simulation

use crate::body::{BodyKey, RigidBody4D, StaticCollider};
use crate::collision::{aabb_vs_aabb, aabb_vs_plane, sphere_vs_aabb, sphere_vs_plane, Contact};
use crate::shapes::{Collider, Sphere4D};
use rust4d_math::Vec4;
use slotmap::SlotMap;

/// Configuration for the physics simulation
#[derive(Clone, Debug)]
pub struct PhysicsConfig {
    /// Gravity acceleration (applied to Y-axis, negative = down)
    pub gravity: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: -20.0,
        }
    }
}

impl PhysicsConfig {
    /// Create a new physics config with the given gravity
    pub fn new(gravity: f32) -> Self {
        Self { gravity }
    }
}

/// The physics world containing all rigid bodies
pub struct PhysicsWorld {
    /// All rigid bodies in the world (using generational keys)
    bodies: SlotMap<BodyKey, RigidBody4D>,
    /// Static colliders (floors, walls, platforms)
    static_colliders: Vec<StaticCollider>,
    /// Physics configuration
    pub config: PhysicsConfig,
}

impl PhysicsWorld {
    /// Create a new physics world with default configuration
    pub fn new() -> Self {
        Self::with_config(PhysicsConfig::default())
    }

    /// Create a new physics world with custom configuration
    pub fn with_config(config: PhysicsConfig) -> Self {
        Self {
            bodies: SlotMap::with_key(),
            static_colliders: Vec::new(),
            config,
        }
    }

    /// Add a static collider to the world
    pub fn add_static_collider(&mut self, collider: StaticCollider) {
        self.static_colliders.push(collider);
    }

    /// Get immutable access to static colliders
    pub fn static_colliders(&self) -> &[StaticCollider] {
        &self.static_colliders
    }

    /// Add a body to the world and return its key
    pub fn add_body(&mut self, body: RigidBody4D) -> BodyKey {
        self.bodies.insert(body)
    }

    /// Remove a body from the world and return it
    pub fn remove_body(&mut self, key: BodyKey) -> Option<RigidBody4D> {
        self.bodies.remove(key)
    }

    /// Get an immutable reference to a body by key
    pub fn get_body(&self, key: BodyKey) -> Option<&RigidBody4D> {
        self.bodies.get(key)
    }

    /// Get a mutable reference to a body by key
    pub fn get_body_mut(&mut self, key: BodyKey) -> Option<&mut RigidBody4D> {
        self.bodies.get_mut(key)
    }

    /// Get the number of bodies in the world
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    /// Iterate over all body keys
    pub fn body_keys(&self) -> impl Iterator<Item = BodyKey> + '_ {
        self.bodies.keys()
    }

    /// Step the physics simulation forward by dt seconds
    ///
    /// This performs:
    /// 1. Gravity application to non-static bodies with gravity enabled
    /// 2. Velocity integration into position
    /// 3. Static collider collision detection and resolution
    /// 4. Body-body collision detection and resolution
    pub fn step(&mut self, dt: f32) {
        // Phase 1: Apply gravity and integrate velocity
        for (_key, body) in &mut self.bodies {
            if body.is_static {
                continue;
            }

            // Apply gravity
            if body.affected_by_gravity {
                body.velocity.y += self.config.gravity * dt;
            }

            // Integrate velocity into position
            let displacement = body.velocity * dt;
            body.position = body.position + displacement;
            body.collider = body.collider.translated(displacement);
        }

        // Phase 2: Resolve static collider collisions
        self.resolve_static_collisions();

        // Phase 3: Resolve body-body collisions
        self.resolve_body_collisions();
    }

    /// Check for collision between a body collider and a static collider
    fn check_static_collision(body_collider: &Collider, static_collider: &Collider) -> Option<Contact> {
        match (body_collider, static_collider) {
            // Body sphere vs static plane
            (Collider::Sphere(sphere), Collider::Plane(plane)) => {
                sphere_vs_plane(sphere, plane)
            }
            // Body AABB vs static plane
            (Collider::AABB(aabb), Collider::Plane(plane)) => {
                aabb_vs_plane(aabb, plane)
            }
            // Body sphere vs static AABB
            (Collider::Sphere(sphere), Collider::AABB(aabb)) => {
                sphere_vs_aabb(sphere, aabb)
            }
            // Body AABB vs static AABB
            (Collider::AABB(body_aabb), Collider::AABB(static_aabb)) => {
                aabb_vs_aabb(body_aabb, static_aabb)
            }
            // Body sphere vs static sphere (rare but possible)
            (Collider::Sphere(body_sphere), Collider::Sphere(static_sphere)) => {
                Self::sphere_vs_sphere(body_sphere, static_sphere)
            }
            // Body AABB vs static sphere
            (Collider::AABB(aabb), Collider::Sphere(sphere)) => {
                // Flip the result since sphere_vs_aabb returns normal pointing from AABB to sphere
                sphere_vs_aabb(sphere, aabb).map(|mut c| {
                    c.normal = -c.normal;
                    c
                })
            }
            // Plane colliders don't move so body can't be a plane
            (Collider::Plane(_), _) => None,
        }
    }

    /// Sphere vs sphere collision (returns contact from sphere A toward B)
    fn sphere_vs_sphere(a: &Sphere4D, b: &Sphere4D) -> Option<Contact> {
        let delta = b.center - a.center;
        let dist_sq = delta.length_squared();
        let min_dist = a.radius + b.radius;

        if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
            let dist = dist_sq.sqrt();
            let penetration = min_dist - dist;
            let normal = delta.normalized();
            let point = a.center + normal * a.radius;
            Some(Contact::new(point, normal, penetration))
        } else {
            None
        }
    }

    /// Resolve collisions between bodies and static colliders
    fn resolve_static_collisions(&mut self) {
        for (_key, body) in &mut self.bodies {
            if body.is_static {
                continue;
            }

            for static_col in &self.static_colliders {
                let contact = Self::check_static_collision(&body.collider, &static_col.collider);

                if let Some(contact) = contact {
                    if contact.is_colliding() {
                        // Push the body out of the static collider
                        let correction = contact.normal * contact.penetration;
                        body.apply_correction(correction);

                        // Combine body and static collider materials
                        let combined = body.material.combine(&static_col.material);

                        // Handle velocity response
                        let velocity_along_normal = body.velocity.dot(contact.normal);
                        if velocity_along_normal < 0.0 {
                            // Body is moving into the collider
                            // Remove the normal component of velocity and optionally bounce
                            let normal_velocity = contact.normal * velocity_along_normal;
                            body.velocity = body.velocity - normal_velocity * (1.0 + combined.restitution);

                            // Apply friction to horizontal (tangent) velocity
                            let tangent_velocity = body.velocity - contact.normal * body.velocity.dot(contact.normal);
                            let tangent_speed = tangent_velocity.length();

                            if tangent_speed > 0.0001 {
                                let friction_factor = 1.0 - combined.friction;
                                body.velocity = contact.normal * body.velocity.dot(contact.normal)
                                              + tangent_velocity * friction_factor;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Resolve collisions between bodies
    fn resolve_body_collisions(&mut self) {
        // Collect all keys first (needed because we can't iterate and mutate)
        let keys: Vec<BodyKey> = self.bodies.keys().collect();
        let key_count = keys.len();

        // Check all pairs of bodies
        for i in 0..key_count {
            for j in (i + 1)..key_count {
                let key_a = keys[i];
                let key_b = keys[j];

                // Get colliders for both bodies
                let (collider_a, collider_b, is_static_a, is_static_b) = {
                    let body_a = &self.bodies[key_a];
                    let body_b = &self.bodies[key_b];
                    (body_a.collider, body_b.collider, body_a.is_static, body_b.is_static)
                };

                // Skip if both bodies are static
                if is_static_a && is_static_b {
                    continue;
                }

                // Check for collision based on collider types
                // The contact normal convention: points FROM body A TOWARD body B
                let contact = match (&collider_a, &collider_b) {
                    (Collider::Sphere(a), Collider::Sphere(b)) => {
                        Self::sphere_vs_sphere(a, b)
                    }
                    (Collider::Sphere(sphere), Collider::AABB(aabb)) => {
                        // sphere_vs_aabb returns normal pointing from AABB toward sphere
                        // We want normal from A (sphere) toward B (AABB), so flip it
                        sphere_vs_aabb(sphere, aabb).map(|mut c| {
                            c.normal = -c.normal;
                            c
                        })
                    }
                    (Collider::AABB(aabb), Collider::Sphere(sphere)) => {
                        // sphere_vs_aabb returns normal pointing from AABB toward sphere
                        // We want normal from A (AABB) toward B (sphere), which is already correct
                        sphere_vs_aabb(sphere, aabb)
                    }
                    (Collider::AABB(a), Collider::AABB(b)) => {
                        // aabb_vs_aabb returns normal pointing from B toward A
                        // We want normal from A toward B, so flip it
                        aabb_vs_aabb(a, b).map(|mut c| {
                            c.normal = -c.normal;
                            c
                        })
                    }
                    // Plane colliders are only used for static colliders
                    (Collider::Plane(_), _) | (_, Collider::Plane(_)) => None,
                };

                if let Some(contact) = contact {
                    if contact.is_colliding() {
                        self.resolve_body_pair_collision(key_a, key_b, &contact, is_static_a, is_static_b);
                    }
                }
            }
        }
    }

    /// Resolve collision between two specific bodies
    fn resolve_body_pair_collision(
        &mut self,
        key_a: BodyKey,
        key_b: BodyKey,
        contact: &crate::collision::Contact,
        is_static_a: bool,
        is_static_b: bool,
    ) {
        // Determine how to split the correction
        let (correction_a, correction_b) = if is_static_a {
            // Only move B
            (Vec4::ZERO, contact.normal * contact.penetration)
        } else if is_static_b {
            // Only move A
            (-contact.normal * contact.penetration, Vec4::ZERO)
        } else {
            // Split based on mass
            let mass_a = self.bodies[key_a].mass;
            let mass_b = self.bodies[key_b].mass;
            let total_mass = mass_a + mass_b;

            let ratio_a = mass_b / total_mass;
            let ratio_b = mass_a / total_mass;

            (
                -contact.normal * contact.penetration * ratio_a,
                contact.normal * contact.penetration * ratio_b,
            )
        };

        // Apply position corrections
        if !is_static_a {
            self.bodies[key_a].apply_correction(correction_a);
        }
        if !is_static_b {
            self.bodies[key_b].apply_correction(correction_b);
        }

        // Combine materials from both bodies
        let combined = self.bodies[key_a].material.combine(&self.bodies[key_b].material);

        // Handle velocity response with restitution
        if !is_static_a {
            let vel_along_normal = self.bodies[key_a].velocity.dot(-contact.normal);
            if vel_along_normal < 0.0 {
                let normal_velocity = -contact.normal * vel_along_normal;
                self.bodies[key_a].velocity = self.bodies[key_a].velocity - normal_velocity * (1.0 + combined.restitution);

                // Apply friction to tangent velocity
                let tangent_velocity = self.bodies[key_a].velocity - (-contact.normal) * self.bodies[key_a].velocity.dot(-contact.normal);
                let tangent_speed = tangent_velocity.length();
                if tangent_speed > 0.0001 {
                    let friction_factor = 1.0 - combined.friction;
                    self.bodies[key_a].velocity = (-contact.normal) * self.bodies[key_a].velocity.dot(-contact.normal)
                                                + tangent_velocity * friction_factor;
                }
            }
        }

        if !is_static_b {
            let vel_along_normal = self.bodies[key_b].velocity.dot(contact.normal);
            if vel_along_normal < 0.0 {
                let normal_velocity = contact.normal * vel_along_normal;
                self.bodies[key_b].velocity = self.bodies[key_b].velocity - normal_velocity * (1.0 + combined.restitution);

                // Apply friction to tangent velocity
                let tangent_velocity = self.bodies[key_b].velocity - contact.normal * self.bodies[key_b].velocity.dot(contact.normal);
                let tangent_speed = tangent_velocity.length();
                if tangent_speed > 0.0001 {
                    let friction_factor = 1.0 - combined.friction;
                    self.bodies[key_b].velocity = contact.normal * self.bodies[key_b].velocity.dot(contact.normal)
                                                + tangent_velocity * friction_factor;
                }
            }
        }
    }
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::PhysicsMaterial;

    #[test]
    fn test_physics_config_default() {
        let config = PhysicsConfig::default();
        assert_eq!(config.gravity, -20.0);
    }

    #[test]
    fn test_physics_config_custom() {
        let config = PhysicsConfig::new(-10.0);
        assert_eq!(config.gravity, -10.0);
    }

    /// Helper to create a world with a floor at the given Y position
    fn world_with_floor(gravity: f32, floor_y: f32, floor_material: PhysicsMaterial) -> PhysicsWorld {
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(gravity));
        world.add_static_collider(StaticCollider::floor(floor_y, floor_material));
        world
    }

    #[test]
    fn test_world_add_body() {
        let mut world = PhysicsWorld::new();
        assert_eq!(world.body_count(), 0);

        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
        let key = world.add_body(body);

        // Key should be valid and retrievable
        assert!(world.get_body(key).is_some());
        assert_eq!(world.body_count(), 1);
    }

    #[test]
    fn test_world_get_body() {
        let mut world = PhysicsWorld::new();
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
        let handle = world.add_body(body);

        let retrieved = world.get_body(handle).expect("Body should exist");
        assert_eq!(retrieved.position, Vec4::new(0.0, 5.0, 0.0, 0.0));
    }

    #[test]
    fn test_world_get_body_mut() {
        let mut world = PhysicsWorld::new();
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
        let handle = world.add_body(body);

        {
            let body_mut = world.get_body_mut(handle).expect("Body should exist");
            body_mut.velocity = Vec4::new(1.0, 0.0, 0.0, 0.0);
        }

        let retrieved = world.get_body(handle).expect("Body should exist");
        assert_eq!(retrieved.velocity, Vec4::new(1.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_stale_key_returns_none() {
        let mut world = PhysicsWorld::new();
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
        let key = world.add_body(body);

        // Key is valid initially
        assert!(world.get_body(key).is_some());

        // Remove the body
        let removed = world.remove_body(key);
        assert!(removed.is_some());

        // Key is now stale - should return None
        assert!(world.get_body(key).is_none());

        // Add a new body - it gets a different key
        let new_body = RigidBody4D::new_sphere(Vec4::new(1.0, 5.0, 0.0, 0.0), 0.5);
        let new_key = world.add_body(new_body);

        // Old key still returns None (generational safety)
        assert!(world.get_body(key).is_none());
        // New key works
        assert!(world.get_body(new_key).is_some());
    }

    #[test]
    fn test_gravity_application() {
        let mut world = PhysicsWorld::new();
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5);
        let handle = world.add_body(body);

        // Step for 0.1 seconds
        world.step(0.1);

        let body = world.get_body(handle).unwrap();
        // Velocity should have gravity applied: 0 + (-20) * 0.1 = -2.0
        assert!((body.velocity.y - (-2.0)).abs() < 0.0001);
    }

    #[test]
    fn test_velocity_integration() {
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(10.0, 0.0, 0.0, 0.0));
        let handle = world.add_body(body);

        world.step(1.0);

        let body = world.get_body(handle).unwrap();
        // Position should have moved: 0 + 10 * 1.0 = 10.0
        assert!((body.position.x - 10.0).abs() < 0.0001);
    }

    #[test]
    fn test_static_body_does_not_move() {
        let mut world = PhysicsWorld::new();
        let body = RigidBody4D::new_static_aabb(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));
        let handle = world.add_body(body);

        world.step(1.0);

        let body = world.get_body(handle).unwrap();
        assert_eq!(body.position, Vec4::ZERO);
        assert_eq!(body.velocity, Vec4::ZERO);
    }

    #[test]
    fn test_floor_collision() {
        let mut world = world_with_floor(-20.0, 0.0, PhysicsMaterial::CONCRETE);
        // Sphere starting below the floor (partially penetrating)
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.3, 0.0, 0.0), 0.5)
            .with_gravity(false);
        let handle = world.add_body(body);

        world.step(0.016);

        let body = world.get_body(handle).unwrap();
        // Should be pushed up so the bottom of the sphere is at y=0
        // Sphere center should be at y=0.5 (radius)
        assert!(body.position.y >= 0.5 - 0.001);
    }

    #[test]
    fn test_floor_collision_with_downward_velocity() {
        // Use a floor material with zero restitution
        let mut world = world_with_floor(0.0, 0.0, PhysicsMaterial::new(0.5, 0.0));
        // Sphere above floor with downward velocity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.6, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(0.0, -10.0, 0.0, 0.0))
            .with_gravity(false);
        let handle = world.add_body(body);

        // Step enough to hit the floor
        world.step(0.1);

        let body = world.get_body(handle).unwrap();
        // Velocity should be zeroed (no bounce, restitution = 0)
        assert!(body.velocity.y.abs() < 0.001);
    }

    #[test]
    fn test_floor_collision_with_bounce() {
        // Perfect bounce floor (restitution = 1.0)
        let mut world = world_with_floor(0.0, 0.0, PhysicsMaterial::new(0.5, 1.0));

        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.6, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(0.0, -10.0, 0.0, 0.0));
        let handle = world.add_body(body);

        world.step(0.1);

        let body = world.get_body(handle).unwrap();
        // With perfect restitution, velocity should flip
        assert!(body.velocity.y > 0.0);
    }

    #[test]
    fn test_body_body_collision_sphere_vs_static_aabb() {
        // No floor (no static colliders)
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

        // Static AABB
        let aabb = RigidBody4D::new_static_aabb(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));
        world.add_body(aabb);

        // Sphere moving toward the AABB
        let sphere = RigidBody4D::new_sphere(Vec4::new(2.0, 0.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(-10.0, 0.0, 0.0, 0.0));
        let sphere_handle = world.add_body(sphere);

        // Step until collision
        for _ in 0..10 {
            world.step(0.016);
        }

        let sphere = world.get_body(sphere_handle).unwrap();
        // Sphere should have stopped (or bounced back) and not penetrate the AABB
        // The AABB extends from -1 to 1 on x-axis, sphere should stop at x >= 1.5
        assert!(sphere.position.x >= 1.5 - 0.1);
    }

    #[test]
    fn test_body_body_collision_two_spheres() {
        // No floor (no static colliders)
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

        // First sphere stationary
        let sphere1 = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5);
        let handle1 = world.add_body(sphere1);

        // Second sphere moving toward first
        let sphere2 = RigidBody4D::new_sphere(Vec4::new(2.0, 0.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(-10.0, 0.0, 0.0, 0.0));
        let handle2 = world.add_body(sphere2);

        // Step until collision
        for _ in 0..20 {
            world.step(0.016);
        }

        let sphere1 = world.get_body(handle1).unwrap();
        let sphere2 = world.get_body(handle2).unwrap();

        // Spheres should not penetrate each other
        let distance = (sphere2.position - sphere1.position).length();
        assert!(distance >= 1.0 - 0.1); // Combined radii = 1.0
    }

    #[test]
    fn test_collider_stays_synced_with_position() {
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(5.0, 0.0, 0.0, 0.0));
        let handle = world.add_body(body);

        world.step(1.0);

        let body = world.get_body(handle).unwrap();
        // Collider center should match position
        assert_eq!(body.collider.center(), body.position);
    }

    #[test]
    fn test_gravity_disabled_body() {
        let mut world = PhysicsWorld::new();
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
            .with_gravity(false);
        let handle = world.add_body(body);

        world.step(1.0);

        let body = world.get_body(handle).unwrap();
        // Body should not have fallen (no gravity)
        assert_eq!(body.position.y, 10.0);
        assert_eq!(body.velocity.y, 0.0);
    }

    #[test]
    fn test_friction_slows_horizontal_movement() {
        // High friction floor (rubber)
        let mut world = world_with_floor(-20.0, 0.0, PhysicsMaterial::RUBBER);

        // Sphere sliding on floor with horizontal velocity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.5, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(10.0, -1.0, 0.0, 0.0)) // Moving right, slightly into floor
            .with_gravity(false);
        let handle = world.add_body(body);

        world.step(0.016);

        let body = world.get_body(handle).unwrap();
        // Horizontal velocity should be reduced by friction
        // Rubber has friction 0.9, so velocity should be significantly reduced
        assert!(body.velocity.x < 10.0, "Friction should slow horizontal movement");
        assert!(body.velocity.x < 5.0, "High friction should reduce velocity significantly");
    }

    #[test]
    fn test_ice_floor_low_friction() {
        // Ice floor (very low friction)
        let mut world = world_with_floor(-20.0, 0.0, PhysicsMaterial::ICE);

        // Sphere sliding on floor with horizontal velocity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.5, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(10.0, -1.0, 0.0, 0.0))
            .with_gravity(false);
        let handle = world.add_body(body);

        world.step(0.016);

        let body = world.get_body(handle).unwrap();
        // Ice has friction 0.05, so velocity should barely change
        // Combined friction = sqrt(0.5 * 0.05) = sqrt(0.025) ≈ 0.158
        // friction_factor = 1 - 0.158 ≈ 0.842, so velocity ≈ 10 * 0.842 = 8.42
        assert!(body.velocity.x > 8.0, "Ice should have minimal friction");
    }

    #[test]
    fn test_static_colliders() {
        let mut world = PhysicsWorld::new();
        assert_eq!(world.static_colliders().len(), 0);

        world.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));
        assert_eq!(world.static_colliders().len(), 1);

        // Add a wall
        world.add_static_collider(StaticCollider::plane(
            Vec4::new(1.0, 0.0, 0.0, 0.0),  // Normal pointing +X
            0.0,
            PhysicsMaterial::METAL,
        ));
        assert_eq!(world.static_colliders().len(), 2);
    }

    #[test]
    fn test_multiple_static_colliders() {
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(-10.0));

        // Floor at Y = 0
        world.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));

        // Ceiling at Y = 10 (normal pointing down)
        world.add_static_collider(StaticCollider::plane(
            Vec4::new(0.0, -1.0, 0.0, 0.0),
            -10.0,
            PhysicsMaterial::METAL,
        ));

        // Ball in the middle
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
        world.add_body(body);

        // Step simulation - ball should bounce between floor and ceiling
        for _ in 0..1000 {
            world.step(0.016);
        }

        // Ball should still be between 0 and 10
        let ball = world.bodies.values().next().unwrap();
        assert!(ball.position.y >= 0.0 && ball.position.y <= 10.0,
            "Ball should be between floor and ceiling, got y={}", ball.position.y);
    }
}
