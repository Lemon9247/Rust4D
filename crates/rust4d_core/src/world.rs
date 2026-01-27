//! World container for entities
//!
//! The World manages all entities in the simulation.

use crate::Entity;
use rust4d_physics::{PhysicsConfig, PhysicsWorld};
use slotmap::{new_key_type, SlotMap};

// Define generational key type for entities
new_key_type! {
    /// Key to an entity in the world
    ///
    /// Uses generational indexing to prevent the ABA problem where a key
    /// could point to a reused slot. If an entity is removed and its slot reused,
    /// old keys will return None instead of pointing to the wrong entity.
    pub struct EntityKey;
}

/// The 4D world containing all entities
///
/// The World is the central container for all game objects.
/// It manages entities and integrates with physics simulation.
pub struct World {
    /// All entities in the world (using generational keys)
    entities: SlotMap<EntityKey, Entity>,
    /// Optional physics simulation (None = no physics)
    physics_world: Option<PhysicsWorld>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Create a new empty world
    pub fn new() -> Self {
        Self {
            entities: SlotMap::with_key(),
            physics_world: None,
        }
    }

    /// Create a world with pre-allocated capacity for entities
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: SlotMap::with_capacity_and_key(capacity),
            physics_world: None,
        }
    }

    /// Enable physics for this world
    pub fn with_physics(mut self, config: PhysicsConfig) -> Self {
        self.physics_world = Some(PhysicsWorld::with_config(config));
        self
    }

    /// Get the physics world (if enabled)
    pub fn physics(&self) -> Option<&PhysicsWorld> {
        self.physics_world.as_ref()
    }

    /// Get mutable physics world (if enabled)
    pub fn physics_mut(&mut self) -> Option<&mut PhysicsWorld> {
        self.physics_world.as_mut()
    }

    /// Add an entity to the world, returning its key
    pub fn add_entity(&mut self, entity: Entity) -> EntityKey {
        self.entities.insert(entity)
    }

    /// Remove an entity from the world and return it
    pub fn remove_entity(&mut self, key: EntityKey) -> Option<Entity> {
        self.entities.remove(key)
    }

    /// Get a reference to an entity by key
    pub fn get_entity(&self, key: EntityKey) -> Option<&Entity> {
        self.entities.get(key)
    }

    /// Get a mutable reference to an entity by key
    pub fn get_entity_mut(&mut self, key: EntityKey) -> Option<&mut Entity> {
        self.entities.get_mut(key)
    }

    /// Get the number of entities
    #[inline]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Check if the world is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Iterate over all entity keys
    pub fn entity_keys(&self) -> impl Iterator<Item = EntityKey> + '_ {
        self.entities.keys()
    }

    /// Update the world by stepping physics and syncing entity transforms
    ///
    /// This method:
    /// 1. Steps the physics simulation (if enabled)
    /// 2. Syncs entity transforms from their associated physics bodies
    pub fn update(&mut self, dt: f32) {
        // Step the physics simulation
        if let Some(ref mut physics) = self.physics_world {
            physics.step(dt);
        }

        // Sync entity transforms from their physics bodies
        if let Some(ref physics) = self.physics_world {
            for (_key, entity) in &mut self.entities {
                if let Some(body_key) = entity.physics_body {
                    if let Some(body) = physics.get_body(body_key) {
                        entity.transform.position = body.position;
                    }
                }
            }
        }
    }

    /// Clear all entities from the world
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    /// Iterate over all entities
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    /// Iterate over all entities mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.values_mut()
    }

    /// Iterate over keys and entities
    pub fn iter_with_keys(&self) -> impl Iterator<Item = (EntityKey, &Entity)> {
        self.entities.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Material, ShapeRef};
    use rust4d_math::Tesseract4D;

    fn make_test_entity() -> Entity {
        let tesseract = Tesseract4D::new(2.0);
        Entity::new(ShapeRef::shared(tesseract))
    }

    #[test]
    fn test_world_new() {
        let world = World::new();
        assert!(world.is_empty());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_world_add_entity() {
        let mut world = World::new();
        let entity = make_test_entity();
        let key = world.add_entity(entity);

        // Key should be valid
        assert!(world.get_entity(key).is_some());
        assert_eq!(world.entity_count(), 1);
    }

    #[test]
    fn test_world_get_entity() {
        let mut world = World::new();
        let entity = make_test_entity();
        let handle = world.add_entity(entity);

        let retrieved = world.get_entity(handle);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().shape().vertex_count(), 16);
    }

    #[test]
    fn test_world_get_entity_mut() {
        let mut world = World::new();
        let entity = make_test_entity();
        let handle = world.add_entity(entity);

        if let Some(entity) = world.get_entity_mut(handle) {
            entity.material = Material::RED;
        }

        let retrieved = world.get_entity(handle).unwrap();
        assert_eq!(retrieved.material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_world_entity_count() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn test_world_clear() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        world.clear();
        assert!(world.is_empty());
    }

    #[test]
    fn test_world_iter() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        let count = world.iter().count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_world_iter_with_keys() {
        let mut world = World::new();
        let key1 = world.add_entity(make_test_entity());
        let key2 = world.add_entity(make_test_entity());

        let keys: Vec<_> = world.iter_with_keys().map(|(k, _)| k).collect();
        assert_eq!(keys.len(), 2);
        // Keys should contain both added keys (order may vary with SlotMap)
        assert!(keys.contains(&key1));
        assert!(keys.contains(&key2));
    }

    #[test]
    fn test_world_update() {
        let mut world = World::new();
        world.add_entity(make_test_entity());

        // Just verify it doesn't panic for now
        world.update(0.016);
    }

    #[test]
    fn test_world_default() {
        let world = World::default();
        assert!(world.is_empty());
    }

    #[test]
    fn test_world_with_capacity() {
        let world = World::with_capacity(100);
        assert!(world.is_empty());
        // Can't directly test capacity, but it shouldn't affect behavior
    }

    #[test]
    fn test_stale_key_returns_none() {
        let mut world = World::new();
        let entity = make_test_entity();
        let key = world.add_entity(entity);

        // Key is valid initially
        assert!(world.get_entity(key).is_some());

        // Remove the entity
        let removed = world.remove_entity(key);
        assert!(removed.is_some());

        // Key is now stale - should return None
        assert!(world.get_entity(key).is_none());

        // Add a new entity - it gets a different key
        let new_entity = make_test_entity();
        let new_key = world.add_entity(new_entity);

        // Old key still returns None (generational safety)
        assert!(world.get_entity(key).is_none());
        // New key works
        assert!(world.get_entity(new_key).is_some());
    }

    #[test]
    fn test_world_with_physics() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with physics enabled (no gravity for predictable test)
        let config = PhysicsConfig::new(0.0, -100.0, 0.0);
        let mut world = World::new().with_physics(config);

        assert!(world.physics().is_some());

        // Add a physics body with horizontal velocity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 5.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(10.0, 0.0, 0.0, 0.0));
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Verify initial position
        assert_eq!(world.get_entity(entity_handle).unwrap().transform.position.x, 0.0);

        // Step physics (1 second with 10 units/sec velocity = 10 units displacement)
        world.update(1.0);

        // Entity transform should now reflect the physics body position
        let entity = world.get_entity(entity_handle).unwrap();
        assert!((entity.transform.position.x - 10.0).abs() < 0.001);
        assert!((entity.transform.position.y - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_physics_sync_with_gravity() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with gravity (default config)
        let mut world = World::new().with_physics(PhysicsConfig::default());

        // Add a physics body that will fall
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5);
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Step physics
        world.update(0.1);

        // Entity should have fallen (gravity is -20, so after 0.1s, velocity = -2.0)
        // Position changes by: initial_vel * dt + 0.5 * g * dt^2 = 0 + 0.5 * (-20) * 0.01 = -0.1
        // But the actual integration is: v += g*dt, then p += v*dt
        // So v = -2.0, then p = 10.0 + (-2.0) * 0.1 = 10.0 - 0.2 = 9.8
        let entity = world.get_entity(entity_handle).unwrap();
        assert!(entity.transform.position.y < 10.0);
    }

    #[test]
    fn test_entity_without_physics_body() {
        // Create a world with physics
        let mut world = World::new().with_physics(PhysicsConfig::default());

        // Add an entity WITHOUT a physics body
        let mut entity = make_test_entity();
        entity.transform.position = rust4d_math::Vec4::new(5.0, 5.0, 5.0, 5.0);
        let entity_handle = world.add_entity(entity);

        // Step physics
        world.update(1.0);

        // Entity position should be unchanged (not linked to physics)
        let entity = world.get_entity(entity_handle).unwrap();
        assert_eq!(entity.transform.position.x, 5.0);
        assert_eq!(entity.transform.position.y, 5.0);
    }
}
