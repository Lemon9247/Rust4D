# A2: Core & Scene Review - rust4d_core

**Agent**: A2 (Core & Scene Reviewer)
**Date**: 2026-01-30
**Crate**: `rust4d_core`
**Total Lines**: 6,142 (12 `.rs` source files + 1 integration test file)
**Total Tests**: 202 (189 unit + 13 integration)

---

## 1. Entity & World System

### Entity Struct (`entity.rs`, 518 lines, 18 tests)

**Fields** (lines 121-136):
- `name: Option<String>` - optional, for lookup by name
- `tags: HashSet<String>` - categorization (e.g., "dynamic", "static", "enemy")
- `transform: Transform4D` - position, rotation, scale in 4D
- `shape: ShapeRef` - either `Shared(Arc<dyn ConvexShape4D>)` or `Owned(Box<dyn ConvexShape4D>)`
- `material: Material` - base_color `[f32; 4]` only
- `physics_body: Option<BodyKey>` - optional link to physics simulation
- `dirty: DirtyFlags` (private) - bitflags for change tracking (TRANSFORM, MESH, MATERIAL)

**Construction**: Builder pattern with `new()`, `with_material()`, `with_transform()`, `with_name()`, `with_tag()`, `with_tags()`, `with_physics_body()`. New entities always start as `DirtyFlags::ALL`.

**DirtyFlags** (lines 14-32): Uses `bitflags!` macro with NONE=0, TRANSFORM=1, MESH=2, MATERIAL=4, ALL=7. Setters like `set_position()`, `set_transform()`, and `set_material()` automatically mark the appropriate dirty flags.

**ShapeRef** (lines 84-109): Enum with `Shared(Arc<dyn ConvexShape4D>)` and `Owned(Box<dyn ConvexShape4D>)`. Smart design for memory efficiency -- many entities can share the same shape.

**EntityTemplate** (lines 263-318): Serializable mirror of Entity using `ShapeTemplate` enum instead of trait objects. `to_entity()` method converts back to a live Entity. Tags stored as `Vec<String>` (vs `HashSet` in Entity).

**Assessment**: Entity is reasonably well-structured for a prototype stage. The `HashSet<String>` for tags works but is allocation-heavy -- an enum-based tag system or bitflags would be more performant for an FPS. The name is `Option<String>` which is fine. Entity is NOT getting bloated yet at 7 fields, though the lack of component extensibility (no ECS) will become a bottleneck.

### World (`world.rs`, 1391 lines, 47 tests)

**Storage**: `SlotMap<EntityKey, Entity>` with generational indexing (line 52). This gives O(1) access and prevents ABA problems when entities are removed/reused.

**Indexes**:
- `name_index: HashMap<String, EntityKey>` - fast name lookup
- `parents: HashMap<EntityKey, EntityKey>` - child -> parent mapping
- `children_map: HashMap<EntityKey, Vec<EntityKey>>` - parent -> children mapping

**Physics integration**: `physics_world: Option<PhysicsWorld>` (line 56). The `update(dt)` method (lines 219-239) steps physics, then syncs entity transforms from physics bodies. Smart optimization: only marks dirty if position actually changed (line 231).

**Queries**:
- `get_entity(key)` / `get_entity_mut(key)` - O(1) by key
- `get_by_name(name)` / `get_by_name_mut(name)` - O(1) via name index
- `get_by_tag(tag)` - O(n) linear scan over all entities
- `dirty_entities()` / `dirty_entities_mut()` - O(n) filter
- `root_entities()` - O(n) filter
- `entity_keys()`, `iter()`, `iter_mut()`, `iter_with_keys()` - full iteration

**Entity removal** (lines 129-165): Proper cleanup of name index, physics body, and hierarchy. Children are orphaned (made root entities) when parent is removed.

**Assessment**: The World is well-implemented for its current scope. The `get_by_tag()` being O(n) will be a problem in larger worlds -- a tag index (similar to name_index) would help. No spatial queries exist (no spatial partitioning, no frustum culling, no range queries).

### Hierarchy (`world.rs`, lines 288-515)

**Parent-child**: Dual maps (`parents` and `children_map`) allow bidirectional traversal. `add_child()` handles reparenting automatically -- removes from old parent if already parented.

**Cycle detection** (lines 339-342, 506-515): `is_ancestor()` walks up from child to root. Called before `add_child()` to prevent cycles. Self-parenting is explicitly caught (line 330). This is correct but O(depth) per add_child.

**Transform accumulation** (`world_transform()`, lines 386-410): Builds chain from leaf to root, then composes from root to leaf. Uses `Transform4D::compose()` which correctly handles position + rotation + scale. The composition order is mathematically correct: root-first composition means parent transform is applied first.

