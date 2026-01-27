//! Physics world and simulation

use crate::body::{BodyHandle, RigidBody4D};
use crate::collision::{aabb_vs_aabb, aabb_vs_plane, sphere_vs_aabb, sphere_vs_plane};
use crate::shapes::{Collider, Plane4D};
use rust4d_math::Vec4;

/// Configuration for the physics simulation
#[derive(Clone, Debug)]
pub struct PhysicsConfig {
    /// Gravity acceleration (applied to Y-axis, negative = down)
    pub gravity: f32,
    /// Y position of the floor plane
    pub floor_y: f32,
    /// Default restitution (bounciness) for collisions
    pub restitution: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: -20.0,
            floor_y: 0.0,
            restitution: 0.0,
        }
    }
}

impl PhysicsConfig {
    /// Create a new physics config with custom values
    pub fn new(gravity: f32, floor_y: f32, restitution: f32) -> Self {
        Self {
            gravity,
            floor_y,
            restitution,
        }
    }
}

/// The physics world containing all rigid bodies
pub struct PhysicsWorld {
    /// All rigid bodies in the world
    bodies: Vec<RigidBody4D>,
    /// The floor plane
    floor: Plane4D,
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
            bodies: Vec::new(),
            floor: Plane4D::floor(config.floor_y),
            config,
        }
    }

    /// Add a body to the world and return its handle
    pub fn add_body(&mut self, body: RigidBody4D) -> BodyHandle {
        let handle = BodyHandle(self.bodies.len());
        self.bodies.push(body);
        handle
    }

    /// Get an immutable reference to a body by handle
    pub fn get_body(&self, handle: BodyHandle) -> Option<&RigidBody4D> {
        self.bodies.get(handle.0)
    }

    /// Get a mutable reference to a body by handle
    pub fn get_body_mut(&mut self, handle: BodyHandle) -> Option<&mut RigidBody4D> {
        self.bodies.get_mut(handle.0)
    }

    /// Get the number of bodies in the world
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    /// Step the physics simulation forward by dt seconds
    ///
    /// This performs:
    /// 1. Gravity application to non-static bodies with gravity enabled
    /// 2. Velocity integration into position
    /// 3. Floor collision detection and resolution
    /// 4. Body-body collision detection and resolution
    pub fn step(&mut self, dt: f32) {
        // Phase 1: Apply gravity and integrate velocity
        for body in &mut self.bodies {
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

        // Phase 2: Resolve floor collisions
        self.resolve_floor_collisions();

        // Phase 3: Resolve body-body collisions
        self.resolve_body_collisions();
    }

    /// Resolve collisions between bodies and the floor
    fn resolve_floor_collisions(&mut self) {
        for body in &mut self.bodies {
            if body.is_static {
                continue;
            }

            let contact = match &body.collider {
                Collider::Sphere(sphere) => sphere_vs_plane(sphere, &self.floor),
                Collider::AABB(aabb) => aabb_vs_plane(aabb, &self.floor),
            };

            if let Some(contact) = contact {
                if contact.is_colliding() {
                    // Push the body out of the floor
                    let correction = contact.normal * contact.penetration;
                    body.apply_correction(correction);

                    // Handle velocity response
                    let velocity_along_normal = body.velocity.dot(contact.normal);
                    if velocity_along_normal < 0.0 {
                        // Body is moving into the floor
                        let restitution = body.restitution.max(self.config.restitution);

                        // Remove the normal component of velocity and optionally bounce
                        let normal_velocity = contact.normal * velocity_along_normal;
                        body.velocity = body.velocity - normal_velocity * (1.0 + restitution);
                    }
                }
            }
        }
    }

    /// Resolve collisions between bodies
    fn resolve_body_collisions(&mut self) {
        let body_count = self.bodies.len();

        // Check all pairs of bodies
        for i in 0..body_count {
            for j in (i + 1)..body_count {
                // Get colliders for both bodies
                let (collider_a, collider_b, is_static_a, is_static_b) = {
                    let body_a = &self.bodies[i];
                    let body_b = &self.bodies[j];
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
                        // Sphere vs Sphere
                        let delta = b.center - a.center;
                        let dist_sq = delta.length_squared();
                        let min_dist = a.radius + b.radius;

                        if dist_sq < min_dist * min_dist && dist_sq > 0.0001 {
                            let dist = dist_sq.sqrt();
                            let penetration = min_dist - dist;
                            // Normal points from A toward B
                            let normal = delta.normalized();
                            let point = a.center + normal * a.radius;
                            Some(crate::collision::Contact::new(point, normal, penetration))
                        } else {
                            None
                        }
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
                };

                if let Some(contact) = contact {
                    if contact.is_colliding() {
                        self.resolve_body_pair_collision(i, j, &contact, is_static_a, is_static_b);
                    }
                }
            }
        }
    }

    /// Resolve collision between two specific bodies
    fn resolve_body_pair_collision(
        &mut self,
        i: usize,
        j: usize,
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
            let mass_a = self.bodies[i].mass;
            let mass_b = self.bodies[j].mass;
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
            self.bodies[i].apply_correction(correction_a);
        }
        if !is_static_b {
            self.bodies[j].apply_correction(correction_b);
        }

        // Handle velocity response (simple push-apart)
        if !is_static_a {
            let vel_along_normal = self.bodies[i].velocity.dot(-contact.normal);
            if vel_along_normal < 0.0 {
                let restitution = self.bodies[i].restitution.max(self.config.restitution);
                let normal_velocity = -contact.normal * vel_along_normal;
                self.bodies[i].velocity = self.bodies[i].velocity - normal_velocity * (1.0 + restitution);
            }
        }

        if !is_static_b {
            let vel_along_normal = self.bodies[j].velocity.dot(contact.normal);
            if vel_along_normal < 0.0 {
                let restitution = self.bodies[j].restitution.max(self.config.restitution);
                let normal_velocity = contact.normal * vel_along_normal;
                self.bodies[j].velocity = self.bodies[j].velocity - normal_velocity * (1.0 + restitution);
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

    #[test]
    fn test_physics_config_default() {
        let config = PhysicsConfig::default();
        assert_eq!(config.gravity, -20.0);
        assert_eq!(config.floor_y, 0.0);
        assert_eq!(config.restitution, 0.0);
    }

    #[test]
    fn test_physics_config_custom() {
        let config = PhysicsConfig::new(-10.0, 5.0, 0.5);
        assert_eq!(config.gravity, -10.0);
        assert_eq!(config.floor_y, 5.0);
        assert_eq!(config.restitution, 0.5);
    }

    #[test]
    fn test_world_add_body() {
        let mut world = PhysicsWorld::new();
        assert_eq!(world.body_count(), 0);

        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5);
        let handle = world.add_body(body);

        assert_eq!(handle.index(), 0);
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
    fn test_world_invalid_handle() {
        let world = PhysicsWorld::new();
        let invalid_handle = BodyHandle(999);
        assert!(world.get_body(invalid_handle).is_none());
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
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0, 0.0, 0.0)); // No gravity
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
        let mut world = PhysicsWorld::new();
        // Sphere starting below the floor
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
        let mut world = PhysicsWorld::new();
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
        let config = PhysicsConfig::new(0.0, 0.0, 1.0); // Perfect bounce
        let mut world = PhysicsWorld::with_config(config);

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
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0, -100.0, 0.0)); // No floor in the way

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
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0, -100.0, 0.0)); // No floor in the way

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
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0, 0.0, 0.0));

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
}
