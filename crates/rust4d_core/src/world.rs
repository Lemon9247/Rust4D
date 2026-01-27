//! World container for entities
//!
//! The World manages all entities in the simulation.

use crate::Entity;

/// A handle to an entity in the world
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityHandle(usize);

impl EntityHandle {
    /// Get the raw index of this handle
    #[inline]
    pub fn index(&self) -> usize {
        self.0
    }
}

/// The 4D world containing all entities
///
/// The World is the central container for all game objects.
/// It manages entities and will eventually integrate with physics.
pub struct World {
    /// All entities in the world
    entities: Vec<Entity>,
    // Future: physics_world: PhysicsWorld,
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
            entities: Vec::new(),
        }
    }

    /// Create a world with pre-allocated capacity for entities
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entities: Vec::with_capacity(capacity),
        }
    }

    /// Add an entity to the world, returning its handle
    pub fn add_entity(&mut self, entity: Entity) -> EntityHandle {
        let handle = EntityHandle(self.entities.len());
        self.entities.push(entity);
        handle
    }

    /// Get a reference to an entity by handle
    pub fn get_entity(&self, handle: EntityHandle) -> Option<&Entity> {
        self.entities.get(handle.0)
    }

    /// Get a mutable reference to an entity by handle
    pub fn get_entity_mut(&mut self, handle: EntityHandle) -> Option<&mut Entity> {
        self.entities.get_mut(handle.0)
    }

    /// Get all entities as a slice
    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    /// Get all entities as a mutable slice
    pub fn entities_mut(&mut self) -> &mut [Entity] {
        &mut self.entities
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

    /// Update the world (future: steps physics)
    ///
    /// Currently a no-op, but will eventually:
    /// - Step the physics simulation
    /// - Update entity transforms from rigid bodies
    pub fn update(&mut self, _dt: f32) {
        // Future: self.physics_world.step(dt);
        // Future: sync transforms from physics
    }

    /// Clear all entities from the world
    pub fn clear(&mut self) {
        self.entities.clear();
    }

    /// Iterate over all entities
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }

    /// Iterate over all entities mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.iter_mut()
    }

    /// Iterate over handles and entities
    pub fn iter_with_handles(&self) -> impl Iterator<Item = (EntityHandle, &Entity)> {
        self.entities
            .iter()
            .enumerate()
            .map(|(i, e)| (EntityHandle(i), e))
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
        let handle = world.add_entity(entity);

        assert_eq!(handle.index(), 0);
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
    fn test_world_entities() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        let entities = world.entities();
        assert_eq!(entities.len(), 2);
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
    fn test_world_iter_with_handles() {
        let mut world = World::new();
        world.add_entity(make_test_entity());
        world.add_entity(make_test_entity());

        let handles: Vec<_> = world.iter_with_handles().map(|(h, _)| h).collect();
        assert_eq!(handles.len(), 2);
        assert_eq!(handles[0].index(), 0);
        assert_eq!(handles[1].index(), 1);
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
    fn test_entity_handle() {
        let handle = EntityHandle(42);
        assert_eq!(handle.index(), 42);

        // Test PartialEq
        let handle2 = EntityHandle(42);
        assert_eq!(handle, handle2);
    }
}