**Recursive operations**:
- `delete_recursive()` (lines 417-466): BFS to collect all descendants, then removes each. Cleans up name index, physics bodies, and hierarchy maps.
- `descendants()` (lines 472-493): BFS traversal, returns `Vec<EntityKey>`.

**Assessment**: The hierarchy system is complete and correct for its use case. Cycle detection, reparenting, transform accumulation, and recursive deletion all work. Missing: cached world transforms (recomputed each call), depth limits, hierarchy events/callbacks.

### Transform4D (`transform.rs`, 261 lines, 9 tests)

**Fields**: `position: Vec4`, `rotation: Rotor4` (serialized as 8-float array via custom serde), `scale: f32` (uniform only).

**Key operations**: `transform_point()`, `transform_direction()`, `inverse()`, `compose()`, `translate()`, `rotate()`. The transform applies scale -> rotate -> translate (correct SRT order, line 89).

**Custom serde** (lines 12-30): `rotor4_serde` module serializes `Rotor4` as `[f32; 8]` since the math crate doesn't derive Serialize/Deserialize. Pragmatic workaround.

**Assessment**: Solid and mathematically correct. The inverse computation handles near-zero scale (line 111). Only uniform scale is supported -- non-uniform scale would be needed for stretched shapes but uniform is fine for a start.

---

## 2. Scene Management

### Scene Serialization (`scene.rs`, 611 lines, 16 tests)

**Scene struct** (lines 22-33): `name: String`, `entities: Vec<EntityTemplate>`, `gravity: Option<f32>`, `player_spawn: Option<[f32; 4]>`. All fields serde-compatible.

**RON format**: Load/save to RON files with `ron::ser::PrettyConfig` for human-readable output. Tests verify round-trip serialization including the exact file format with struct names.

**ActiveScene** (lines 211-337): Runtime wrapper holding a `World` instance. `from_template()` (lines 227-310) is the key method that:
1. Creates World with physics config (from template or override)
2. Iterates entity templates, creating entities
3. For "static" tagged hyperplanes: creates `StaticCollider::floor_bounded()`
4. For "dynamic" tagged entities: creates `RigidBody4D::new_aabb()` with BodyType::Dynamic, mass 10.0, PhysicsMaterial::WOOD
5. Creates player kinematic body from spawn position

**Error types**: `SceneLoadError` (Io/Parse), `SceneSaveError` (Io/Serialize), `SceneError` (unified: Io/Parse/Serialize/NotLoaded/NoActiveScene). Proper From conversions between them.

**Assessment**: Scene loading is functional. The tag-based physics assignment in `from_template()` (lines 245-287) is hardcoded logic -- "static" creates colliders, "dynamic" creates rigid bodies. This works for the demo but lacks flexibility. No support for custom physics properties per entity (friction, restitution are hardcoded to WOOD/CONCRETE).

### ShapeTemplate (`shapes.rs`, 129 lines, 4 tests)

**Variants**: Only two shapes supported:
- `Tesseract { size: f32 }` - 4D hypercube
- `Hyperplane { y, size, subdivisions, cell_size, thickness }` - floor/ground

Uses `#[serde(tag = "type")]` for tagged enum serialization. `create_shape()` returns `Box<dyn ConvexShape4D>`.

**Assessment**: Very limited shape vocabulary. A boomer shooter would need spheres, cylinders, custom convex hulls, sector/portal geometry, etc.

### SceneManager (`scene_manager.rs`, 786 lines, 28 tests)

**Features**:
- Template management: `load_scene(path)`, `register_template()`, `get_template()`
- Scene instantiation: `instantiate(template_name)` creates ActiveScene from template
- **Scene stack**: `push_scene()`, `pop_scene()`, `switch_to()` -- supports overlays like pause menus
- **Transitions**: `switch_to_with_transition()` with TransitionEffect enum
- **Overlay stack**: Separate `overlay_stack` for HUD/minimap layers
- **Async loading**: `load_scene_async()` + `poll_loading()` via embedded SceneLoader
- Active scene access: `active_scene()`, `active_world()`, `active_world_mut()`

**Transition integration**: `update_transition()` advances the transition and auto-switches when complete.

**Assessment**: SceneManager is feature-rich for its stage. Scene stack, overlays, async loading, and transitions are all good features for a game. The overlay stack and main stack are independent, which is correct for HUD-over-gameplay patterns.

### Scene Transitions (`scene_transition.rs`, 384 lines, 12 tests)

**Effects**:
- `Instant` - immediate cut (Duration::ZERO)
- `Fade { duration }` - fade to black then fade in (alpha: 1->0->1)
- `Crossfade { duration }` - blend between scenes (alpha: 0->1)
- `Slide { duration, direction }` - Left/Right/Up/Down

