# Phase 7: Rendering Optimization

**Status:** Not Started
**Sessions:** 1-2
**Dependencies:** Phase 1 (Handles), Phase 2 (Entity Identity)
**Parallelizable With:** Phases 3-6 (independent of physics changes)

---

## Goal

Add dirty tracking to avoid rebuilding all geometry when one entity moves. Current implementation is O(n) for any change.

---

## Problem

Current code rebuilds everything on any entity movement:

```rust
// main.rs
if current_pos != self.last_tesseract_pos {
    self.geometry = Self::build_geometry(&self.world);  // FULL REBUILD
    slice_pipeline.upload_tetrahedra(...);              // FULL RE-UPLOAD
}
```

This becomes expensive as entity count grows.

---

## Solution

Track which entities changed and only update those:

```rust
pub struct Entity {
    // ...
    dirty: DirtyFlags,
}

bitflags! {
    pub struct DirtyFlags: u8 {
        const TRANSFORM = 1 << 0;
        const MESH = 1 << 1;
        const MATERIAL = 1 << 2;
    }
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `crates/rust4d_core/src/entity.rs` | Add dirty flags |
| `crates/rust4d_core/src/world.rs` | Track dirty entities, mark on physics sync |
| `crates/rust4d_render/src/renderable.rs` | Support incremental updates |
| `src/main.rs` | Use incremental geometry updates |

---

## Implementation Steps

### Wave 1: Dirty Flags (Sequential)

1. Add flags to `entity.rs`:
   ```rust
   use bitflags::bitflags;

   bitflags! {
       #[derive(Clone, Copy, Debug, Default)]
       pub struct DirtyFlags: u8 {
           const NONE = 0;
           const TRANSFORM = 1 << 0;
           const MESH = 1 << 1;
           const MATERIAL = 1 << 2;
           const ALL = Self::TRANSFORM.bits() | Self::MESH.bits() | Self::MATERIAL.bits();
       }
   }

   pub struct Entity {
       pub transform: Transform4D,
       pub shape: ShapeRef,
       pub material: Material,
       pub physics_body: Option<BodyKey>,
       pub name: Option<String>,
       pub tags: HashSet<String>,
       dirty: DirtyFlags,
   }

   impl Entity {
       pub fn is_dirty(&self) -> bool {
           !self.dirty.is_empty()
       }

       pub fn dirty_flags(&self) -> DirtyFlags {
           self.dirty
       }

       pub fn mark_dirty(&mut self, flags: DirtyFlags) {
           self.dirty |= flags;
       }

       pub fn clear_dirty(&mut self) {
           self.dirty = DirtyFlags::NONE;
       }

       pub fn set_position(&mut self, position: Vec4) {
           if self.transform.position != position {
               self.transform.position = position;
               self.dirty |= DirtyFlags::TRANSFORM;
           }
       }

       pub fn set_rotation(&mut self, rotation: Rotor4) {
           // Similar pattern
           self.transform.rotation = rotation;
           self.dirty |= DirtyFlags::TRANSFORM;
       }
   }
   ```

### Wave 2: World Dirty Tracking (Sequential)

1. Update `World` to track and query dirty entities:
   ```rust
   impl World {
       /// Get all entities that have been modified
       pub fn dirty_entities(&self) -> impl Iterator<Item = (EntityKey, &Entity)> {
           self.entities.iter().filter(|(_, e)| e.is_dirty())
       }

       /// Clear dirty flags after processing
       pub fn clear_all_dirty(&mut self) {
           for (_, entity) in &mut self.entities {
               entity.clear_dirty();
           }
       }

       /// Check if any entity is dirty
       pub fn has_dirty_entities(&self) -> bool {
           self.entities.values().any(|e| e.is_dirty())
       }
   }
   ```

2. Update physics sync in `update()`:
   ```rust
   pub fn update(&mut self, dt: f32) {
       if let Some(physics) = &mut self.physics_world {
           physics.step(dt);

           // Sync entity transforms from physics, marking dirty
           for (_, entity) in &mut self.entities {
               if let Some(body_key) = entity.physics_body {
                   if let Some(body) = physics.get_body(body_key) {
                       if entity.transform.position != body.position {
                           entity.transform.position = body.position;
                           entity.mark_dirty(DirtyFlags::TRANSFORM);
                       }
                   }
               }
           }
       }
   }
   ```

### Wave 3: Renderable Incremental Updates (Sequential)

1. Add per-entity geometry tracking to `RenderableGeometry`:
   ```rust
   pub struct EntityGeometry {
       pub vertex_offset: u32,
       pub vertex_count: u32,
       pub tetra_offset: u32,
       pub tetra_count: u32,
   }

   pub struct RenderableGeometry {
       pub vertices: Vec<Vertex4D>,
       pub tetrahedra: Vec<GpuTetrahedron>,
       entity_ranges: HashMap<EntityKey, EntityGeometry>,
   }

   impl RenderableGeometry {
       pub fn add_entity(&mut self, key: EntityKey, entity: &Entity, color_fn: &impl Fn(Vec4, Material) -> Vec4) {
           let vertex_offset = self.vertices.len() as u32;
           let tetra_offset = self.tetrahedra.len() as u32;

           // Add geometry (existing code)
           self.add_entity_with_color(entity, color_fn);

           // Track range
           self.entity_ranges.insert(key, EntityGeometry {
               vertex_offset,
               vertex_count: self.vertices.len() as u32 - vertex_offset,
               tetra_offset,
               tetra_count: self.tetrahedra.len() as u32 - tetra_offset,
           });
       }

       /// Update just one entity's geometry
       pub fn update_entity(&mut self, key: EntityKey, entity: &Entity, color_fn: &impl Fn(Vec4, Material) -> Vec4) {
           if let Some(range) = self.entity_ranges.get(&key).copied() {
               // Remove old geometry
               // Note: This is simplified; real implementation needs index adjustment

               // For now, just rebuild (optimization: use stable indices)
               self.rebuild_entity(key, entity, color_fn);
           }
       }
   }
   ```

2. Alternative: Use separate buffers per entity (simpler):
   ```rust
   pub struct EntityRenderData {
       pub vertices: Vec<Vertex4D>,
       pub tetrahedra: Vec<GpuTetrahedron>,
       pub dirty: bool,
   }

   pub struct RenderableGeometry {
       entities: HashMap<EntityKey, EntityRenderData>,
   }

   impl RenderableGeometry {
       pub fn set_entity(&mut self, key: EntityKey, entity: &Entity, color_fn: &impl Fn(Vec4, Material) -> Vec4) {
           let data = self.entities.entry(key).or_insert_with(EntityRenderData::default);
           data.vertices.clear();
           data.tetrahedra.clear();
           // Fill with entity geometry
           data.dirty = true;
       }

       pub fn dirty_entities(&self) -> impl Iterator<Item = EntityKey> + '_ {
           self.entities.iter()
               .filter(|(_, data)| data.dirty)
               .map(|(key, _)| *key)
       }

       pub fn mark_clean(&mut self, key: EntityKey) {
           if let Some(data) = self.entities.get_mut(&key) {
               data.dirty = false;
           }
       }

       /// Get combined vertex/tetra buffers for GPU upload
       pub fn combined_vertices(&self) -> Vec<Vertex4D> {
           self.entities.values().flat_map(|d| d.vertices.iter().copied()).collect()
       }

       pub fn combined_tetrahedra(&self) -> Vec<GpuTetrahedron> {
           self.entities.values().flat_map(|d| d.tetrahedra.iter().copied()).collect()
       }
   }
   ```

### Wave 4: Main Integration (Sequential)

1. Update `main.rs` to use incremental updates:
   ```rust
   // In render loop:

   // Step physics (marks entities dirty)
   self.world.update(dt);

   // Check if any entity changed
   if self.world.has_dirty_entities() {
       // Update only dirty entities
       for (key, entity) in self.world.dirty_entities() {
           let color_fn = self.get_color_fn_for_entity(entity);
           self.geometry.update_entity(key, entity, &color_fn);
       }

       // Re-upload changed data to GPU
       self.slice_pipeline.upload_vertices(&self.geometry.combined_vertices());
       self.slice_pipeline.upload_tetrahedra(&self.geometry.combined_tetrahedra());

       // Clear dirty flags
       self.world.clear_all_dirty();
   }
   ```

2. Initial geometry build:
   ```rust
   fn build_initial_geometry(world: &World) -> RenderableGeometry {
       let mut geometry = RenderableGeometry::new();
       for (key, entity) in world.iter() {
           let color_fn = Self::get_color_fn_for_entity(entity);
           geometry.set_entity(key, entity, &color_fn);
       }
       geometry
   }
   ```

---

## Commits

1. "Add DirtyFlags to Entity"
2. "Track dirty entities in World"
3. "Add per-entity geometry tracking to RenderableGeometry"
4. "Use incremental geometry updates in main.rs"

---

## Verification

1. **Unit tests:**
   ```rust
   #[test]
   fn test_dirty_flag_on_position_change() {
       let mut entity = Entity::new(shape);
       assert!(!entity.is_dirty());

       entity.set_position(Vec4::new(1.0, 0.0, 0.0, 0.0));
       assert!(entity.is_dirty());
       assert!(entity.dirty_flags().contains(DirtyFlags::TRANSFORM));

       entity.clear_dirty();
       assert!(!entity.is_dirty());
   }

   #[test]
   fn test_physics_sync_marks_dirty() {
       let mut world = World::new().with_physics(PhysicsConfig::default());
       // Add entity with physics body...

       world.update(0.016);  // Physics step

       // Entity should be dirty if physics moved it
       assert!(world.has_dirty_entities());
   }
   ```

2. **Performance test:**
   - Add 100 entities
   - Move 1 entity
   - Measure time for geometry update
   - Should be much faster than rebuilding all 100

3. **Manual test:**
   - Visual behavior unchanged
   - FPS should remain high with many entities

---

## Future Optimizations

After this foundation:
- **GPU buffer sub-updates:** Upload only changed vertex ranges
- **Instancing:** Same mesh, different transforms = single draw call
- **Culling:** Skip entities far from camera
- **LOD:** Lower detail for distant objects

---

## Rollback Plan

If issues arise, fall back to full rebuild:
```rust
if self.world.has_dirty_entities() {
    self.geometry = Self::build_initial_geometry(&self.world);  // Full rebuild
    self.world.clear_all_dirty();
}
```
