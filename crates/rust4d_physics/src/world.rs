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

/// Default jump velocity for player
pub const DEFAULT_JUMP_VELOCITY: f32 = 8.0;

/// The physics world containing all rigid bodies
pub struct PhysicsWorld {
    /// All rigid bodies in the world (using generational keys)
    bodies: SlotMap<BodyKey, RigidBody4D>,
    /// Static colliders (floors, walls, platforms)
    static_colliders: Vec<StaticCollider>,
    /// Physics configuration
    pub config: PhysicsConfig,
    /// The player body key (if a player has been registered)
    player_body: Option<BodyKey>,
    /// Jump velocity for the player
    player_jump_velocity: f32,
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
            player_body: None,
            player_jump_velocity: DEFAULT_JUMP_VELOCITY,
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

    // ====== Player Body Management ======

    /// Register a body as the player body
    ///
    /// This enables player-specific features like jump and grounded detection.
    /// The body should typically be kinematic (no gravity, user-controlled velocity).
    pub fn set_player_body(&mut self, key: BodyKey) {
        self.player_body = Some(key);
    }

    /// Set the jump velocity for the player
    pub fn set_player_jump_velocity(&mut self, velocity: f32) {
        self.player_jump_velocity = velocity;
    }

    /// Get the player body key
    pub fn player_key(&self) -> Option<BodyKey> {
        self.player_body
    }

    /// Get an immutable reference to the player body
    pub fn player(&self) -> Option<&RigidBody4D> {
        self.player_body.and_then(|key| self.bodies.get(key))
    }

    /// Get a mutable reference to the player body
    pub fn player_mut(&mut self) -> Option<&mut RigidBody4D> {
        self.player_body.and_then(|key| self.bodies.get_mut(key))
    }

    /// Get the player's current position
    pub fn player_position(&self) -> Option<Vec4> {
        self.player().map(|body| body.position)
    }

    /// Check if the player is currently grounded
    pub fn player_is_grounded(&self) -> bool {
        self.player().map(|body| body.grounded).unwrap_or(false)
    }

    /// Apply horizontal movement to the player (XZ plane + W for 4D)
    ///
    /// This sets the player's velocity on the XZ and W axes.
    /// The Y component is controlled by gravity and jumping.
    pub fn apply_player_movement(&mut self, movement: Vec4) {
        if let Some(body) = self.player_mut() {
            // Only set horizontal velocity - preserve Y for gravity/jumping
            body.velocity.x = movement.x;
            body.velocity.z = movement.z;
            body.velocity.w = movement.w;
        }
    }

    /// Attempt to make the player jump
    ///
    /// Only succeeds if the player is grounded. Sets vertical velocity
    /// to the configured jump velocity.
    pub fn player_jump(&mut self) -> bool {
        let jump_vel = self.player_jump_velocity;
        if let Some(body) = self.player_mut() {
            if body.grounded {
                body.velocity.y = jump_vel;
                body.grounded = false;
                return true;
            }
        }
        false
    }

