# Rust4D Architecture Refactor: Long-Term Vision

**Created:** 2026-01-27
**Status:** Planning
**Goal:** Transform Rust4D from a prototype into a well-architected 4D game engine

---

## Executive Summary

This document outlines the target architecture for Rust4D based on industry best practices and Rust game engine patterns. The refactoring is divided into 7 phases that can be implemented incrementally, with each phase delivering value independently.

---

## Current Pain Points

| Issue | Impact | Phase to Fix |
|-------|--------|--------------|
| `BodyHandle(usize)` vulnerable to ABA problem | Stale handles, crashes | Phase 0 |
| Entity identified by array index (`if idx == 0`) | Breaks on reorder | Phase 1 |
| No friction in physics | Unrealistic sliding | Phase 2 |
| Hardcoded single floor plane | Can't add walls/platforms | Phase 3 |
| PlayerPhysics separate from PhysicsWorld | Two collision systems | Phase 4 |
| No collision filtering | Everything collides | Phase 5 |
| Full geometry rebuild on any change | O(n) performance | Phase 6 |
| main.rs is 500-line monolith | Untestable, rigid | Phase 7 |

---

## Target Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        GameEngine                           │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │ InputSystem │ │PhysicsSystem│ │RenderSystem │            │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘            │
│         │               │               │                   │
│         └───────────────┼───────────────┘                   │
│                         ▼                                   │
│                 ┌───────────────┐                           │
│                 │     World     │                           │
│                 │ (EntityStore) │                           │
│                 └───────┬───────┘                           │
│                         │                                   │
│         ┌───────────────┼───────────────┐                   │
│         ▼               ▼               ▼                   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐            │
│  │   Entity    │ │PhysicsWorld │ │ RenderState │            │
│  │ (SlotMap)   │ │(Colliders)  │ │(DirtyTrack) │            │
│  └─────────────┘ └─────────────┘ └─────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Architectural Decisions

### 1. Entity System: Lightweight EC (Not Full ECS)
**Rationale:** Full ECS (like Bevy) adds complexity that isn't needed for Rust4D's current scope. Fyrox proves production engines can succeed without ECS.

**Pattern:**
- Entities own their components directly (Option<T> fields)
- Generational handles via `slotmap` crate
- Named lookup via `HashMap<String, EntityKey>`
- No system scheduling or archetype storage

### 2. Physics: Rapier-Inspired Patterns
**Rationale:** Rapier is the Rust physics standard. Adopt its proven patterns.

**Patterns to adopt:**
- `PhysicsMaterial` with friction + restitution on colliders
- Collision groups via bitmasks
- Collision events collected during step()
- Static colliders in separate list (efficient)

**Not adopting (yet):**
- RigidBody/Collider separation (requires significant refactor)
- Broad phase acceleration structures (premature optimization)

### 3. Rendering: Dirty Tracking
**Rationale:** Current O(n) rebuild is wasteful. Track what changed.

**Pattern:**
- Each entity has `dirty: bool` flag
- Physics marks entity dirty when position changes
- Renderer only rebuilds dirty entities
- GPU buffer updates are incremental

### 4. Scene Management: Flat Registry + Names
**Rationale:** Scene graphs are "outdated junk" for modern games. Keep it simple.

**Pattern:**
- Flat `SlotMap<EntityKey, Entity>` storage
- `HashMap<String, EntityKey>` for named lookup
- Optional tags: `HashSet<String>` per entity
- Scene files (RON format) for serialization (future)

---

## Phase Overview

| Phase | Name | Sessions | Dependencies | Parallelizable |
|-------|------|----------|--------------|----------------|
| 0 | Foundation (Generational Handles) | 1-2 | None | No (foundational) |
| 1 | Entity Identity | 1 | Phase 0 | No |
| 2 | Physics Materials | 1 | None | **Yes** (parallel with 1) |
| 3 | Static Colliders | 1-2 | Phase 0, 2 | No |
| 4 | Player Integration | 1-2 | Phase 3 | No |
| 5 | Collision Groups | 1 | Phase 3 | **Yes** (parallel with 4) |
| 6 | Rendering Optimization | 1-2 | Phase 0, 1 | **Yes** (parallel with 3-5) |
| 7 | main.rs Decomposition | 2-3 | All above | No (final) |

**Total estimated sessions:** 9-14

---

## Parallelism Opportunities

```
Phase 0 (Foundation)
       │
       ├───────────────┬───────────────┐
       ▼               ▼               │
   Phase 1         Phase 2             │
   (Entity ID)    (Materials)          │
       │               │               │
       ▼               ▼               │
       └───────┬───────┘               │
               ▼                       ▼
           Phase 3                 Phase 6
        (Static Colliders)       (Rendering)
               │                       │
       ┌───────┴───────┐               │
       ▼               ▼               │
   Phase 4         Phase 5             │
  (Player)        (Groups)             │
       │               │               │
       └───────┬───────┴───────────────┘
               ▼
           Phase 7
        (Decomposition)
```

---

## New Dependencies

```toml
[workspace.dependencies]
slotmap = "1.0"      # Generational handles
bitflags = "2.4"     # Collision groups
```

---

## Success Criteria

After all phases complete:

1. **No array index identification** - All entity references use generational handles
2. **Friction works** - Objects slow down on surfaces
3. **Multiple floors/walls** - Static colliders are data-driven
4. **Unified physics** - Player uses same system as other entities
5. **Selective collision** - Collision groups filter interactions
6. **Incremental rendering** - Only changed entities trigger GPU updates
7. **Clean main.rs** - App struct under 100 lines, systems testable

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking changes mid-refactor | Each phase has working checkpoint |
| Performance regression | Benchmark before/after each phase |
| Scope creep | Strict phase boundaries, defer features |
| Parallel merge conflicts | Clear file ownership per phase |

---

## Individual Phase Plans

Each phase has its own detailed plan document:

- `01-foundation-handles.md` - Generational handles with slotmap
- `02-entity-identity.md` - Names, tags, lookup
- `03-physics-materials.md` - Friction and material system
- `04-static-colliders.md` - Replace hardcoded floor
- `05-player-integration.md` - Unify player with physics world
- `06-collision-groups.md` - Bitmask-based filtering
- `07-rendering-optimization.md` - Dirty tracking
- `08-main-decomposition.md` - System extraction

---

## References

- [Handles are the Better Pointers](https://floooh.github.io/2018/06/17/handles-vs-pointers.html)
- [Rapier Documentation](https://rapier.rs/docs/)
- [Fyrox Architecture](https://github.com/FyroxEngine/Fyrox/blob/master/ARCHITECTURE.md)
- [Game Programming Patterns - Component](https://gameprogrammingpatterns.com/component.html)
- Research reports in `scratchpad/reports/architecture-refactor/`
