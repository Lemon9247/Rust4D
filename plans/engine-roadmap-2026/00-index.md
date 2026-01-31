# Rust4D Engine Roadmap 2026

**Created:** 2026-01-27
**Updated:** 2026-01-31
**Status:** Active Development -- Phases 1-5 complete, preparing for engine/game split

---

## Overview

This folder contains the complete implementation roadmap for the Rust4D engine. The project is a 4D game engine written in Rust, targeting a 4D boomer shooter ("Rust4D-Shooter") as its first game. The engine will be split into a generic library and a separate game repository.

The roadmap has three stages:
1. **Completed original phases (1-5)** -- Foundation, scene management, documentation, architecture, advanced features
2. **Engine/game split** -- ECS migration, crate restructuring, game repo creation
3. **Post-split feature development** -- Combat, audio, enemies, level tools, editor

---

## Part I: Completed Work (Original Phases 1-5)

These phases are **COMPLETE**. They established the engine's foundation: 4D math (Rotor4 geometric algebra), GPU hyperplane slicing, physics, scene management, asset caching, and entity hierarchy.

| Phase | Plan | Sessions | Status |
|-------|------|----------|--------|
| 1 | [Foundation](./phase-1-foundation.md) | 4 | **COMPLETE** |
| 2 | [Scene Management](./phase-2-scene-management.md) | 3 | **COMPLETE** (via EntityTemplate) |
| 3 | [Documentation](./phase-3-documentation.md) | 4-5 | **COMPLETE** |
| 4 | [Architecture](./phase-4-architecture.md) | 4 | **COMPLETE** |
| 5 | [Advanced Features](./phase-5-advanced-features.md) | 5-6 | **COMPLETE** |

**Total completed: ~20-22 sessions**

### Completion Notes

- **Phase 1A (Scene Serialization)**: COMPLETE -- RON-based scene files working
- **Phase 1B (Configuration System)**: COMPLETE -- TOML config with env var overrides
- **Phase 2A (Scene Manager)**: COMPLETE -- Basic scene management implemented
- **Phase 2B (Prefab System)**: COMPLETE -- Implemented as `EntityTemplate` (simpler approach than planned Prefab struct; override support was descoped)
- **Phase 3A (Examples + README)**: COMPLETE -- 4 examples, enhanced README, ARCHITECTURE.md
- **Phase 3B (Comprehensive Guides)**: COMPLETE -- Getting Started, User Guide, Developer Guide all created
- **Phase 4A (System Extraction)**: COMPLETE -- main.rs refactored from 588 to 328 lines, 4 systems extracted
- **Phase 5A (Asset Management)**: COMPLETE -- AssetCache with hot reload, dedup, dependency tracking, GC
- **Phase 5B (Entity Hierarchy)**: COMPLETE -- Parent-child on World with cycle detection, transform accumulation
- **Phase 5C (Advanced Scene Features)**: COMPLETE -- Transitions, overlays, async loading, validation

---

## Part II: Engine/Game Split

The engine will be split into two repositories: a generic **Rust4D Engine** library and a **Rust4D-Shooter** game repo. This is the critical path that gates all gameplay feature development.

### Stage 1: Pre-Split Foundation (~1 session)

**Plan:** [split-phase-0-foundation.md](./split-phase-0-foundation.md)

Five independent tasks that must be completed BEFORE the ECS migration begins:

| Task | Priority | Estimate |
|------|----------|----------|
| Rotor4 Serialization (BLOCKING) | A | 0.25 session |
| Physics Type Serialization Audit | B (deferred) | -- |
| Fixed Timestep for Physics | A | 0.5 session |
| Diagonal Movement Normalization | B | 0.1 session |
| Re-enable Back-Face Culling | B | 0.1 session |

All tasks are independent and can be parallelized, though the total is small enough for a single session.

### Stage 2: Engine/Game Split (9.5-14 sessions)

**Summary:** [split-phases-1-5.md](./split-phases-1-5.md)
**Full plan:** `scratchpad/plans/2026-01-30-engine-game-split.md`

| Phase | Name | Sessions | Depends On | Parallel? |
|-------|------|----------|------------|-----------|
| 1 | ECS Migration (hecs) | 4-6 | Phase 0 Foundation | No (sequential) |
| 2 | Game Logic Extraction + rust4d_game | 3-4 | Phase 1 | No (sequential) |
| 3 | Pluggable Scene Instantiation | 1 | Phase 2 | No (sequential) |
| 4 | Create Game Repository | 1-2 | Phases 1-3 | Yes (with Phase 5) |
| 5 | Engine Cleanup | 0.5-1 | Phase 3 | Yes (with Phase 4) |

