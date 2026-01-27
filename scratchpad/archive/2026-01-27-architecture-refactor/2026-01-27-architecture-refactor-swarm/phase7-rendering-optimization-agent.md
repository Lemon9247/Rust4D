# Phase 7: Rendering Optimization Agent Report

**Agent:** Rendering Optimization Agent
**Date:** 2026-01-27
**Task:** Add dirty tracking to avoid rebuilding all geometry when one entity moves

## Summary

Implemented a comprehensive dirty tracking system for entities and worlds. This allows the rendering system to detect which entities have changed and need their geometry rebuilt, rather than rebuilding everything on every frame.

## Changes Made

### 1. Entity Dirty Flags (entity.rs)

Added `DirtyFlags` using the `bitflags` crate:

```rust
bitflags! {
    pub struct DirtyFlags: u8 {
        const NONE = 0;
        const TRANSFORM = 1 << 0;
        const MESH = 1 << 1;
        const MATERIAL = 1 << 2;
        const ALL = Self::TRANSFORM.bits() | Self::MESH.bits() | Self::MATERIAL.bits();
    }
}
```

Added methods to Entity:
- `is_dirty()` - Check if any flags are set
- `dirty_flags()` - Get current flags
- `mark_dirty(flags)` - Add flags (combines with existing)
- `clear_dirty()` - Reset all flags
- `set_position(pos)` - Update position and mark TRANSFORM dirty
- `set_transform(transform)` - Update transform and mark TRANSFORM dirty
- `set_material(material)` - Update material and mark MATERIAL dirty

New entities are created with `DirtyFlags::ALL` so they're included in initial geometry builds.

### 2. World Dirty Tracking (world.rs)

Added World-level dirty tracking methods:

```rust
impl World {
    /// Check if any entity needs rebuild
    pub fn has_dirty_entities(&self) -> bool;

    /// Iterate over dirty entities
    pub fn dirty_entities(&self) -> impl Iterator<Item = (EntityKey, &Entity)>;

    /// Iterate over dirty entities mutably
    pub fn dirty_entities_mut(&mut self) -> impl Iterator<Item = (EntityKey, &mut Entity)>;

    /// Clear all dirty flags
    pub fn clear_all_dirty(&mut self);
}
```

### 3. Physics Sync Dirty Tracking

Updated `World::update()` to mark entities dirty only when physics actually changes their position:

```rust
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
```

This optimization means stationary physics bodies won't trigger unnecessary geometry rebuilds.

### 4. Exports

Added `DirtyFlags` to public exports in `lib.rs`:
```rust
pub use entity::{Material, Entity, ShapeRef, DirtyFlags};
```

## Tests Added

### Entity Tests (12 new tests)
- `test_dirty_flags_default` - Default is NONE/empty
- `test_dirty_flags_all` - ALL contains all three flags
- `test_dirty_flags_combine` - Bitwise OR works correctly
- `test_new_entity_is_dirty` - New entities have ALL flags
- `test_entity_clear_dirty` - clear_dirty resets to NONE
- `test_entity_mark_dirty` - mark_dirty adds flags
- `test_set_position_marks_dirty` - Position changes trigger TRANSFORM
- `test_set_transform_marks_dirty` - Transform changes trigger TRANSFORM
- `test_set_material_marks_dirty` - Material changes trigger MATERIAL
- `test_mark_dirty_combines_flags` - Multiple marks accumulate

### World Tests (6 new tests)
- `test_new_entities_are_dirty` - Added entities are dirty
- `test_clear_all_dirty` - Bulk clear works
- `test_dirty_entities_iterator` - Only dirty entities returned
- `test_dirty_entities_mut` - Mutable iteration works
- `test_physics_sync_marks_dirty` - Moving bodies mark dirty
- `test_physics_sync_no_change_not_dirty` - Stationary bodies don't

## Commits

1. `bf45205` - Add DirtyFlags to Entity for change tracking
2. `03c2d80` - Track dirty entities in World with physics sync

## Usage Example

The main.rs can now use dirty tracking like this:

```rust
// In render loop
if self.world.has_dirty_entities() {
    // Full rebuild for now (per-entity updates can come later)
    self.geometry = Self::build_geometry(&self.world);
    // Re-upload to GPU...
    self.world.clear_all_dirty();
}
```

Or for more granular updates:
```rust
for (key, entity) in self.world.dirty_entities() {
    // Update just this entity's GPU buffer region
    update_entity_geometry(key, entity);
}
self.world.clear_all_dirty();
```

## Future Improvements

1. **Per-entity GPU buffer updates** - Instead of rebuilding all geometry, update just the dirty entities' vertex regions
2. **Dirty mesh tracking** - Add `set_shape()` method that marks MESH dirty
3. **GPU-side transform updates** - For TRANSFORM-only changes, could update transform uniform instead of rebuilding vertices
4. **Spatial partitioning dirty tracking** - Combine with spatial hash to update only visible dirty entities

## Notes

- The `bitflags` dependency was already added to the workspace by the Collision Agent for CollisionLayer, so coordination was smooth
- Dirty tracking is separate from physics collision filtering (Phase 6) - no conflicts
- All 237 workspace tests pass
