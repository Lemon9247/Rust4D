//! World container for entities
//!
//! The World manages all entities in the simulation.

use std::collections::HashMap;
use crate::{Entity, DirtyFlags};
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
    /// Index from entity names to keys (for fast name lookup)
    name_index: HashMap<String, EntityKey>,
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
            name_index: HashMap::new(),
            physics_world: None,
        }
    }

    /// Create a world with pre-allocated capacity for entities
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: SlotMap::with_capacity_and_key(capacity),
            name_index: HashMap::new(),
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
        // Get the name before moving the entity
        let name = entity.name.clone();
        let key = self.entities.insert(entity);

        // If the entity has a name, add it to the index
        if let Some(name) = name {
            self.name_index.insert(name, key);
        }

        key
    }

    /// Remove an entity from the world and return it
    pub fn remove_entity(&mut self, key: EntityKey) -> Option<Entity> {
        // Remove from entities first
        if let Some(entity) = self.entities.remove(key) {
            // Clean up name index if the entity had a name
            if let Some(ref name) = entity.name {
                self.name_index.remove(name);
            }
            Some(entity)
        } else {
            None
        }
    }

    /// Get a reference to an entity by key
    pub fn get_entity(&self, key: EntityKey) -> Option<&Entity> {
        self.entities.get(key)
    }

    /// Get a mutable reference to an entity by key
    pub fn get_entity_mut(&mut self, key: EntityKey) -> Option<&mut Entity> {
        self.entities.get_mut(key)
    }

    /// Get an entity by name
    pub fn get_by_name(&self, name: &str) -> Option<(EntityKey, &Entity)> {
        let key = *self.name_index.get(name)?;
        let entity = self.entities.get(key)?;
        Some((key, entity))
    }

    /// Get a mutable reference to an entity by name
    pub fn get_by_name_mut(&mut self, name: &str) -> Option<(EntityKey, &mut Entity)> {
        let key = *self.name_index.get(name)?;
        let entity = self.entities.get_mut(key)?;
        Some((key, entity))
    }

    /// Get all entities with a specific tag
    pub fn get_by_tag<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = (EntityKey, &'a Entity)> {
        self.entities.iter().filter(move |(_, entity)| entity.has_tag(tag))
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
    /// 3. Marks entities as dirty when their transforms change
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
                        // Only update and mark dirty if position actually changed
                        if entity.transform.position != body.position {
                            entity.transform.position = body.position;
                            entity.mark_dirty(DirtyFlags::TRANSFORM);
                        }
                    }
                }
            }
        }
    }

    // --- Dirty tracking methods ---

    /// Check if any entity in the world has dirty flags set
    pub fn has_dirty_entities(&self) -> bool {
        self.entities.values().any(|entity| entity.is_dirty())
    }

    /// Iterate over all dirty entities (entities with any dirty flags set)
    pub fn dirty_entities(&self) -> impl Iterator<Item = (EntityKey, &Entity)> {
        self.entities.iter().filter(|(_, entity)| entity.is_dirty())
    }

    /// Iterate over dirty entities mutably
    pub fn dirty_entities_mut(&mut self) -> impl Iterator<Item = (EntityKey, &mut Entity)> {
        self.entities.iter_mut().filter(|(_, entity)| entity.is_dirty())
    }

    /// Clear dirty flags on all entities
    pub fn clear_all_dirty(&mut self) {
        for entity in self.entities.values_mut() {
            entity.clear_dirty();
        }
    }

    /// Clear all entities from the world
    pub fn clear(&mut self) {
        self.entities.clear();
        self.name_index.clear();
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
        let config = PhysicsConfig::new(0.0);
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

    #[test]
    fn test_get_by_name() {
        let mut world = World::new();

        // Add a named entity
        let entity = make_test_entity().with_name("tesseract");
        let key = world.add_entity(entity);

        // Should be able to find by name
        let result = world.get_by_name("tesseract");
        assert!(result.is_some());
        let (found_key, found_entity) = result.unwrap();
        assert_eq!(found_key, key);
        assert_eq!(found_entity.name, Some("tesseract".to_string()));

        // Non-existent name should return None
        assert!(world.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_get_by_name_mut() {
        let mut world = World::new();

        // Add a named entity
        let entity = make_test_entity().with_name("tesseract");
        let key = world.add_entity(entity);

        // Should be able to get mutable reference by name
        {
            let result = world.get_by_name_mut("tesseract");
            assert!(result.is_some());
            let (found_key, entity) = result.unwrap();
            assert_eq!(found_key, key);
            entity.material = Material::RED;
        }

        // Verify the mutation worked
        let entity = world.get_entity(key).unwrap();
        assert_eq!(entity.material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_get_by_tag() {
        let mut world = World::new();

        // Add entities with different tags
        let dynamic1 = make_test_entity().with_tag("dynamic").with_name("dyn1");
        let dynamic2 = make_test_entity().with_tag("dynamic").with_name("dyn2");
        let static1 = make_test_entity().with_tag("static").with_name("stat1");
        let _key1 = world.add_entity(dynamic1);
        let _key2 = world.add_entity(dynamic2);
        let _key3 = world.add_entity(static1);

        // Should find 2 dynamic entities
        let dynamic_entities: Vec<_> = world.get_by_tag("dynamic").collect();
        assert_eq!(dynamic_entities.len(), 2);

        // Should find 1 static entity
        let static_entities: Vec<_> = world.get_by_tag("static").collect();
        assert_eq!(static_entities.len(), 1);
        assert_eq!(static_entities[0].1.name, Some("stat1".to_string()));

        // Non-existent tag should return empty iterator
        let none_entities: Vec<_> = world.get_by_tag("nonexistent").collect();
        assert!(none_entities.is_empty());
    }

    #[test]
    fn test_name_index_cleanup_on_remove() {
        let mut world = World::new();

        // Add a named entity
        let entity = make_test_entity().with_name("tesseract");
        let key = world.add_entity(entity);

        // Should be able to find by name
        assert!(world.get_by_name("tesseract").is_some());

        // Remove the entity
        world.remove_entity(key);

        // Name should no longer be in the index
        assert!(world.get_by_name("tesseract").is_none());
    }

    #[test]
    fn test_name_index_cleanup_on_clear() {
        let mut world = World::new();

        // Add named entities
        world.add_entity(make_test_entity().with_name("entity1"));
        world.add_entity(make_test_entity().with_name("entity2"));

        // Should be able to find by name
        assert!(world.get_by_name("entity1").is_some());
        assert!(world.get_by_name("entity2").is_some());

        // Clear the world
        world.clear();

        // Names should no longer be in the index
        assert!(world.get_by_name("entity1").is_none());
        assert!(world.get_by_name("entity2").is_none());
    }

    #[test]
    fn test_entity_without_name() {
        let mut world = World::new();

        // Add an unnamed entity
        let entity = make_test_entity();
        let key = world.add_entity(entity);

        // Entity should exist but not be findable by any name
        assert!(world.get_entity(key).is_some());
        assert!(world.get_by_name("").is_none());
    }

    // --- Dirty tracking tests ---

    #[test]
    fn test_new_entities_are_dirty() {
        let mut world = World::new();
        let key = world.add_entity(make_test_entity());

        // New entities should be dirty (DirtyFlags::ALL)
        let entity = world.get_entity(key).unwrap();
        assert!(entity.is_dirty());
        assert!(world.has_dirty_entities());
    }

    #[test]
    fn test_clear_all_dirty() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        // Both should be dirty initially
        assert!(world.has_dirty_entities());
        assert_eq!(world.dirty_entities().count(), 2);

        // Clear all dirty flags
        world.clear_all_dirty();

        // None should be dirty now
        assert!(!world.has_dirty_entities());
        assert_eq!(world.dirty_entities().count(), 0);
    }

    #[test]
    fn test_dirty_entities_iterator() {
        let mut world = World::new();
        let key1 = world.add_entity(make_test_entity());
        let key2 = world.add_entity(make_test_entity());

        // Clear dirty flags
        world.clear_all_dirty();

        // Manually mark one as dirty
        if let Some(entity) = world.get_entity_mut(key1) {
            entity.mark_dirty(DirtyFlags::TRANSFORM);
        }

        // Only one should be dirty
        let dirty: Vec<_> = world.dirty_entities().collect();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].0, key1);

        // The other should not be dirty
        let entity2 = world.get_entity(key2).unwrap();
        assert!(!entity2.is_dirty());
    }

    #[test]
    fn test_physics_sync_marks_dirty() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with physics enabled (no gravity for predictable test)
        let config = PhysicsConfig::new(0.0);
        let mut world = World::new().with_physics(config);

        // Add a physics body with horizontal velocity
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5)
            .with_velocity(Vec4::new(10.0, 0.0, 0.0, 0.0));
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Clear dirty flags
        world.clear_all_dirty();
        assert!(!world.has_dirty_entities());

        // Step physics - entity should move and become dirty
        world.update(1.0);

        // Entity should now be dirty
        let entity = world.get_entity(entity_handle).unwrap();
        assert!(entity.is_dirty());
        assert!(entity.dirty_flags().contains(DirtyFlags::TRANSFORM));
    }

    #[test]
    fn test_physics_sync_no_change_not_dirty() {
        use rust4d_physics::RigidBody4D;
        use rust4d_math::Vec4;

        // Create a world with physics (no gravity, no velocity = no movement)
        let config = PhysicsConfig::new(0.0);
        let mut world = World::new().with_physics(config);

        // Add a stationary physics body
        let body = RigidBody4D::new_sphere(Vec4::new(0.0, 0.0, 0.0, 0.0), 0.5);
        let body_handle = world.physics_mut().unwrap().add_body(body);

        // Create an entity linked to the physics body
        let entity = make_test_entity().with_physics_body(body_handle);
        let entity_handle = world.add_entity(entity);

        // Clear dirty flags
        world.clear_all_dirty();

        // Step physics - no movement should occur
        world.update(1.0);

        // Entity should NOT be dirty (position didn't change)
        let entity = world.get_entity(entity_handle).unwrap();
        assert!(!entity.is_dirty());
    }

    #[test]
    fn test_dirty_entities_mut() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        // Clear dirty flags
        world.clear_all_dirty();

        // Mark first entity dirty manually
        for entity in world.iter_mut().take(1) {
            entity.mark_dirty(DirtyFlags::TRANSFORM);
        }

        // Use dirty_entities_mut to clear dirty on just the dirty entities
        for (_, entity) in world.dirty_entities_mut() {
            entity.material = Material::RED;
            entity.clear_dirty();
        }

        // No entities should be dirty now
        assert!(!world.has_dirty_entities());
    }
}
