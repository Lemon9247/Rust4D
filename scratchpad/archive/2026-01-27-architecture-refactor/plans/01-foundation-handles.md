# Phase 1: Foundation - Generational Handles

**Status:** Not Started
**Sessions:** 1-2
**Dependencies:** None
**Parallelizable With:** Nothing (foundational)

---

## Goal

Replace unsafe `usize`-based handles with generational handles using `slotmap` crate. This prevents the ABA problem where a handle points to a reused slot.

---

## Problem

Current implementation:
```rust
// rust4d_physics/src/body.rs
pub struct BodyHandle(pub(crate) usize);

// If body 0 is removed and slot reused:
let handle = world.add_body(body_a);  // handle.0 = 0
world.remove_body(handle);            // Slot 0 freed
world.add_body(body_b);               // Reuses slot 0
// handle now incorrectly references body_b!
```

---

## Solution

Use `slotmap` for O(1) insert/remove/access with generation checking:

```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! {
    pub struct BodyKey;
    pub struct EntityKey;
}

pub struct PhysicsWorld {
    bodies: SlotMap<BodyKey, RigidBody4D>,
    // ...
}

pub struct World {
    entities: SlotMap<EntityKey, Entity>,
    // ...
}
```

---

## Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` (workspace) | Add `slotmap = "1.0"` to dependencies |
| `crates/rust4d_physics/Cargo.toml` | Add slotmap dependency |
| `crates/rust4d_physics/src/body.rs` | Replace `BodyHandle` with `BodyKey` |
| `crates/rust4d_physics/src/world.rs` | Use `SlotMap<BodyKey, RigidBody4D>` |
| `crates/rust4d_core/Cargo.toml` | Add slotmap dependency |
| `crates/rust4d_core/src/entity.rs` | Replace `EntityHandle` with `EntityKey` |
| `crates/rust4d_core/src/world.rs` | Use `SlotMap<EntityKey, Entity>` |
| `crates/rust4d_core/src/lib.rs` | Re-export new key types |
| `src/main.rs` | Update to use new key types |

---

## Implementation Steps

### Wave 1: Add Dependencies (Sequential)

1. Add to workspace `Cargo.toml`:
   ```toml
   [workspace.dependencies]
   slotmap = "1.0"
   ```

2. Add to `crates/rust4d_physics/Cargo.toml`:
   ```toml
   slotmap.workspace = true
   ```

3. Add to `crates/rust4d_core/Cargo.toml`:
   ```toml
   slotmap.workspace = true
   ```

### Wave 2: Physics Handles (Can parallelize with Wave 3)

**Agent A: Physics World Handles**

1. Define `BodyKey` in `body.rs`:
   ```rust
   use slotmap::new_key_type;

   new_key_type! {
       pub struct BodyKey;
   }
   ```

2. Update `PhysicsWorld` in `world.rs`:
   ```rust
   use slotmap::SlotMap;

   pub struct PhysicsWorld {
       bodies: SlotMap<BodyKey, RigidBody4D>,
       // ...
   }
   ```

3. Update all methods:
   - `add_body(&mut self, body) -> BodyKey`
   - `remove_body(&mut self, key: BodyKey) -> Option<RigidBody4D>`
   - `get_body(&self, key: BodyKey) -> Option<&RigidBody4D>`
   - `get_body_mut(&mut self, key: BodyKey) -> Option<&mut RigidBody4D>`

4. Update tests

### Wave 3: Entity Handles (Can parallelize with Wave 2)

**Agent B: Core Entity Handles**

1. Define `EntityKey` in `entity.rs`:
   ```rust
   use slotmap::new_key_type;

   new_key_type! {
       pub struct EntityKey;
   }
   ```

2. Update `Entity` struct - change `physics_body` field:
   ```rust
   pub physics_body: Option<BodyKey>,  // Was Option<BodyHandle>
   ```

3. Update `World` in `world.rs`:
   ```rust
   pub struct World {
       entities: SlotMap<EntityKey, Entity>,
       // ...
   }
   ```

4. Update all methods:
   - `add_entity(&mut self, entity) -> EntityKey`
   - `get_entity(&self, key: EntityKey) -> Option<&Entity>`
   - `get_entity_mut(&mut self, key: EntityKey) -> Option<&mut Entity>`
   - `iter()` - now iterates `SlotMap`

5. Update tests

### Wave 4: Main Integration (Sequential)

1. Update `src/main.rs`:
   - Replace `EntityHandle` with `EntityKey`
   - Replace `BodyHandle` with `BodyKey`
   - Update all entity/body access patterns

2. Run full test suite
3. Manual testing

---

## Commits

1. "Add slotmap dependency to workspace"
2. "Replace BodyHandle with generational BodyKey"
3. "Replace EntityHandle with generational EntityKey"
4. "Update main.rs for generational handles"

---

## Verification

1. **Unit tests pass:** `cargo test`
2. **No compilation warnings about unused code**
3. **Manual test:** Add/remove entities rapidly, verify no crashes
4. **Test stale handle detection:**
   ```rust
   let key = world.add_entity(entity);
   world.remove_entity(key);
   assert!(world.get_entity(key).is_none()); // Stale handle returns None
   ```

---

## Rollback Plan

If issues arise, revert to `usize` handles. The change is isolated to handle types.