**Execution order:**
```
Phase 0 (Foundation, 1 session)
  -> Phase 1 (ECS Migration, 4-6 sessions)
    -> Phase 2 (Game Logic Extraction, 3-4 sessions)
      -> Phase 3 (Pluggable Scenes, 1 session)
        -> Phase 4 (Game Repo) + Phase 5 (Cleanup)  [parallel]
```

**Key decisions:**
- Full hecs ECS migration (not partial ComponentStore approach)
- New `rust4d_game` crate for CharacterController4D, events, scene helpers
- Git URL hybrid dependency model for the game repo
- Input refactored to action/axis abstraction (InputAction, InputMap)

**Target architecture after split:**
```
Rust4D/                              # Engine repo (library-only)
  crates/
    rust4d_math                      # Pure 4D math (Vec4, Rotor4, shapes)
    rust4d_physics                   # Generic 4D physics
    rust4d_core                      # ECS via hecs, scenes, assets
    rust4d_game                      # NEW: CharacterController4D, events, FSM
    rust4d_render                    # GPU rendering pipeline
    rust4d_input                     # Action/axis input abstraction

Rust4D-Shooter/                      # Game repo (new)
  src/                               # Game entry point, systems, input
  scenes/                            # Game scenes (RON)
  config/                            # Game config (TOML)
```

---

## Part III: Post-Split Engine Features

After the split is complete, these phases add the engine capabilities needed for the boomer shooter and any future 4D game. Total engine estimate: **24.75-28.25 sessions** (critical path much shorter with parallelism).

| Phase | Plan | Sessions | Priority | Status | Key Dependencies |
|-------|------|----------|----------|--------|------------------|
| P1: Combat Core | [post-split-phase-1-combat-core.md](./post-split-phase-1-combat-core.md) | 1.75 | P0 | Planned | Split Phase 2 (generic physics API) |
| P2: Weapons & Feedback | [post-split-phase-2-weapons-feedback.md](./post-split-phase-2-weapons-feedback.md) | 4.5-5.5 | P1 | Planned | Foundation (fixed timestep), P1 (raycasting) |
| P3: Enemies & AI | [post-split-phase-3-enemies-ai.md](./post-split-phase-3-enemies-ai.md) | 4.0 | P1 | Planned | Foundation, P1 (raycasting for LOS) |
| P4: Level Design | [post-split-phase-4-level-design.md](./post-split-phase-4-level-design.md) | 4.5 | P1 | Planned | Foundation (serialization), P1 (trigger events) |
| P5: Editor & Polish | [post-split-phase-5-editor-polish.md](./post-split-phase-5-editor-polish.md) | 10-12.5 | P2 | Planned | All prior phases |

### Post-Split Phase Summaries

**P1: Combat Core** -- Raycasting (`Ray4D` in math, ray-shape intersections, world raycast), collision event reporting (`CollisionEvent`, `drain_collision_events()`), trigger system bug fix (asymmetric detection), trigger enter/exit/stay tracking. The engine provides collision/trigger DATA; the game builds health, damage, and weapons on top.

**P2: Weapons & Feedback** -- New `rust4d_audio` crate wrapping kira with 4D spatial audio and W-distance filtering. egui-wgpu integration in `rust4d_render` via `OverlayRenderer` for HUD. CPU-simulated, GPU-rendered billboard particle system. `ScreenShake` and `TimedEffect` helpers in `rust4d_game`. Depth texture exposure for shared depth buffer.

**P3: Enemies & AI** -- Sprite/billboard rendering pipeline with 4D W-distance fade. `SpriteAnimation` for frame-based animation. Spatial query API (`query_sphere`, `query_area_effect` for hyperspherical explosions). Generic `StateMachine<S>` in `rust4d_game`. `apply_impulse` for knockback. `line_of_sight` wrapping raycasting.

**P4: Level Design Pipeline** -- `Hyperprism4D` (rectangular hyperprism with independent X/Y/Z/W dimensions) and `Hypersphere4D` (renderable 4D sphere via icosphere subdivision). RON preview tool with hot-reload for level design iteration. `Tween<T>` system with easing functions for smooth property animation. Declarative trigger system (`TriggerDef` in RON with `GameEvent(String)` escape hatch).

