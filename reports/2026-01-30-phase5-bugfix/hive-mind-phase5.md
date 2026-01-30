# Hive Mind: Phase 5 Implementation + Movement Bug Verification

## Task Overview
Implement the full Phase 5 (Advanced Features) for Rust4D on branch `feature/phase-5-and-bugfix`. This covers:
- **5A**: Asset Management (cache, hot reload, dependency tracking)
- **5B**: Entity Hierarchy (parent-child transforms, cycle detection, recursive delete)
- **5C**: Advanced Scene Features (transitions, overlays, streaming, validation)

Also verify the movement rotation bug (W-axis using camera.ana()) is already fixed.

**Branch**: `feature/phase-5-and-bugfix`
**Base crate**: `crates/rust4d_core` (all Phase 5 work lives here)

## Key Design Constraints
- All new code goes in `crates/rust4d_core/src/`
- Must not break existing tests (currently all pass)
- Follow existing patterns: `Entity`, `World`, `Scene`, `SceneManager`
- Use RON for serialization, serde for derive
- Existing dependencies: slotmap, bitflags, serde, ron, log, rust4d_math, rust4d_physics

## Agents
1. **Asset Agent** - Implements Phase 5A: AssetCache, Asset trait, hot reload, ShapeRef::Asset variant
2. **Hierarchy Agent** - Implements Phase 5B: parent/children on Entity, World hierarchy ops, cycle detection
3. **Scene Features Agent** - Implements Phase 5C: SceneTransition, overlays, SceneLoader (async), SceneValidator

## File Ownership (NO CONFLICTS)
### Asset Agent owns:
- `crates/rust4d_core/src/asset_cache.rs` (NEW)
- `crates/rust4d_core/src/asset_error.rs` (NEW)

### Hierarchy Agent owns:
- Modifications to `crates/rust4d_core/src/entity.rs` (add parent/children fields)
- Modifications to `crates/rust4d_core/src/world.rs` (add hierarchy operations)

### Scene Features Agent owns:
- `crates/rust4d_core/src/scene_transition.rs` (NEW)
- `crates/rust4d_core/src/scene_loader.rs` (NEW)
- `crates/rust4d_core/src/scene_validator.rs` (NEW)
- Modifications to `crates/rust4d_core/src/scene_manager.rs` (add transitions, overlays, async loading)

### SHARED (Queen integrates):
- `crates/rust4d_core/src/lib.rs` - Queen adds all new module declarations and exports
- `crates/rust4d_core/Cargo.toml` - Queen adds any new dependencies

## Coordination Notes
- Each agent writes code to their owned files only
- Agents should NOT modify lib.rs or Cargo.toml -- Queen will do that
- Each agent should write comprehensive tests inline (`#[cfg(test)]`)
- Agents report what pub exports they need added to lib.rs

## Questions for Discussion
(Agents can add questions here)

## Status
- [x] Asset Agent: COMPLETE - created asset_error.rs (137 lines, 8 tests) and asset_cache.rs (816 lines, 26 tests)
- [x] Hierarchy Agent: COMPLETE - modified world.rs (+280 lines, 11 methods, 20 tests, HierarchyError enum). No changes to entity.rs.
- [x] Scene Features Agent: COMPLETE - created scene_transition.rs (~220 lines, 13 tests), scene_loader.rs (~150 lines, 6 tests), scene_validator.rs (~200 lines, 11 tests), modified scene_manager.rs (+130 lines, 12 new tests)
- [ ] Queen integration (lib.rs, Cargo.toml, cross-module wiring): Pending
- [ ] Final synthesis: Pending

## Reports Generated
- `asset-agent-report.md` - Phase 5A asset management implementation report
- `hierarchy-agent-report.md` - Phase 5B entity hierarchy implementation report
- `scene-features-agent-report.md` - Phase 5C advanced scene features implementation report

## Key Findings
- **Hierarchy Agent**: Stored hierarchy on World (not Entity) to avoid circular module deps between entity.rs and world.rs. EntityKey is defined in world.rs, so entity.rs can't reference it without a circular dependency. Storing hierarchy maps (parents, children_map) on World is the cleaner ECS-style approach anyway.
- **Hierarchy Agent**: Transform4D already has a `compose()` method that handles position+rotation+scale correctly, so `world_transform()` does full composition, not just position addition.

## Exports Needed in lib.rs

### Asset Agent (Phase 5A):
```rust
mod asset_error;
mod asset_cache;

pub use asset_error::AssetError;
pub use asset_cache::{AssetId, AssetHandle, Asset, AssetCache};
```
No new Cargo.toml dependencies needed (uses only std + log).

### Hierarchy Agent (Phase 5B):
```rust
// Update existing re-export line in lib.rs:
pub use world::{World, EntityKey, HierarchyError};
```
No new modules, no new Cargo.toml dependencies. Only modified world.rs.

### Scene Features Agent (Phase 5C):
```rust
mod scene_transition;
mod scene_loader;
mod scene_validator;

pub use scene_transition::{SceneTransition, TransitionEffect, SlideDirection};
pub use scene_loader::{SceneLoader, LoadResult};
pub use scene_validator::{SceneValidator, ValidationError};
```
No new Cargo.toml dependencies needed (uses only std::sync::mpsc, std::thread, std::time, std::collections::HashSet).
