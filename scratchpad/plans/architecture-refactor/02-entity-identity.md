# Phase 2: Entity Identity

**Status:** Not Started
**Sessions:** 1
**Dependencies:** Phase 1 (Foundation)
**Parallelizable With:** Phase 3 (Materials)

---

## Goal

Add entity names and tags for easy lookup. Fix the fragile `if idx == 0` pattern that identifies entities by array position.

---

## Problem

Current code identifies entities by index:
```rust
// main.rs build_geometry()
for (idx, entity) in world.iter().enumerate() {
    if idx == 0 {
        // Tesseract - use position gradient
    } else {
        // Floor - use checkerboard
    }
}
```

This breaks if entities are reordered, added, or removed.

---

## Solution

Add optional name and tags to Entity:

```rust
pub struct Entity {
    pub name: Option<String>,
    pub tags: HashSet<String>,
    // ... existing fields
}

impl World {
    pub fn get_by_name(&self, name: &str) -> Option<(EntityKey, &Entity)>;
    pub fn get_by_tag(&self, tag: &str) -> impl Iterator<Item = (EntityKey, &Entity)>;
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/rust4d_core/src/entity.rs` | Add `name`, `tags` fields |
| `crates/rust4d_core/src/world.rs` | Add name index, lookup methods |
| `src/main.rs` | Use named entities instead of index |

---

## Implementation Steps

### Wave 1: Entity Fields (Sequential)

1. Update `Entity` in `entity.rs`:
   ```rust
   use std::collections::HashSet;

   pub struct Entity {
       pub name: Option<String>,
       pub tags: HashSet<String>,
       pub transform: Transform4D,
       pub shape: ShapeRef,
       pub material: Material,
       pub physics_body: Option<BodyKey>,
   }
   ```

2. Add builder methods:
   ```rust
   impl Entity {
       pub fn with_name(mut self, name: impl Into<String>) -> Self {
           self.name = Some(name.into());
           self
       }

       pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
           self.tags.insert(tag.into());
           self
       }

       pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
           for tag in tags {
               self.tags.insert(tag.into());
           }
           self
       }
   }
   ```

3. Update constructors to initialize empty `tags: HashSet::new()` and `name: None`

### Wave 2: World Lookup (Sequential)

1. Add name index to `World`:
   ```rust
   use std::collections::HashMap;

   pub struct World {
       entities: SlotMap<EntityKey, Entity>,
       name_index: HashMap<String, EntityKey>,
       physics_world: Option<PhysicsWorld>,
   }
   ```

2. Update `add_entity` to maintain index:
   ```rust
   pub fn add_entity(&mut self, entity: Entity) -> EntityKey {
       let key = self.entities.insert(entity);
       if let Some(name) = &self.entities[key].name {
           self.name_index.insert(name.clone(), key);
       }
       key
   }
   ```

3. Add lookup methods:
   ```rust
   pub fn get_by_name(&self, name: &str) -> Option<(EntityKey, &Entity)> {
       self.name_index.get(name)
           .and_then(|&key| self.entities.get(key).map(|e| (key, e)))
   }

   pub fn get_by_name_mut(&mut self, name: &str) -> Option<(EntityKey, &mut Entity)> {
       self.name_index.get(name).copied()
           .and_then(move |key| self.entities.get_mut(key).map(|e| (key, e)))
   }

   pub fn get_by_tag(&self, tag: &str) -> impl Iterator<Item = (EntityKey, &Entity)> {
       self.entities.iter()
           .filter(move |(_, e)| e.tags.contains(tag))
   }
   ```

4. Add remove method that cleans up index:
   ```rust
   pub fn remove_entity(&mut self, key: EntityKey) -> Option<Entity> {
       if let Some(entity) = self.entities.remove(key) {
           if let Some(name) = &entity.name {
               self.name_index.remove(name);
           }
           Some(entity)
       } else {
           None
       }
   }
   ```

### Wave 3: Main Integration (Sequential)

1. Update entity creation in `main.rs`:
   ```rust
   // Tesseract
   let tesseract_entity = world.add_entity(
       Entity::with_material(tesseract_shape, tesseract_material)
           .with_name("tesseract")
           .with_tag("dynamic")
           .with_physics_body(tesseract_body)
   );

   // Floor
   world.add_entity(
       Entity::new(floor_shape)
           .with_name("floor")
           .with_tag("static")
   );
   ```

2. Update `build_geometry` to use names/tags:
   ```rust
   fn build_geometry(world: &World) -> RenderableGeometry {
       let mut geometry = RenderableGeometry::new();

       for (key, entity) in world.iter() {
           let color_fn = if entity.tags.contains("dynamic") {
               &position_gradient_color
           } else {
               &checkerboard_color
           };
           geometry.add_entity_with_color(entity, color_fn);
       }

       geometry
   }
   ```

3. Or use name for specific entities:
   ```rust
   if let Some((_, tesseract)) = world.get_by_name("tesseract") {
       // Handle tesseract specifically
   }
   ```

---

## Commits

1. "Add name and tags fields to Entity"
2. "Add name index and lookup methods to World"
3. "Use named entities in main.rs"

---

## Verification

1. **Unit tests:**
   ```rust
   #[test]
   fn test_entity_lookup_by_name() {
       let mut world = World::new();
       let key = world.add_entity(Entity::new(shape).with_name("player"));

       let (found_key, entity) = world.get_by_name("player").unwrap();
       assert_eq!(found_key, key);
       assert_eq!(entity.name.as_deref(), Some("player"));
   }

   #[test]
   fn test_entity_lookup_by_tag() {
       let mut world = World::new();
       world.add_entity(Entity::new(shape1).with_tag("enemy"));
       world.add_entity(Entity::new(shape2).with_tag("enemy"));
       world.add_entity(Entity::new(shape3).with_tag("player"));

       let enemies: Vec<_> = world.get_by_tag("enemy").collect();
       assert_eq!(enemies.len(), 2);
   }
   ```

2. **Manual test:** Reorder entity creation, verify behavior unchanged

3. **No index-based identification** remaining in codebase:
   ```bash
   grep -r "idx == 0" src/
   # Should return nothing
   ```

---

## Rollback Plan

Names and tags are additive. If issues arise, make them unused but keep the fields.