**P5: Editor & Polish** -- Triplanar texture mapping (no compute shader changes needed). `PointLight4D` with W-distance attenuation. Directional shadow mapping. `InputMap::rebind()` with TOML persistence. New `rust4d_editor` crate with `EditorHost` trait, entity list, property inspector, W-slice navigation thumbnails, and scene save/load.

### Game Repo Roadmap

**Plan:** [game-roadmap-summary.md](./game-roadmap-summary.md)
**Estimate:** 13-21 sessions (runs in parallel with engine post-split phases)

The game repo builds all boomer-shooter-specific systems on top of engine APIs:

| Game Phase | Sessions | Engine Prerequisite |
|------------|----------|---------------------|
| Phase 0: Game Repo Setup | 1-2 | Engine split complete |
| Phase 1: Combat Core | 2-3 | Engine P1 |
| Phase 2: Weapons & Feedback | 3-4 | Engine P2 |
| Phase 3: Enemies | 3-4 | Engine P3 |
| Phase 4: Level Design | 2-4 | Engine P4 |
| Phase 5: Polish | 2-4 | Engine P5 |

Game work starts after the split is complete (Split Phase 4) and each game phase begins as the corresponding engine phase delivers its APIs.

---

## Full Dependency Graph

```
COMPLETED (Phases 1-5)
  |
  v
STAGE 1: PRE-SPLIT FOUNDATION (~1 session)
  [Rotor4 serde] [Fixed timestep] [Diagonal norm] [Back-face cull]
  |
  v
STAGE 2: ENGINE/GAME SPLIT (9.5-14 sessions, sequential)
  Split P1: ECS Migration (4-6)
    -> Split P2: Game Logic Extraction (3-4)
      -> Split P3: Pluggable Scenes (1)
        -> Split P4: Create Game Repo (1-2) + Split P5: Cleanup (0.5-1)
                                                |
  ==============================================|============================
  POST-SPLIT FEATURE DEVELOPMENT                |
  ==============================================|============================
                                                |
                              Game Phase 0 starts here
                                                |
  P4-Wave1 --+                                  |
  (shapes,   |                                  v
  zero deps) |   P1-A: Raycasting --------+   Game P0: Repo Setup
             |   P1-B: Collision Events ---+
             |              |               \
             |              v                \
             |   P2-A: Audio (kira) ----+     +---> Game P1: Combat Core
             |   P2-B: HUD (egui) -----+     |
             |   P2-C: Particles ------+     |
             |   P2-D: Screen Effects -+     |
             |              |                 |
             |              v                 v
             |   P3-A: Sprites ---------+   Game P2: Weapons & Feedback
             |   P3-B: Spatial Queries -+
             |   P3-C: FSM (tiny) -----+
             |              |                 |
             |              v                 v
             +-> P4-B: RON Preview ----+    Game P3: Enemies
                 P4-C: Tweens ---------+
                 P4-D: Triggers -------+
                            |                 |
                            v                 v
                 P5-A: Textures -------+    Game P4: Level Design
                 P5-B: Lighting -------+
                 P5-C: Input Rebind ---+
                 P5-D: Editor (largest)+
                            |                 |
                            v                 v
                      Engine Complete       Game P5: Polish
```

### Key Dependency Relationships

- **P1 (Combat Core)** is the first post-split phase and has no engine-internal dependencies beyond the split itself.
- **P2 (Weapons)** depends on P1 for raycasting (hitscan weapons) and Foundation for fixed timestep (particle simulation).
- **P3 (Enemies)** depends on P1 for raycasting (line-of-sight) and P2 for the particle system (blood, explosions).
- **P4 Wave 1 (shape types)** has ZERO dependencies and can start during or before the split.
- **P4 remaining waves** depend on Foundation (serialization) and P1 (trigger events).
- **P5 (Editor)** has the deepest dependency chain -- needs ECS, serialization, all shape types (P4), egui (P2), and a working renderer.
- **Game phases** run in parallel with engine post-split phases, each starting when the corresponding engine phase delivers.

---

## Parallelization Strategy

### During the Split (Sequential -- Limited Parallelism)

The split phases are mostly sequential (ECS must complete before game logic extraction, etc.), but Split Phases 4 and 5 can run in parallel.

### Post-Split Engine Work (High Parallelism)

Many post-split sub-phases can run in parallel. Key opportunities:

