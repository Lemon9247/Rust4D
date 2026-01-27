# Hive Mind: Phases 5, 6, 7 (Parallel Wave)

**Started:** 2026-01-27
**Status:** In Progress
**Location:** `scratchpad/reports/2026-01-27-architecture-refactor-swarm/`

## Overview

Three agents working in parallel on independent features:

| Agent | Phase | Task | Status |
|-------|-------|------|--------|
| Player Agent | Phase 5 | Player Integration | In Progress |
| Collision Agent | Phase 6 | Collision Groups | **COMPLETE** |
| Render Agent | Phase 7 | Rendering Optimization | **COMPLETE** |

## Completed Prerequisites

- Phase 1: Generational handles (slotmap) - COMPLETE
- Phase 2: Entity identity (names/tags) - COMPLETE
- Phase 3: Physics materials (friction) - COMPLETE
- Phase 4: Static colliders - COMPLETE

## File Ownership

To avoid merge conflicts, each agent owns specific files:

### Player Agent (Phase 5)
- `crates/rust4d_physics/src/player.rs` - Convert to player body wrapper
- `crates/rust4d_physics/src/world.rs` - Add player_body tracking, grounded detection
- `crates/rust4d_physics/src/body.rs` - Add BodyType enum
- `crates/rust4d_input/src/camera_controller.rs` - Adjust input handling (if needed)
- `src/main.rs` - Remove PlayerPhysics, use unified system

### Collision Agent (Phase 6)
- `crates/rust4d_physics/src/collision.rs` - Add CollisionLayer, CollisionFilter
- `crates/rust4d_physics/src/body.rs` - Add filter field (coordinate with Player Agent)
- `Cargo.toml` (workspace) - Add bitflags dependency
- `crates/rust4d_physics/Cargo.toml` - Add bitflags

### Render Agent (Phase 7)
- `crates/rust4d_core/src/entity.rs` - Add DirtyFlags
- `crates/rust4d_core/src/world.rs` - Track dirty entities
- `crates/rust4d_render/src/renderable.rs` - Per-entity geometry tracking

## Coordination Rules

1. **Shared File: body.rs**
   - Player Agent adds: BodyType enum, grounded field
   - Collision Agent adds: filter field
   - **Rule:** Player Agent commits first, Collision Agent rebases

2. **Shared File: main.rs**
   - Player Agent removes PlayerPhysics, uses unified physics
   - Collision Agent may set collision filters
   - **Rule:** Player Agent commits first, Collision Agent adds filters after

3. All agents should add their changes to the hive-mind file as they work

## Questions for Coordination

(Agents should add questions here if they need input)

## Progress Updates

### Render Agent (Phase 7) - 2026-01-27

**Status: COMPLETE**

Implemented dirty tracking system:
1. Added `DirtyFlags` bitflags to Entity (TRANSFORM, MESH, MATERIAL)
2. Added dirty tracking methods to Entity (`is_dirty`, `mark_dirty`, `clear_dirty`, `set_position`, etc.)
3. Added World-level dirty tracking (`has_dirty_entities`, `dirty_entities`, `clear_all_dirty`)
4. Updated physics sync to mark entities dirty only when position actually changes
5. Added 18 new tests (12 entity, 6 world)
6. All 237 workspace tests pass

Commits:
- `bf45205` - Add DirtyFlags to Entity for change tracking
- `03c2d80` - Track dirty entities in World with physics sync

Note: Did NOT modify renderable.rs - per-entity GPU updates can be a future enhancement. The core dirty tracking infrastructure is complete and ready for use.