    /// Step the physics simulation forward by dt seconds
    ///
    /// This performs:
    /// 1. Gravity application to non-static bodies with gravity enabled
    /// 2. Velocity integration into position
    /// 3. Static collider collision detection and resolution
    /// 4. Body-body collision detection and resolution
    pub fn step(&mut self, dt: f32) {
        // Reset grounded state for player before collision detection
        if let Some(key) = self.player_body {
            if let Some(body) = self.bodies.get_mut(key) {
                body.grounded = false;
            }
        }

        // Phase 1: Apply gravity and integrate velocity
        for (key, body) in &mut self.bodies {
            if body.is_static() {
                continue;
            }

            // Apply gravity to:
            // - Dynamic bodies (normal physics objects)
            // - The player body (kinematic but needs gravity for jumping/falling)
            let is_player = self.player_body == Some(key);
            if body.affected_by_gravity() || is_player {
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
        // Threshold for considering a surface as "ground" (normal pointing mostly up)
        const GROUND_NORMAL_THRESHOLD: f32 = 0.7;

        for (_key, body) in &mut self.bodies {
            if body.is_static() {
                continue;
            }

            for static_col in &self.static_colliders {
                // Check if collision layers allow this interaction
                if !body.filter.collides_with(&static_col.filter) {
                    continue;
                }

                let contact = Self::check_static_collision(&body.collider, &static_col.collider);

                if let Some(contact) = contact {
                    if contact.is_colliding() {
                        // Push the body out of the static collider
                        let correction = contact.normal * contact.penetration;
                        body.apply_correction(correction);

                        // Check if this is a ground contact (normal pointing up)
                        // This is used for grounded state detection
                        if contact.normal.y > GROUND_NORMAL_THRESHOLD {
                            body.grounded = true;
                        }

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

                // Get colliders and filters for both bodies
                let (collider_a, collider_b, is_static_a, is_static_b, filter_a, filter_b) = {
                    let body_a = &self.bodies[key_a];
                    let body_b = &self.bodies[key_b];
                    (body_a.collider, body_b.collider, body_a.is_static(), body_b.is_static(), body_a.filter, body_b.filter)
                };

                // Skip if both bodies are static
                if is_static_a && is_static_b {
                    continue;
                }

                // Check if collision layers allow this interaction
                if !filter_a.collides_with(&filter_b) {
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
        let is_kinematic_a = self.bodies[key_a].is_kinematic();
        let is_kinematic_b = self.bodies[key_b].is_kinematic();

        // Position correction rules:
        // - Static bodies never move
        // - Kinematic bodies: pushed by static geometry, NOT pushed by dynamic bodies
        // - Dynamic bodies: always pushed
        //
        // can_correct = not static AND (not kinematic OR other is static)
        let can_correct_a = !is_static_a && (!is_kinematic_a || is_static_b);
        let can_correct_b = !is_static_b && (!is_kinematic_b || is_static_a);

        // Determine how to split the correction
        let (correction_a, correction_b) = if !can_correct_a && can_correct_b {
            // Only B moves
            (Vec4::ZERO, contact.normal * contact.penetration)
        } else if can_correct_a && !can_correct_b {
            // Only A moves
            (-contact.normal * contact.penetration, Vec4::ZERO)
        } else if !can_correct_a && !can_correct_b {
            // Neither can move (both static, shouldn't happen)
            (Vec4::ZERO, Vec4::ZERO)
        } else {
            // Both can move - split based on mass
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
        if can_correct_a {
            self.bodies[key_a].apply_correction(correction_a);
        }
        if can_correct_b {
            self.bodies[key_b].apply_correction(correction_b);
        }

        // Combine materials from both bodies
        let combined = self.bodies[key_a].material.combine(&self.bodies[key_b].material);

        // Velocity response rules:
        // - Static bodies: no velocity (implicit)
        // - Kinematic bodies: velocity is user-controlled, never modified by collisions
        // - Dynamic bodies: velocity response applied
        let can_modify_velocity_a = !is_static_a && !is_kinematic_a;
        let can_modify_velocity_b = !is_static_b && !is_kinematic_b;

        // Handle velocity response with restitution
        if can_modify_velocity_a {
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

        if can_modify_velocity_b {
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

    // ====== Player Body Tests ======

    #[test]
    fn test_player_body_registration() {
        let mut world = PhysicsWorld::new();

        // Create player body (kinematic - no gravity)
        let player = RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
            .with_body_type(crate::body::BodyType::Kinematic);
        let player_key = world.add_body(player);

        // Register as player
        world.set_player_body(player_key);

        // Check player body access
        assert_eq!(world.player_key(), Some(player_key));
        assert!(world.player().is_some());
        assert!(world.player_mut().is_some());
    }

    #[test]
    fn test_player_position() {
        let mut world = PhysicsWorld::new();

        let start_pos = Vec4::new(5.0, 2.0, 3.0, 1.0);
        let player = RigidBody4D::new_sphere(start_pos, 0.5)
            .with_body_type(crate::body::BodyType::Kinematic);
        let player_key = world.add_body(player);
        world.set_player_body(player_key);

        assert_eq!(world.player_position(), Some(start_pos));
    }

    #[test]
    fn test_player_movement() {
        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity

        let player = RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
            .with_body_type(crate::body::BodyType::Kinematic);
        let player_key = world.add_body(player);
        world.set_player_body(player_key);

        // Apply horizontal movement
        world.apply_player_movement(Vec4::new(10.0, 0.0, 5.0, 2.0));

        // Step physics
        world.step(0.1);

        // Check player moved in XZW but Y was preserved
        let pos = world.player_position().unwrap();
        assert!((pos.x - 1.0).abs() < 0.01); // 10 * 0.1 = 1.0
        assert!((pos.y - 1.0).abs() < 0.01); // Y unchanged
        assert!((pos.z - 0.5).abs() < 0.01); // 5 * 0.1 = 0.5
        assert!((pos.w - 0.2).abs() < 0.01); // 2 * 0.1 = 0.2
    }

    #[test]
    fn test_player_grounded_detection() {
        let mut world = world_with_floor(0.0, 0.0, PhysicsMaterial::CONCRETE);

        // Player just above floor (radius 0.5, position at y=0.5 means touching floor)
        let player = RigidBody4D::new_sphere(Vec4::new(0.0, 0.4, 0.0, 0.0), 0.5)
            .with_body_type(crate::body::BodyType::Kinematic);
        let player_key = world.add_body(player);
        world.set_player_body(player_key);

        // Initially not grounded
        assert!(!world.player_is_grounded());

        // Step to detect floor collision
        world.step(0.016);

        // Should be grounded after collision detection
        assert!(world.player_is_grounded());
    }

    #[test]
    fn test_player_jump() {
        let mut world = world_with_floor(0.0, 0.0, PhysicsMaterial::CONCRETE);

        // Player on floor
        let player = RigidBody4D::new_sphere(Vec4::new(0.0, 0.4, 0.0, 0.0), 0.5)
            .with_body_type(crate::body::BodyType::Kinematic);
        let player_key = world.add_body(player);
        world.set_player_body(player_key);

        // Step to get grounded
        world.step(0.016);
        assert!(world.player_is_grounded());

        // Jump
        let jumped = world.player_jump();
        assert!(jumped);
        assert!(!world.player_is_grounded());

        // Check velocity set
        let vel = world.player().unwrap().velocity;
        assert_eq!(vel.y, DEFAULT_JUMP_VELOCITY);
    }

    #[test]
    fn test_player_cannot_jump_while_airborne() {
        let mut world = PhysicsWorld::new();

        // Player in the air
        let player = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
            .with_body_type(crate::body::BodyType::Kinematic);
        let player_key = world.add_body(player);
        world.set_player_body(player_key);

        // Not grounded initially
        assert!(!world.player_is_grounded());

        // Jump should fail
        let jumped = world.player_jump();
        assert!(!jumped);

        // Velocity should still be zero
        let vel = world.player().unwrap().velocity;
        assert_eq!(vel.y, 0.0);
    }

    #[test]
    fn test_player_jump_velocity_config() {
        let mut world = PhysicsWorld::new();
        world.set_player_jump_velocity(15.0);

        // Player that's grounded
        let mut player = RigidBody4D::new_sphere(Vec4::new(0.0, 0.5, 0.0, 0.0), 0.5)
            .with_body_type(crate::body::BodyType::Kinematic);
        player.grounded = true; // Manually set grounded for test
        let player_key = world.add_body(player);
        world.set_player_body(player_key);

        // Jump
        world.player_jump();

        // Check custom velocity used
        let vel = world.player().unwrap().velocity;
        assert_eq!(vel.y, 15.0);
    }

    // ====== Collision Filtering Tests ======

    #[test]
    fn test_collision_filter_static_collider_skip() {
        use crate::collision::{CollisionFilter, CollisionLayer};

        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

        // Create a trigger zone that only detects players
        // but players don't collide with triggers
        let trigger = StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE)
            .with_filter(CollisionFilter::trigger(CollisionLayer::PLAYER));
        world.add_static_collider(trigger);

        // A sphere with default filter (DEFAULT layer) - should pass through trigger
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.5, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(0.0, -10.0, 0.0, 0.0));
        let handle = world.add_body(body);

        // Step physics - body should fall through trigger (no collision)
        world.step(0.1);

        let body = world.get_body(handle).unwrap();
        // Body should have moved down (no floor collision)
        assert!(body.position.y < 0.5, "Body should fall through trigger zone");
    }

    #[test]
    fn test_collision_filter_body_body_skip() {
        use crate::collision::CollisionFilter;

        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

        // Two players - players don't collide with each other
        let player1 = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::player());
        let handle1 = world.add_body(player1);

        let player2 = RigidBody4D::new_sphere(Vec4::new(0.8, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::player());
        let _handle2 = world.add_body(player2);

        // They overlap (centers 0.8 apart, combined radii 1.0) but shouldn't collide
        world.step(0.016);

        // Player1's position should be unchanged (no push)
        let p1 = world.get_body(handle1).unwrap();
        assert_eq!(p1.position.x, 0.0, "Players should not push each other");
    }

    #[test]
    fn test_collision_filter_body_body_collide() {
        use crate::collision::CollisionFilter;

        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

        // Player vs enemy - they should collide
        let player = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::player());
        let handle_player = world.add_body(player);

        let enemy = RigidBody4D::new_sphere(Vec4::new(0.8, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::enemy());
        world.add_body(enemy);

        // They overlap and should collide
        world.step(0.016);

        // Player's position should change (pushed)
        let p = world.get_body(handle_player).unwrap();
        assert!(p.position.x < 0.0, "Player should be pushed by enemy");
    }

    #[test]
    fn test_player_projectile_filter() {
        use crate::collision::CollisionFilter;

        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0));

        // Player
        let player = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_filter(CollisionFilter::player());
        let handle_player = world.add_body(player);

        // Player's projectile moving toward player - should not hit
        let projectile = RigidBody4D::new_sphere(Vec4::new(1.5, 0.0, 0.0, 0.0), 0.3)
            .with_filter(CollisionFilter::player_projectile())
            .with_velocity(Vec4::new(-20.0, 0.0, 0.0, 0.0));
        world.add_body(projectile);

        // Step several times
        for _ in 0..10 {
            world.step(0.016);
        }

        // Player should not have moved (projectile passed through)
        let p = world.get_body(handle_player).unwrap();
        assert_eq!(p.position.x, 0.0, "Player projectile should not hit player");
    }

    // ====== Kinematic-Dynamic Collision Tests ======

    #[test]
    fn test_kinematic_pushes_dynamic() {
        // Kinematic body colliding with dynamic should push the dynamic body only
        use crate::body::BodyType;

        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity

        // Kinematic body (player-like) moving right
        let kinematic = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_body_type(BodyType::Kinematic)
            .with_velocity(Vec4::new(5.0, 0.0, 0.0, 0.0));
        let key_kinematic = world.add_body(kinematic);

        // Dynamic body (pushable object) slightly to the right
        let dynamic = RigidBody4D::new_sphere(Vec4::new(1.0, 0.0, 0.0, 0.0), 0.5)
            .with_body_type(BodyType::Dynamic);
        let key_dynamic = world.add_body(dynamic);

        let initial_kinematic_x = 0.0;
        let initial_dynamic_x = 1.0;

        // Step physics multiple times to let collision occur
        for _ in 0..10 {
            world.step(0.016);
        }

        let kinematic_body = world.get_body(key_kinematic).unwrap();
        let dynamic_body = world.get_body(key_dynamic).unwrap();

        // Kinematic should have moved (velocity-driven)
        assert!(
            kinematic_body.position.x > initial_kinematic_x,
            "Kinematic should move based on its velocity"
        );

        // Dynamic should have been pushed (moved more than just overlap resolution)
        assert!(
            dynamic_body.position.x > initial_dynamic_x,
            "Dynamic body should be pushed by kinematic"
        );
    }

    #[test]
    fn test_kinematic_not_pushed_by_dynamic() {
        // Dynamic body colliding with kinematic should not move the kinematic
        use crate::body::BodyType;

        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity

        // Kinematic body (player-like) stationary
        let kinematic = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_body_type(BodyType::Kinematic);
        let key_kinematic = world.add_body(kinematic);

        // Dynamic body moving toward kinematic
        let dynamic = RigidBody4D::new_sphere(Vec4::new(2.0, 0.0, 0.0, 0.0), 0.5)
            .with_body_type(BodyType::Dynamic)
            .with_velocity(Vec4::new(-10.0, 0.0, 0.0, 0.0));
        let key_dynamic = world.add_body(dynamic);

        let initial_kinematic_pos = Vec4::new(0.0, 0.0, 0.0, 0.0);

        // Step physics multiple times
        for _ in 0..10 {
            world.step(0.016);
        }

        let kinematic_body = world.get_body(key_kinematic).unwrap();
        let dynamic_body = world.get_body(key_dynamic).unwrap();

        // Kinematic should NOT have moved
        assert!(
            (kinematic_body.position - initial_kinematic_pos).length() < 0.001,
            "Kinematic body should not be pushed by dynamic body"
        );

        // Dynamic should have bounced back or stopped (not passed through)
        assert!(
            dynamic_body.position.x >= kinematic_body.position.x + 0.9, // At least radius distance away
            "Dynamic body should be separated from kinematic"
        );
    }

    #[test]
    fn test_kinematic_velocity_not_modified() {
        // Kinematic body velocity should be unchanged after collision with dynamic
        use crate::body::BodyType;

        let mut world = PhysicsWorld::with_config(PhysicsConfig::new(0.0)); // No gravity

        let initial_velocity = Vec4::new(3.0, 0.0, 0.0, 0.0);

        // Kinematic body moving right
        let kinematic = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_body_type(BodyType::Kinematic)
            .with_velocity(initial_velocity);
        let key_kinematic = world.add_body(kinematic);

        // Dynamic body in the way
        let dynamic = RigidBody4D::new_sphere(Vec4::new(0.8, 0.0, 0.0, 0.0), 0.5)
            .with_body_type(BodyType::Dynamic);
        world.add_body(dynamic);

        // Step physics - collision should occur
        for _ in 0..5 {
            world.step(0.016);
        }

        let kinematic_body = world.get_body(key_kinematic).unwrap();

        // Kinematic velocity should be unchanged (user-controlled)
        assert!(
            (kinematic_body.velocity - initial_velocity).length() < 0.001,
            "Kinematic velocity should not be modified by collision. Expected {:?}, got {:?}",
            initial_velocity,
            kinematic_body.velocity
        );
    }
}