```
Wave A (Can start immediately -- zero dependencies):
  P4 Wave 1: Shape types (Hyperprism4D, Hypersphere4D)  [1 session]

Wave B (After Split completes):
  P1-A: Raycasting         [1 session]    \
  P1-B: Collision Events   [0.75 session]  > parallel (shared world.rs needs merge)

Wave C (After P1 completes -- max parallelism):
  P2-A: Audio (kira)       [1.5-2 sessions]  \
  P2-B: HUD (egui)         [1 session]        > 3 parallel agents
  P2-C: Particles           [1.5-2 sessions]  /
  ---
  P3-A: Sprites             [1.5 sessions]    \
  P3-B+C: Spatial + FSM     [1 session]        > 3 parallel agents
  P3-Particles              [1.5 sessions]    /  (note: shared with P2-C)
  ---
  P4 Wave 2: Trigger data   [0.5 session]     \
  P4 Wave 3: Tweens          [0.5 session]      > parallel
  P4 Wave 4: RON preview    [1-2 sessions]    /

Wave D (Integration):
  P2-D: Screen effects      [0.5 session]
  P4 Wave 5: Trigger runtime [0.5 session]

Wave E (After most features complete):
  P5 Wave 1 (parallel):
    Textures                 [1-2.5 sessions]
    Point lights + shadows   [2 sessions]
    Input rebinding          [0.5 session]
  P5 Wave 2 (sequential after Wave 1):
    Editor framework         [2 sessions]
    -> Entity editing        [2 sessions]  \
    -> W-slice navigation    [1-2 sessions]  > parallel after framework
    -> Scene operations      [1 session]    /
```

**Critical path with full parallelism:**
- Foundation: 1 session
- Split: 9.5-14 sessions
- Post-split engine critical path: ~12-14 sessions (P1 -> P2 -> P3 -> P5 chain, with parallelism reducing wall time)
- **Estimated total critical path: ~22-29 sessions**

---

## Session Summary

### Effort Estimates

| Stage | Sessions (Sequential) | Notes |
|-------|-----------------------|-------|
| Completed Phases 1-5 | 20-22 | DONE |
| Pre-Split Foundation | 1 | All tasks independent |
| Engine/Game Split | 9.5-14 | Mostly sequential |
| Post-Split P1: Combat Core | 1.75 | 2 parallel sub-phases |
| Post-Split P2: Weapons & Feedback | 4.5-5.5 | 3 parallel sub-phases + integration |
| Post-Split P3: Enemies & AI | 4.0 | 3 parallel sub-phases |
| Post-Split P4: Level Design | 4.5 | 5 waves, partial parallelism |
| Post-Split P5: Editor & Polish | 10-12.5 | 2 waves, high internal parallelism |
| **Engine Total (remaining)** | **35.25-42.75** | |
| Game Repo (parallel) | 13-21 | Runs alongside engine work |
| **Grand Total (remaining)** | **35.25-42.75 engine + 13-21 game** | |

### Critical Path Estimate (Accounting for Parallelism)

| Segment | Critical Path Sessions | Notes |
|---------|----------------------|-------|
| Foundation | 1 | Single session |
| Split | 9.5-14 | Sequential core |
| Post-split to playable combat | ~3-4 | P1 + basics |
| Post-split to full engine | ~12-14 | P5 editor is the long tail |
| **Engine critical path** | **~22-29** | Foundation through Editor |
| **Game critical path** | **~13-21** | Starts after Split Phase 4, partially parallel |

---

## Legacy Plans (Superseded)

The following long-term plans were created during the original Phase 1-5 roadmap. They are now **SUPERSEDED** by the new post-split roadmap, which incorporates their relevant content into implementation-ready plans. They are kept for historical reference.