**SceneTransition struct**: Tracks `start_time: Instant`, `progress: f32`, source/dest scene names. `update()` computes progress from elapsed time. `alpha()` provides per-effect rendering information.

**Assessment**: Well-designed with proper time-based progression. The alpha calculations for fade/crossfade are correct. Slide provides direction but no offset calculation -- the renderer would need to compute that from progress + direction.

### Scene Loader (`scene_loader.rs`, 205 lines, 7 tests)

**Architecture**: Spawns a background worker thread using `std::sync::mpsc` channels. `load_async()` sends `LoadRequest`, worker calls `Scene::load()`, sends `LoadResult` back. `poll()` and `poll_all()` non-blocking receive.

**Assessment**: Clean and functional async loading. Single worker thread handles requests sequentially. For a game with many levels, a thread pool would be better, but single thread is fine for now.

### Scene Validator (`scene_validator.rs`, 324 lines, 12 tests)

**Checks**:
1. Empty scene (no entities)
2. Duplicate entity names
3. Unreasonable gravity (|g| > 1000)
4. Extreme spawn position (any component |x| > 10000)

**Not checked**: Shape validity, negative scale, NaN/Inf values, transform bounds, tag consistency, physics configuration validity.

**Assessment**: Basic sanity checks. Good foundation but could be expanded significantly.

---

## 3. Asset System

### AssetCache (`asset_cache.rs`, 816 lines, 28 tests)

**Type-erased storage**: `HashMap<AssetId, CachedEntry>` where CachedEntry holds `Arc<dyn Any + Send + Sync>`. Assets downcast via `Arc::downcast::<T>()`.

**Deduplication**: `path_index: HashMap<PathBuf, AssetId>` prevents loading the same file twice.

**Dependency tracking**: `dependents: Vec<String>` per asset entry. `add_dependent()` / `remove_dependent()` track which scenes use each asset.

**Garbage collection**: `gc()` removes assets with empty `dependents` list. Returns count of removed assets.

**Hot reload**: `check_hot_reload::<T>()` compares file modification timestamps against `load_time`. If changed, calls `T::load_from_file()` again and swaps the Arc data. Gated by `watch_for_changes` flag.

**Asset trait** (lines 63-74): `trait Asset: Sized + Send + Sync + 'static` with one required method: `fn load_from_file(path: &Path) -> Result<Self, AssetError>`.

**Assessment**: Well-architected asset system with solid features. The type-erasure approach is clean. Hot reload uses timestamp comparison which is simple but reliable. Dependency tracking and GC are thoughtful additions. The `check_hot_reload::<T>()` requiring a type parameter means callers must call it once per asset type, which is a minor ergonomic issue.

### AssetError (`asset_error.rs`, 137 lines, 8 tests)

Three variants: `Io(io::Error)`, `Parse(String)`, `NotFound(String)`. Implements `Display`, `Error`, `From<io::Error>`, `From<String>`, `From<&str>`.

---

## 4. Testing Analysis

### Test Count by Module

| Module | Tests | Lines |
|--------|-------|-------|
| entity.rs | 18 | 518 |
| world.rs | 47 | 1391 |
| transform.rs | 9 | 261 |
| shapes.rs | 4 | 129 |
| scene.rs | 16 | 611 |
| scene_manager.rs | 28 | 786 |
| scene_transition.rs | 12 | 384 |
| scene_loader.rs | 7 | 205 |
| scene_validator.rs | 12 | 324 |
| asset_cache.rs | 28 | 816 |
| asset_error.rs | 8 | 137 |
| **physics_integration.rs** | **13** | **536** |
| **Total** | **202** | **6142** |

### Quality Assessment

**Strengths**:
- Tests are meaningful and test actual behavior, not just "doesn't panic"
- Dirty flag tests verify exact flag combinations
- Hierarchy tests cover cycles, reparenting, deep hierarchies, recursive deletion
- Physics integration tests verify the full pipeline (scene->physics->entity sync)
- Asset cache tests use real temp files and verify hot reload, GC, dedup
- Error display tests ensure user-facing messages are correct

**Weaknesses**:
- Scene loader tests rely on `thread::sleep()` which can be flaky in CI
- No stress tests or benchmarks
- No tests for concurrent access patterns
- Transform tests don't test 4D-specific rotation combinations (e.g., XW plane)
- No serialization round-trip tests for Transform4D with non-identity rotation

### What's NOT Tested
- `Scene::load()` and `Scene::save()` with actual file I/O (only round-trip via ron strings)
- `SceneManager::load_scene()` (requires file system)
- World behavior under high entity counts
- Name collision during `add_entity()` (duplicate names silently overwrite the index)
- Edge cases in `world_transform()` with broken parent chains
- Multiple SceneLoader workers handling many concurrent requests under load

---

## 5. Boomer Shooter Gaps

### Critical Missing Systems

