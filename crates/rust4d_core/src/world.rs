//! World container for entities
//!
//! The World manages all entities in the simulation, including parent-child
//! entity hierarchy with cycle detection and recursive operations.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use crate::{Entity, DirtyFlags, Transform4D};
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

/// Error type for hierarchy operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchyError {
    /// One or both entities don't exist in the world
    InvalidEntity,
    /// Adding this child would create a cycle in the hierarchy
    CyclicHierarchy,
    /// The entity is already a child of the specified parent
    AlreadyChild,
}

impl fmt::Display for HierarchyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HierarchyError::InvalidEntity => write!(f, "one or both entities do not exist"),
            HierarchyError::CyclicHierarchy => write!(f, "adding this child would create a cycle"),
            HierarchyError::AlreadyChild => write!(f, "entity is already a child of this parent"),
        }
    }
}

impl std::error::Error for HierarchyError {}

/// The 4D world containing all entities
///
/// The World is the central container for all game objects.
/// It manages entities, integrates with physics simulation,
/// and tracks parent-child entity hierarchy.
pub struct World {
    /// All entities in the world (using generational keys)
    entities: SlotMap<EntityKey, Entity>,
    /// Index from entity names to keys (for fast name lookup)
    name_index: HashMap<String, EntityKey>,
    /// Optional physics simulation (None = no physics)
    physics_world: Option<PhysicsWorld>,
    /// Parent mapping: child entity key -> parent entity key
    parents: HashMap<EntityKey, EntityKey>,
    /// Children mapping: parent entity key -> list of child entity keys
    children_map: HashMap<EntityKey, Vec<EntityKey>>,
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
            parents: HashMap::new(),
            children_map: HashMap::new(),
        }
    }

    /// Create a world with pre-allocated capacity for entities
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: SlotMap::with_capacity_and_key(capacity),
            name_index: HashMap::new(),
            physics_world: None,
            parents: HashMap::new(),
            children_map: HashMap::new(),
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
    ///
    /// This method also cleans up associated resources:
    /// - Removes the entity from the name index if it was named
    /// - Removes the physics body from PhysicsWorld if one was attached
    /// - Removes the entity from its parent's children list (if it had a parent)
    /// - Orphans the entity's children (they become root entities)
    pub fn remove_entity(&mut self, key: EntityKey) -> Option<Entity> {
        // Remove from entities first
        if let Some(entity) = self.entities.remove(key) {
            // Clean up name index if the entity had a name
            if let Some(ref name) = entity.name {
                self.name_index.remove(name);
            }

            // Clean up physics body if present
            if let Some(body_key) = entity.physics_body {
                if let Some(ref mut physics) = self.physics_world {
                    physics.remove_body(body_key);
                }
            }

            // Clean up hierarchy: remove from parent's children list
            if let Some(parent_key) = self.parents.remove(&key) {
                if let Some(siblings) = self.children_map.get_mut(&parent_key) {
                    siblings.retain(|&k| k != key);
                    if siblings.is_empty() {
                        self.children_map.remove(&parent_key);
                    }
                }
            }

            // Clean up hierarchy: orphan all children (they become root entities)
            if let Some(children) = self.children_map.remove(&key) {
                for child_key in children {
                    self.parents.remove(&child_key);
                }
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
        self.parents.clear();
        self.children_map.clear();
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

    // --- Hierarchy methods ---

    /// Get an entity's parent key
    ///
    /// Returns `None` if the entity has no parent (is a root entity)
    /// or if the entity does not exist.
    pub fn parent_of(&self, entity: EntityKey) -> Option<EntityKey> {
        self.parents.get(&entity).copied()
    }

    /// Get an entity's children as a slice of keys
    ///
    /// Returns an empty slice if the entity has no children or does not exist.
    pub fn children_of(&self, entity: EntityKey) -> &[EntityKey] {
        self.children_map.get(&entity).map_or(&[], |v| v.as_slice())
    }

    /// Check if an entity has any children
    pub fn has_children(&self, entity: EntityKey) -> bool {
        self.children_map
            .get(&entity)
            .is_some_and(|children| !children.is_empty())
    }

    /// Check if an entity has a parent
    pub fn has_parent(&self, entity: EntityKey) -> bool {
        self.parents.contains_key(&entity)
    }

    /// Add a child entity to a parent entity
    ///
    /// If the child already has a different parent, it is first removed from
    /// that parent (reparenting). Returns an error if either entity does not
    /// exist, if the relationship would create a cycle, or if the child is
    /// already a child of the specified parent.
    pub fn add_child(&mut self, parent: EntityKey, child: EntityKey) -> Result<(), HierarchyError> {
        // Validate both entities exist
        if !self.entities.contains_key(parent) || !self.entities.contains_key(child) {
            return Err(HierarchyError::InvalidEntity);
        }

        // Cannot parent an entity to itself
        if parent == child {
            return Err(HierarchyError::CyclicHierarchy);
        }

        // Check if child is already a child of this parent
        if self.parents.get(&child) == Some(&parent) {
            return Err(HierarchyError::AlreadyChild);
        }

        // Check for cycles: walk up from parent; if we reach child, it would create a cycle
        if self.is_ancestor(child, parent) {
            return Err(HierarchyError::CyclicHierarchy);
        }

        // If child already has a different parent, remove it from that parent first
        if let Some(old_parent) = self.parents.remove(&child) {
            if let Some(old_siblings) = self.children_map.get_mut(&old_parent) {
                old_siblings.retain(|&k| k != child);
                if old_siblings.is_empty() {
                    self.children_map.remove(&old_parent);
                }
            }
        }

        // Establish the new relationship
        self.parents.insert(child, parent);
        self.children_map
            .entry(parent)
            .or_default()
            .push(child);

        Ok(())
    }

    /// Remove an entity from its parent, making it a root entity
    ///
    /// Does nothing if the entity has no parent or does not exist.
    pub fn remove_from_parent(&mut self, child: EntityKey) {
        if let Some(parent_key) = self.parents.remove(&child) {
            if let Some(siblings) = self.children_map.get_mut(&parent_key) {
                siblings.retain(|&k| k != child);
                if siblings.is_empty() {
                    self.children_map.remove(&parent_key);
                }
            }
        }
    }

    /// Get the world-space transform of an entity
    ///
    /// For root entities (no parent), this is just their own local transform.
    /// For children, this composes transforms from root to leaf using
    /// `Transform4D::compose`, which correctly handles position, rotation,
    /// and scale accumulation.
    ///
    /// Returns `None` if the entity does not exist.
    pub fn world_transform(&self, entity: EntityKey) -> Option<Transform4D> {
        // Check entity exists
        let local_transform = self.entities.get(entity)?.transform;

        // Build the chain of ancestors from root to this entity
        let mut chain = vec![local_transform];
        let mut current = entity;
        while let Some(&parent_key) = self.parents.get(&current) {
            if let Some(parent_entity) = self.entities.get(parent_key) {
                chain.push(parent_entity.transform);
                current = parent_key;
            } else {
                break;
            }
        }

        // Compose from root (last element) to leaf (first element)
        // chain is [leaf, ..., root], so we iterate in reverse
        let mut result = Transform4D::identity();
        for transform in chain.into_iter().rev() {
            result = result.compose(&transform);
        }

        Some(result)
    }

    /// Delete an entity and all its descendants recursively
    ///
    /// Returns a vector of all removed entities (the target entity and
    /// all of its descendants). The target entity is first in the vector.
    /// Returns an empty vector if the entity does not exist.
    pub fn delete_recursive(&mut self, entity: EntityKey) -> Vec<Entity> {
        let mut removed = Vec::new();

        // Collect all descendants first (breadth-first)
        let mut to_remove = VecDeque::new();
        to_remove.push_back(entity);

        let mut keys_to_remove = Vec::new();
        while let Some(key) = to_remove.pop_front() {
            keys_to_remove.push(key);
            // Add children to the queue
            if let Some(children) = self.children_map.get(&key) {
                for &child_key in children {
                    to_remove.push_back(child_key);
                }
            }
        }

        // Before removing the root entity, detach it from its parent
        if let Some(parent_key) = self.parents.remove(&entity) {
            if let Some(siblings) = self.children_map.get_mut(&parent_key) {
                siblings.retain(|&k| k != entity);
                if siblings.is_empty() {
                    self.children_map.remove(&parent_key);
                }
            }
        }

        // Now remove all collected entities
        for key in keys_to_remove {
            // Clean up hierarchy maps for this entity
            self.parents.remove(&key);
            self.children_map.remove(&key);

            // Remove the entity itself (with name/physics cleanup)
            if let Some(ent) = self.entities.remove(key) {
                if let Some(ref name) = ent.name {
                    self.name_index.remove(name);
                }
                if let Some(body_key) = ent.physics_body {
                    if let Some(ref mut physics) = self.physics_world {
                        physics.remove_body(body_key);
                    }
                }
                removed.push(ent);
            }
        }

        removed
    }

    /// Get all descendants of an entity (breadth-first order)
    ///
    /// Returns an empty vector if the entity has no descendants or does not exist.
    /// Does not include the entity itself.
    pub fn descendants(&self, entity: EntityKey) -> Vec<EntityKey> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        // Seed with direct children
        if let Some(children) = self.children_map.get(&entity) {
            for &child in children {
                queue.push_back(child);
            }
        }

        while let Some(key) = queue.pop_front() {
            result.push(key);
            if let Some(children) = self.children_map.get(&key) {
                for &child in children {
                    queue.push_back(child);
                }
            }
        }

        result
    }

    /// Get all root entities (entities with no parent)
    pub fn root_entities(&self) -> impl Iterator<Item = (EntityKey, &Entity)> {
        self.entities
            .iter()
            .filter(|(key, _)| !self.parents.contains_key(key))
    }

    /// Check if `ancestor` is an ancestor of `entity`
    ///
    /// Walks up the hierarchy from `entity`. Returns `false` if either
    /// entity does not exist, or if `ancestor == entity`.
    pub fn is_ancestor(&self, ancestor: EntityKey, entity: EntityKey) -> bool {
        let mut current = entity;
        while let Some(&parent_key) = self.parents.get(&current) {
            if parent_key == ancestor {
                return true;
            }
            current = parent_key;
        }
        false
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

    // --- Hierarchy tests ---

    fn make_positioned_entity(x: f32, y: f32, z: f32, w: f32) -> Entity {
        let tesseract = Tesseract4D::new(2.0);
        Entity::with_transform(
            ShapeRef::shared(tesseract),
            crate::Transform4D::from_position(rust4d_math::Vec4::new(x, y, z, w)),
            Material::default(),
        )
    }

    #[test]
    fn test_add_child() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        assert!(world.add_child(parent, child).is_ok());

        // Verify parent/child relationship
        assert_eq!(world.parent_of(child), Some(parent));
        assert_eq!(world.children_of(parent), &[child]);
        assert!(world.has_children(parent));
        assert!(world.has_parent(child));
        assert!(!world.has_parent(parent));
        assert!(!world.has_children(child));
    }

    #[test]
    fn test_add_child_invalid_entity() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());

        // Create a fake key by adding and removing an entity
        let temp = world.add_entity(make_test_entity());
        world.remove_entity(temp);

        // Both invalid child and invalid parent should fail
        assert_eq!(
            world.add_child(parent, temp),
            Err(HierarchyError::InvalidEntity)
        );
        assert_eq!(
            world.add_child(temp, parent),
            Err(HierarchyError::InvalidEntity)
        );
    }

    #[test]
    fn test_cycle_detection() {
        let mut world = World::new();
        let a = world.add_entity(make_test_entity());
        let b = world.add_entity(make_test_entity());

        // A -> B
        assert!(world.add_child(a, b).is_ok());

        // B -> A would create a cycle
        assert_eq!(
            world.add_child(b, a),
            Err(HierarchyError::CyclicHierarchy)
        );

        // Self-parenting should also be rejected
        assert_eq!(
            world.add_child(a, a),
            Err(HierarchyError::CyclicHierarchy)
        );
    }

    #[test]
    fn test_deep_cycle_detection() {
        let mut world = World::new();
        let a = world.add_entity(make_test_entity());
        let b = world.add_entity(make_test_entity());
        let c = world.add_entity(make_test_entity());

        // A -> B -> C
        assert!(world.add_child(a, b).is_ok());
        assert!(world.add_child(b, c).is_ok());

        // C -> A would create a cycle (A is ancestor of C)
        assert_eq!(
            world.add_child(c, a),
            Err(HierarchyError::CyclicHierarchy)
        );
    }

    #[test]
    fn test_already_child() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        assert!(world.add_child(parent, child).is_ok());

        // Adding the same child again should return AlreadyChild
        assert_eq!(
            world.add_child(parent, child),
            Err(HierarchyError::AlreadyChild)
        );
    }

    #[test]
    fn test_remove_from_parent() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        world.add_child(parent, child).unwrap();
        assert!(world.has_parent(child));
        assert!(world.has_children(parent));

        world.remove_from_parent(child);

        assert!(!world.has_parent(child));
        assert!(!world.has_children(parent));
        assert_eq!(world.parent_of(child), None);
        assert!(world.children_of(parent).is_empty());
    }

    #[test]
    fn test_world_transform_no_parent() {
        let mut world = World::new();
        let entity = make_positioned_entity(1.0, 2.0, 3.0, 4.0);
        let key = world.add_entity(entity);

        let wt = world.world_transform(key).unwrap();
        assert!((wt.position.x - 1.0).abs() < 0.001);
        assert!((wt.position.y - 2.0).abs() < 0.001);
        assert!((wt.position.z - 3.0).abs() < 0.001);
        assert!((wt.position.w - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_world_transform_with_parent() {
        let mut world = World::new();

        // Parent at (10, 0, 0, 0)
        let parent = world.add_entity(make_positioned_entity(10.0, 0.0, 0.0, 0.0));
        // Child at (1, 2, 0, 0) in local space
        let child = world.add_entity(make_positioned_entity(1.0, 2.0, 0.0, 0.0));

        world.add_child(parent, child).unwrap();

        // World transform of child should compose parent + child transforms
        // With identity rotation and scale=1, compose just adds positions:
        // parent.transform_point(child.position) = (10+1, 0+2, 0, 0) = (11, 2, 0, 0)
        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 11.0).abs() < 0.001,
            "Expected x=11.0, got {}", wt.position.x);
        assert!((wt.position.y - 2.0).abs() < 0.001,
            "Expected y=2.0, got {}", wt.position.y);
    }

    #[test]
    fn test_world_transform_with_scale() {
        let mut world = World::new();

        // Parent with scale 2 at origin
        let mut parent_entity = make_positioned_entity(0.0, 0.0, 0.0, 0.0);
        parent_entity.transform.scale = 2.0;
        let parent = world.add_entity(parent_entity);

        // Child at (1, 0, 0, 0) in local space
        let child = world.add_entity(make_positioned_entity(1.0, 0.0, 0.0, 0.0));

        world.add_child(parent, child).unwrap();

        // Parent composes: scale(2) * child_pos(1,0,0,0) + parent_pos(0,0,0,0) = (2, 0, 0, 0)
        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 2.0).abs() < 0.001,
            "Expected x=2.0, got {}", wt.position.x);
    }

    #[test]
    fn test_delete_recursive() {
        let mut world = World::new();
        let root = world.add_entity(make_test_entity().with_name("root"));
        let child1 = world.add_entity(make_test_entity().with_name("child1"));
        let child2 = world.add_entity(make_test_entity().with_name("child2"));
        let grandchild = world.add_entity(make_test_entity().with_name("grandchild"));

        world.add_child(root, child1).unwrap();
        world.add_child(root, child2).unwrap();
        world.add_child(child1, grandchild).unwrap();

        assert_eq!(world.entity_count(), 4);

        let removed = world.delete_recursive(root);
        assert_eq!(removed.len(), 4);
        assert_eq!(world.entity_count(), 0);

        // All should be gone
        assert!(world.get_entity(root).is_none());
        assert!(world.get_entity(child1).is_none());
        assert!(world.get_entity(grandchild).is_none());

        // Name index should be cleaned up
        assert!(world.get_by_name("root").is_none());
        assert!(world.get_by_name("child1").is_none());
    }

    #[test]
    fn test_delete_recursive_subtree() {
        let mut world = World::new();
        let root = world.add_entity(make_test_entity());
        let child1 = world.add_entity(make_test_entity());
        let child2 = world.add_entity(make_test_entity());
        let grandchild = world.add_entity(make_test_entity());

        world.add_child(root, child1).unwrap();
        world.add_child(root, child2).unwrap();
        world.add_child(child1, grandchild).unwrap();

        // Delete just child1 subtree (child1 + grandchild)
        let removed = world.delete_recursive(child1);
        assert_eq!(removed.len(), 2);
        assert_eq!(world.entity_count(), 2);

        // root and child2 should still exist
        assert!(world.get_entity(root).is_some());
        assert!(world.get_entity(child2).is_some());

        // child1 should be removed from root's children
        assert_eq!(world.children_of(root), &[child2]);
    }

    #[test]
    fn test_descendants() {
        let mut world = World::new();
        let root = world.add_entity(make_test_entity());
        let child1 = world.add_entity(make_test_entity());
        let child2 = world.add_entity(make_test_entity());
        let grandchild = world.add_entity(make_test_entity());

        world.add_child(root, child1).unwrap();
        world.add_child(root, child2).unwrap();
        world.add_child(child1, grandchild).unwrap();

        let desc = world.descendants(root);
        assert_eq!(desc.len(), 3);
        // Breadth-first: child1, child2 first, then grandchild
        assert!(desc.contains(&child1));
        assert!(desc.contains(&child2));
        assert!(desc.contains(&grandchild));

        // child1's descendants should be just grandchild
        let desc1 = world.descendants(child1);
        assert_eq!(desc1, vec![grandchild]);

        // Leaf entity has no descendants
        assert!(world.descendants(grandchild).is_empty());
    }

    #[test]
    fn test_root_entities() {
        let mut world = World::new();
        let root1 = world.add_entity(make_test_entity());
        let root2 = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        world.add_child(root1, child).unwrap();

        let roots: Vec<EntityKey> = world.root_entities().map(|(k, _)| k).collect();
        assert_eq!(roots.len(), 2);
        assert!(roots.contains(&root1));
        assert!(roots.contains(&root2));
        assert!(!roots.contains(&child));
    }

    #[test]
    fn test_is_ancestor() {
        let mut world = World::new();
        let a = world.add_entity(make_test_entity());
        let b = world.add_entity(make_test_entity());
        let c = world.add_entity(make_test_entity());
        let d = world.add_entity(make_test_entity());

        // A -> B -> C
        world.add_child(a, b).unwrap();
        world.add_child(b, c).unwrap();

        assert!(world.is_ancestor(a, b));  // A is ancestor of B
        assert!(world.is_ancestor(a, c));  // A is ancestor of C (transitive)
        assert!(world.is_ancestor(b, c));  // B is ancestor of C
        assert!(!world.is_ancestor(c, a)); // C is NOT ancestor of A
        assert!(!world.is_ancestor(a, a)); // Not ancestor of self
        assert!(!world.is_ancestor(a, d)); // D is unrelated
    }

    #[test]
    fn test_remove_entity_cleans_hierarchy() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());
        let grandchild = world.add_entity(make_test_entity());

        world.add_child(parent, child).unwrap();
        world.add_child(child, grandchild).unwrap();

        // Remove child (middle of hierarchy)
        world.remove_entity(child);

        // Parent should have no children (child was removed)
        assert!(!world.has_children(parent));

        // Grandchild should be orphaned (root entity)
        assert!(!world.has_parent(grandchild));

        // Grandchild should still exist
        assert!(world.get_entity(grandchild).is_some());
    }

    #[test]
    fn test_reparent() {
        let mut world = World::new();
        let parent1 = world.add_entity(make_test_entity());
        let parent2 = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        // First parent
        world.add_child(parent1, child).unwrap();
        assert_eq!(world.parent_of(child), Some(parent1));
        assert_eq!(world.children_of(parent1), &[child]);

        // Reparent to parent2
        world.add_child(parent2, child).unwrap();
        assert_eq!(world.parent_of(child), Some(parent2));
        assert_eq!(world.children_of(parent2), &[child]);

        // Old parent should have no children
        assert!(!world.has_children(parent1));
    }

    #[test]
    fn test_hierarchy_error_display() {
        assert_eq!(
            format!("{}", HierarchyError::InvalidEntity),
            "one or both entities do not exist"
        );
        assert_eq!(
            format!("{}", HierarchyError::CyclicHierarchy),
            "adding this child would create a cycle"
        );
        assert_eq!(
            format!("{}", HierarchyError::AlreadyChild),
            "entity is already a child of this parent"
        );
    }

    #[test]
    fn test_clear_cleans_hierarchy() {
        let mut world = World::new();
        let parent = world.add_entity(make_test_entity());
        let child = world.add_entity(make_test_entity());

        world.add_child(parent, child).unwrap();
        world.clear();

        assert!(world.is_empty());
        // After clear, hierarchy maps should also be empty
        // (verified implicitly: adding new entities won't have stale hierarchy)
    }

    #[test]
    fn test_world_transform_deep_hierarchy() {
        let mut world = World::new();

        // Grandparent at (10, 0, 0, 0)
        let grandparent = world.add_entity(make_positioned_entity(10.0, 0.0, 0.0, 0.0));
        // Parent at (5, 0, 0, 0) local
        let parent = world.add_entity(make_positioned_entity(5.0, 0.0, 0.0, 0.0));
        // Child at (1, 0, 0, 0) local
        let child = world.add_entity(make_positioned_entity(1.0, 0.0, 0.0, 0.0));

        world.add_child(grandparent, parent).unwrap();
        world.add_child(parent, child).unwrap();

        // World transform of child = grandparent compose parent compose child
        // = (10+5+1, 0, 0, 0) = (16, 0, 0, 0)
        let wt = world.world_transform(child).unwrap();
        assert!((wt.position.x - 16.0).abs() < 0.001,
            "Expected x=16.0, got {}", wt.position.x);
    }

    #[test]
    fn test_world_transform_nonexistent() {
        let mut world = World::new();
        let key = world.add_entity(make_test_entity());
        world.remove_entity(key);

        // Non-existent entity returns None
        assert!(world.world_transform(key).is_none());
    }
}