| Plan | Status | Superseded By |
|------|--------|---------------|
| [ECS Migration](./long-term-ecs.md) | Superseded | Split Phases 1-5 (full hecs migration) |
| [Visual Scene Editor](./long-term-visual-editor.md) | Superseded | Post-Split P5 Sub-Phase D (rust4d_editor crate) |
| [Scripting System](./long-term-scripting.md) | Superseded | Post-Split P4 Sub-Phase D (declarative triggers cover 80%) |
| [Networking](./long-term-networking.md) | Deferred | Not in current roadmap; foundation laid by fixed timestep + serialization |
| [Advanced Rendering](./long-term-rendering.md) | Superseded | Post-Split P5 Sub-Phases A+B (textures + lighting) |

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-27 | RON for scenes, TOML for config | RON handles Rust types well; TOML is human-friendly for settings |
| 2026-01-27 | Figment for config loading | Hierarchical overrides, env var support |
| 2026-01-27 | Defer ECS migration | Current Entity works; migrate when pain points emerge |
| 2026-01-27 | Architecture graphs required | ARCHITECTURE.md must include Mermaid diagrams |
| 2026-01-30 | Full ECS (hecs) over partial ECS | Partial ComponentStore approach superseded by full split plan; hecs provides real query iteration, component bundles, and proper ECS semantics |
| 2026-01-30 | Engine/game split decided | Engine becomes generic 4D library; game repo owns all shooter-specific code. Enables clean API boundaries and reuse for future games |
| 2026-01-30 | Git URL hybrid dependency model | Game repo uses git URL deps (works anywhere) with .cargo/config.toml path overrides for local dev |
| 2026-01-30 | kira chosen over rodio for audio | Built-in spatial audio, tweens, mixer/tracks, game-focused design. Higher complexity abstracted by rust4d_audio crate |
| 2026-01-30 | Triplanar mapping before UV pipeline for textures | No compute shader changes needed; works immediately with all shapes; proven technique for architectural geometry. UV path deferred until quality evaluation |
| 2026-01-30 | egui for both HUD and editor | Front-loads the egui dependency in P2 (HUD) that P5 (editor) needs. Avoids duplicate integration work. Rich widget set for both HUD and editor panels |
| 2026-01-30 | Particles are 3D, not 4D | Particles exist in sliced output space, bypass compute shader. Avoids W-distance visibility issues and compute cost for zero gameplay benefit |
| 2026-01-30 | Health/damage is 100% game-side | Different games have wildly different health models. Engine provides collision events and raycasting; game defines what "damage" means |
| 2026-01-30 | Collision layer presets stay in engine | `CollisionFilter::player()`, `::enemy()`, etc. are useful defaults for any game |
| 2026-01-30 | Declarative trigger system (RON) | Covers 80% of level scripting needs. `GameEvent(String)` escape hatch bridges engine and game |

---

## How to Use These Plans

### Starting New Work

1. **Check this index** to find the relevant plan document
2. **Read the plan** -- each contains file-level task lists, API designs, verification criteria
3. **Check dependencies** -- each plan lists what must be complete first
4. **Check parallelization notes** -- many sub-phases can run as parallel agents/worktrees

### For the Engine/Game Split

1. Start with `split-phase-0-foundation.md` for pre-split fixes
2. Follow the full split plan at `scratchpad/plans/2026-01-30-engine-game-split.md`
3. Reference `split-phases-1-5.md` for a quick summary

### For Post-Split Feature Work

1. Each `post-split-phase-N-*.md` is self-contained with:
   - Engine vs game boundary (what the engine provides vs what the game builds)
   - API designs with code examples
   - Sub-phase breakdowns with session estimates
   - Parallelization strategy (which sub-phases can run simultaneously)
   - Verification criteria (checklists)
   - Cross-phase dependency details
2. P4 Wave 1 (shape types) can start at any time -- zero dependencies

### For Game Repo Work

1. Read `game-roadmap-summary.md` for the full game-side plan
2. Each game phase maps to an engine phase -- game work starts when engine APIs are delivered

### Swarm Execution

- Each post-split phase identifies parallel sub-phases suitable for multi-agent swarms
- The dependency graph above shows which phases can overlap
- P4 Wave 1 (shapes) is the best early parallel task (zero dependencies)

---

## Source Documents

These plans were synthesized from multiple review swarms:

### Original Engine Review (2026-01-27)
- `scratchpad/reports/2026-01-27-engine-review-swarm/` -- Initial codebase review and roadmap

### Multi-Swarm Engine Review (2026-01-30)
- `scratchpad/reports/2026-01-30-multi-engine-review/cross-swarm-synthesis.md` -- Integrated findings from 7 agents across 3 swarms
- `scratchpad/reports/2026-01-30-multi-engine-review/swarm-a-codebase/` -- Codebase state review
- `scratchpad/reports/2026-01-30-multi-engine-review/swarm-b-roadmap/` -- Roadmap feasibility
- `scratchpad/reports/2026-01-30-multi-engine-review/swarm-c-features/` -- Feature and genre analysis

### Engine/Game Split Plan (2026-01-30)
- `scratchpad/plans/2026-01-30-engine-game-split.md` -- Full split implementation plan

### Engine Roadmap Planning Swarm (2026-01-30)
- `scratchpad/reports/2026-01-30-engine-roadmap/hive-mind.md` -- Cross-phase coordination notes
- `scratchpad/reports/2026-01-30-engine-roadmap/agent-*-report.md` -- Individual agent reports for each phase

### Roadmap Synthesis (2026-01-31)
- `scratchpad/reports/2026-01-31-roadmap-synthesis/` -- Final consolidation into plan documents