1. **Level/Map Loading**: Only Tesseract and Hyperplane shapes exist. No BSP, sector/portal, or mesh-based level geometry. No way to define rooms, corridors, or complex 4D level layouts. This is the single biggest gap.

2. **Spawning System**: No concept of spawn points for enemies, items, or projectiles. The `player_spawn` in Scene is a single static position. No wave spawning, trigger-based spawning, or item respawn logic.

3. **Inventory/Weapon System**: Entity has no component for holding items, weapons, health, ammo, or any gameplay state. The Material is base_color only -- no weapon models, animations, or state machines.

4. **Entity Pooling**: No object pooling for projectiles. Creating/destroying entities via SlotMap `insert`/`remove` works but generates garbage. For a bullet-heavy shooter, a pool with pre-allocated slots would be better.

5. **Tags/Layers for Collision Filtering**: Entity tags are free-form strings. No collision layer/mask system. Physics doesn't seem to have collision groups. Without this, bullets hit everything including the player and walls indiscriminately.

6. **Save/Load Game State**: Scene serialization handles level templates but there's no system for saving runtime game state (player inventory, enemy positions, door states, etc.). World cannot serialize back to Scene.

7. **HUD/UI State**: The overlay system in SceneManager is there, but there's no UI widget system, health bar, ammo counter, crosshair, or damage indicator.

8. **AI / Navigation**: No pathfinding, behavior trees, or AI state. No navmesh or waypoint system.

9. **Audio Integration**: No audio system hooks at all.

10. **Event System**: No entity events (on_damage, on_death, on_pickup, on_trigger). No pub/sub or observer pattern. This makes gameplay scripting very difficult.

### Moderate Gaps

11. **Entity Components**: The monolithic Entity struct has no extensibility mechanism. Adding health, AI state, weapon data, etc. requires modifying the Entity struct directly. An ECS or component-attachment system would be far more maintainable.

12. **Spatial Queries**: No raycasting through the world, no proximity queries, no trigger volumes. Essential for shooting, item pickup, damage zones.

13. **Entity Prefabs/Archetypes**: EntityTemplate is close to a prefab system but there's no way to define archetype hierarchies (e.g., "all enemies have health + AI + collider").

---

## 6. Overall Assessment

### Ratings (1-5)

| Category | Rating | Justification |
|----------|--------|---------------|
| Feature Completeness | 3/5 | Scene management, hierarchy, asset cache, and physics integration are solid. Missing gameplay systems. |
| Code Quality | 4/5 | Clean Rust, good error handling, proper ownership patterns, thorough documentation. Builder patterns are idiomatic. |
| Test Coverage | 4/5 | 202 tests across the crate is excellent. Most edge cases covered. Some filesystem-dependent gaps. |
| FPS Readiness | 2/5 | Core infrastructure is there but gameplay systems (weapons, spawning, AI, spatial queries, event system) are entirely absent. |

### Top 3 Strengths

1. **Robust World + Hierarchy system**: Generational keys via SlotMap, cycle detection, transform accumulation, recursive deletion, and proper cleanup on entity removal. The hierarchy is production-quality code.

2. **Scene management pipeline**: Templates -> instantiation -> runtime world, with async loading, transitions, overlays, and validation. This is a well-thought-out game scene architecture.

3. **Asset cache with hot reload and GC**: Type-erased storage, path deduplication, dependency tracking, garbage collection, and file change detection. A mature asset system for a young engine.

### Top 3 Gaps

1. **No ECS or component extensibility**: Entity is a fixed struct. Adding FPS gameplay features (health, weapons, AI, inventory) requires modifying Entity directly. This will not scale.

2. **No spatial queries or level geometry**: No raycasting, no BSP/portal, no trigger volumes, no proximity queries. Cannot shoot things, pick up items, or build non-trivial levels.

3. **No event/messaging system**: No way for entities to communicate (damage events, pickup events, death events). Gameplay scripting has no foundation.

---

## File Reference Summary

| File | Path | Lines |
|------|------|-------|
| lib.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/lib.rs` | 44 |
| entity.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/entity.rs` | 518 |
| world.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/world.rs` | 1391 |
| transform.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/transform.rs` | 261 |
| shapes.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/shapes.rs` | 129 |
| scene.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene.rs` | 611 |
| scene_manager.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene_manager.rs` | 786 |
| scene_transition.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene_transition.rs` | 384 |
| scene_loader.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene_loader.rs` | 205 |
| scene_validator.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/scene_validator.rs` | 324 |
| asset_cache.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/asset_cache.rs` | 816 |
| asset_error.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/asset_error.rs` | 137 |
| physics_integration.rs | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/tests/physics_integration.rs` | 536 |
| Cargo.toml | `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/Cargo.toml` | 15 |
