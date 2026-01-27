//! SceneBuilder - Declarative scene construction
//!
//! Provides a fluent API for building 4D scenes with physics.

use rust4d_core::{
    Entity, Material, ShapeRef, World,
    Hyperplane4D, PhysicsConfig, RigidBody4D, StaticCollider, Tesseract4D,
};
use rust4d_math::Vec4;
use rust4d_physics::{BodyType, PhysicsMaterial};

/// Builder for constructing 4D scenes with physics
///
/// # Example
/// ```ignore
/// let world = SceneBuilder::new()
///     .with_physics(-20.0)
///     .add_floor(-2.0, 10.0, PhysicsMaterial::CONCRETE)
///     .add_player(Vec4::new(0.0, 0.0, 5.0, 0.0), 0.5)
///     .add_tesseract(Vec4::ZERO, 2.0, "main_tesseract")
///     .build();
/// ```
pub struct SceneBuilder {
    world: World,
    player_start: Option<Vec4>,
}

impl SceneBuilder {
    /// Create a new scene builder
    pub fn new() -> Self {
        Self {
            world: World::new(),
            player_start: None,
        }
    }

    /// Create a scene builder with a pre-allocated world capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            world: World::with_capacity(capacity),
            player_start: None,
        }
    }

    /// Enable physics with the given gravity (negative = downward)
    pub fn with_physics(mut self, gravity: f32) -> Self {
        let config = PhysicsConfig::new(gravity);
        self.world = self.world.with_physics(config);
        self
    }

    /// Add a floor at the given Y position
    ///
    /// This adds both a physics floor collider and a visual floor entity.
    pub fn add_floor(mut self, y: f32, size: f32, material: PhysicsMaterial) -> Self {
        // Add physics floor collider
        if let Some(physics) = self.world.physics_mut() {
            physics.add_static_collider(StaticCollider::floor(y, material));
        }

        // Add visual floor entity
        // Using standard values: subdivisions=10, cell_size=size/2, thickness=0.001
        let floor_shape = Hyperplane4D::new(y, size, 10, size / 2.0, 0.001);
        self.world.add_entity(
            Entity::with_material(ShapeRef::shared(floor_shape), Material::GRAY)
                .with_name("floor")
                .with_tag("static"),
        );

        self
    }

    /// Add a wall plane with the given normal and distance from origin
    ///
    /// Only adds a physics collider (no visual - walls are typically invisible or handled separately).
    pub fn add_wall(mut self, normal: Vec4, distance: f32, material: PhysicsMaterial) -> Self {
        if let Some(physics) = self.world.physics_mut() {
            physics.add_static_collider(StaticCollider::plane(normal, distance, material));
        }
        self
    }

    /// Add a player at the given position with the given collision radius
    ///
    /// The player is a kinematic body (no gravity, user-controlled).
    pub fn add_player(mut self, position: Vec4, radius: f32) -> Self {
        self.player_start = Some(position);

        if let Some(physics) = self.world.physics_mut() {
            let player_body = RigidBody4D::new_sphere(position, radius)
                .with_body_type(BodyType::Kinematic)
                .with_mass(1.0)
                .with_material(PhysicsMaterial::WOOD);

            let body_key = physics.add_body(player_body);
            physics.set_player_body(body_key);
        }

        self
    }

    /// Add a tesseract (4D hypercube) at the given position
    ///
    /// The tesseract is a dynamic physics body with gravity enabled.
    pub fn add_tesseract(mut self, position: Vec4, size: f32, name: &str) -> Self {
        let half_extent = size / 2.0;

        // Add physics body
        let body_key = if let Some(physics) = self.world.physics_mut() {
            let body = RigidBody4D::new_aabb(position, Vec4::new(half_extent, half_extent, half_extent, half_extent))
                .with_body_type(BodyType::Dynamic)
                .with_mass(10.0)
                .with_material(PhysicsMaterial::WOOD);
            Some(physics.add_body(body))
        } else {
            None
        };

        // Add visual entity
        let tesseract = Tesseract4D::new(size);
        let mut entity = Entity::with_material(ShapeRef::shared(tesseract), Material::WHITE)
            .with_name(name)
            .with_tag("dynamic");

        if let Some(key) = body_key {
            entity = entity.with_physics_body(key);
        }

        self.world.add_entity(entity);

        self
    }

    /// Add a custom entity to the scene
    ///
    /// For entities that don't fit the standard patterns.
    pub fn add_entity(mut self, entity: Entity) -> Self {
        self.world.add_entity(entity);
        self
    }

    /// Build the scene and return the configured World
    pub fn build(self) -> World {
        self.world
    }

    /// Get the player's starting position (if a player was added)
    pub fn player_start(&self) -> Option<Vec4> {
        self.player_start
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_scene() {
        let world = SceneBuilder::new().build();
        assert_eq!(world.entity_count(), 0);
        assert!(world.physics().is_none());
    }

    #[test]
    fn test_scene_with_physics() {
        let world = SceneBuilder::new().with_physics(-20.0).build();

        assert!(world.physics().is_some());
        let physics = world.physics().unwrap();
        assert_eq!(physics.config.gravity, -20.0);
    }

    #[test]
    fn test_scene_with_floor() {
        let world = SceneBuilder::new()
            .with_physics(-10.0)
            .add_floor(0.0, 10.0, PhysicsMaterial::CONCRETE)
            .build();

        // Should have a floor entity
        assert_eq!(world.entity_count(), 1);

        // Should have a floor collider
        let physics = world.physics().unwrap();
        assert_eq!(physics.static_colliders().len(), 1);

        // Floor should be named and tagged
        let floor = world.get_by_name("floor");
        assert!(floor.is_some());
        assert!(floor.unwrap().1.has_tag("static"));
    }

    #[test]
    fn test_scene_with_player() {
        let world = SceneBuilder::new()
            .with_physics(-20.0)
            .add_player(Vec4::new(0.0, 1.0, 5.0, 0.0), 0.5)
            .build();

        let physics = world.physics().unwrap();
        assert!(physics.player_key().is_some());

        let player = physics.player().unwrap();
        assert_eq!(player.position, Vec4::new(0.0, 1.0, 5.0, 0.0));
        assert!(player.is_kinematic());
    }

    #[test]
    fn test_scene_with_tesseract() {
        let world = SceneBuilder::new()
            .with_physics(-20.0)
            .add_tesseract(Vec4::ZERO, 2.0, "test_tesseract")
            .build();

        // Should have a tesseract entity
        assert_eq!(world.entity_count(), 1);

        // Should be named and tagged
        let tesseract = world.get_by_name("test_tesseract");
        assert!(tesseract.is_some());
        assert!(tesseract.unwrap().1.has_tag("dynamic"));

        // Should have a physics body
        let physics = world.physics().unwrap();
        assert_eq!(physics.body_count(), 1);
    }

    #[test]
    fn test_full_scene() {
        let builder = SceneBuilder::with_capacity(3)
            .with_physics(-20.0)
            .add_floor(-2.0, 10.0, PhysicsMaterial::CONCRETE)
            .add_player(Vec4::new(0.0, 0.0, 5.0, 0.0), 0.5)
            .add_tesseract(Vec4::ZERO, 2.0, "main_tesseract");

        assert_eq!(builder.player_start(), Some(Vec4::new(0.0, 0.0, 5.0, 0.0)));

        let world = builder.build();

        // 2 entities: floor + tesseract (player is physics-only)
        assert_eq!(world.entity_count(), 2);

        // Physics: 1 static collider (floor) + 2 bodies (player + tesseract)
        let physics = world.physics().unwrap();
        assert_eq!(physics.static_colliders().len(), 1);
        assert_eq!(physics.body_count(), 2);
    }
}
