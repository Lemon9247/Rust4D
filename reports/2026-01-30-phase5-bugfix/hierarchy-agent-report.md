# Hierarchy Agent Report - Phase 5B: Entity Hierarchy

**Date**: 2026-01-30
**Agent**: Hierarchy Agent
**Task**: Phase 5B - Entity Hierarchy (parent-child transforms, cycle detection, recursive delete)
**Status**: COMPLETE - All tests passing (110/110)

## Summary

Implemented a full parent-child entity hierarchy system on the `World` struct in `world.rs`. The hierarchy is stored externally (on World, not on Entity), which avoids circular module dependencies and follows the ECS pattern where relationships are managed by the container, not the components.

No changes were made to `entity.rs` -- all hierarchy state lives on `World`.

## Design Decisions

### Hierarchy stored on World, not Entity
The task initially suggested adding `parent` and `children` fields to `Entity`. However, `Entity` is defined in `entity.rs` and `EntityKey` is defined in `world.rs`. Adding a dependency from entity -> world would create a circular module dependency. Instead, I stored hierarchy using two `HashMap` fields on `World`:
- `parents: HashMap<EntityKey, EntityKey>` (child -> parent)
- `children_map: HashMap<EntityKey, Vec<EntityKey>>` (parent -> children)

This is actually the cleaner ECS-style approach and matches how the name index already works.

### Full transform composition
Rather than just adding positions, `world_transform()` uses `Transform4D::compose()` which correctly handles position + rotation + scale accumulation. This means parent rotation/scale affects child positions correctly.

### Cycle detection via ancestor walk
`add_child()` checks for cycles by calling `is_ancestor(child, parent)` -- if the child is already an ancestor of the proposed parent, adding the relationship would create a cycle. This catches both direct cycles (A->B then B->A) and deep cycles (A->B->C then C->A).

### Reparenting support
If `add_child()` is called on a child that already has a different parent, it automatically removes the child from the old parent first. Calling `add_child()` with the same parent returns `AlreadyChild` error.

## Files Modified

### `crates/rust4d_core/src/world.rs`
- Added imports: `VecDeque`, `fmt`, `Transform4D`
- Added `HierarchyError` enum with `Display` and `Error` impls
- Added `parents` and `children_map` fields to `World` struct
- Updated `World::new()` and `World::with_capacity()` to initialize new fields
- Updated `remove_entity()` to clean up hierarchy (remove from parent, orphan children)
- Updated `clear()` to also clear hierarchy maps
- Added 11 new hierarchy methods to `impl World`
- Added 20 new hierarchy tests

### `crates/rust4d_core/src/entity.rs`
- NO CHANGES (hierarchy stored on World)

## New Public API

### `HierarchyError` enum
```rust
pub enum HierarchyError {
    InvalidEntity,
    CyclicHierarchy,
    AlreadyChild,
}
```

### New methods on `World`
- `parent_of(entity) -> Option<EntityKey>`
- `children_of(entity) -> &[EntityKey]`
- `has_children(entity) -> bool`
- `has_parent(entity) -> bool`
- `add_child(parent, child) -> Result<(), HierarchyError>`
- `remove_from_parent(child)`
- `world_transform(entity) -> Option<Transform4D>`
- `delete_recursive(entity) -> Vec<Entity>`
- `descendants(entity) -> Vec<EntityKey>`
- `root_entities() -> impl Iterator<Item = (EntityKey, &Entity)>`
- `is_ancestor(ancestor, entity) -> bool`

## Exports Needed in lib.rs

The Queen needs to add `HierarchyError` to the world re-export line:
```rust
pub use world::{World, EntityKey, HierarchyError};
```

## Tests Added (20 total)
1. `test_add_child` - Basic parent-child relationship
2. `test_add_child_invalid_entity` - Error on non-existent entity
3. `test_cycle_detection` - Direct cycle (A->B, B->A) and self-parenting
4. `test_deep_cycle_detection` - Deep cycle (A->B->C, C->A)
5. `test_already_child` - Error on duplicate add_child
6. `test_remove_from_parent` - Detach child from parent
7. `test_world_transform_no_parent` - Root entity returns own transform
8. `test_world_transform_with_parent` - Child gets composed position
9. `test_world_transform_with_scale` - Scale correctly composes
10. `test_delete_recursive` - Delete parent removes all descendants
11. `test_delete_recursive_subtree` - Delete subtree preserves siblings
12. `test_descendants` - Breadth-first descendant collection
13. `test_root_entities` - Only root entities returned
14. `test_is_ancestor` - Various ancestor checks including transitive
15. `test_remove_entity_cleans_hierarchy` - Existing remove_entity orphans children
16. `test_reparent` - Moving child from one parent to another
17. `test_hierarchy_error_display` - Display impl for error messages
18. `test_clear_cleans_hierarchy` - clear() also clears hierarchy maps
19. `test_world_transform_deep_hierarchy` - 3-level deep transform composition
20. `test_world_transform_nonexistent` - Non-existent entity returns None

## Test Results
All 110 tests pass (64 pre-existing + 46 new across the crate, 20 of which are hierarchy tests).
Full workspace builds cleanly.
