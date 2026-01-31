# Execution Plan: Parallel Waves for Multi-Swarm Development

**Created:** 2026-01-31
**Purpose:** Break the full Rust4D roadmap into parallelized waves executable by multi-swarm coordination.

---

## How to Read This Document

Each **wave** is a set of tasks that can run simultaneously. Waves execute sequentially -- all tasks in a wave must complete before the next wave starts. Within each wave, tasks are assigned to **swarms** (groups of agents working in separate git worktrees).

A wave completes when all its swarms finish. Some waves have a single swarm (sequential bottleneck); others have 2-4 parallel swarms.

**Plan documents:** Each task references its detailed plan. Read the plan before starting the task.

---

## Wave 0: Pre-Split Foundation
**Swarms:** 1 | **Sessions:** ~1 | **Blocked by:** Nothing

Everything here is independent and small enough for a single session.

| Task | Plan | Estimate | Notes |
|------|------|----------|-------|
| Rotor4 Serialization | [split-phase-0-foundation.md](./split-phase-0-foundation.md) | 0.25 | BLOCKING prereq for ECS |
| Fixed Timestep | [split-phase-0-foundation.md](./split-phase-0-foundation.md) | 0.5 | Accumulator in PhysicsWorld |
| Diagonal Movement Normalization | [split-phase-0-foundation.md](./split-phase-0-foundation.md) | 0.1 | Quick fix |
| Re-enable Back-Face Culling | [split-phase-0-foundation.md](./split-phase-0-foundation.md) | 0.1 | Quick fix, may reveal winding issues |
| P4-A: Shape Type Expansion | [post-split-phase-4-level-design.md](./post-split-phase-4-level-design.md) | 1.0 | **ZERO dependencies** -- Hyperprism4D + Hypersphere4D. Can start here. |

**Swarm 1** (single agent or small swarm): All foundation fixes + shape types.

> Shape types are pulled forward from P4 because they have zero dependencies and touch isolated files (rust4d_math shapes, rust4d_render geometry generation). Getting them done early unblocks P4 and P5 later.

---

## Wave 1: ECS Migration
**Swarms:** 1 | **Sessions:** 4-6 | **Blocked by:** Wave 0

This is the largest sequential bottleneck. The entire Entity/World system is rewritten to hecs. Cannot be easily parallelized because changes are deeply interconnected.

| Task | Plan | Estimate | Notes |
|------|------|----------|-------|
| ECS Migration (hecs) | [engine-game-split.md](./engine-game-split.md) Phase 1 | 4-6 | Replace Entity struct + World with hecs. Port 202 core tests + render/physics integration. |

**Swarm 1**: ECS migration team. See split plan Phase 1 for the full file list and task breakdown.

---

## Wave 2: Game Logic Extraction
**Swarms:** 1 | **Sessions:** 3-4 | **Blocked by:** Wave 1

Extract player-specific code from PhysicsWorld, create rust4d_game crate, refactor input to action/axis abstraction. Scene helpers merged into this phase.

| Task | Plan | Estimate | Notes |
|------|------|----------|-------|
| Game Logic Extraction + rust4d_game | [engine-game-split.md](./engine-game-split.md) Phase 2 | 3-4 | CharacterController4D, events, input actions, scene helpers. 97 physics + 37 input tests to update. |

**Swarm 1**: Extraction team. See split plan Phase 2.

---

## Wave 3: Lua Scripting + Combat Core
**Swarms:** 2 | **Sessions:** 4-6 (critical path) | **Blocked by:** Wave 2

This is the first major parallelism point. The scripting runtime and the combat core engine work are independent -- scripting wraps existing APIs while combat adds new ones.

| Swarm | Task | Plan | Estimate |
|-------|------|------|----------|
| **Swarm A** | Scripting Core Runtime (Sub-Phase A) | [post-split-phase-scripting.md](./post-split-phase-scripting.md) | 2 |
| **Swarm A** | Scripting ECS Bindings (Sub-Phase B) | [post-split-phase-scripting.md](./post-split-phase-scripting.md) | 2-3 |
| **Swarm B** | P1-A: Raycasting | [post-split-phase-1-combat-core.md](./post-split-phase-1-combat-core.md) + [amendments](./lua-phase-amendments.md) | 1 |
| **Swarm B** | P1-B: Collision Events + Triggers | [post-split-phase-1-combat-core.md](./post-split-phase-1-combat-core.md) + [amendments](./lua-phase-amendments.md) | 0.75-1.0 |

**Swarm A** (Scripting): Lua VM init, script loading, lifecycle callbacks, ECS bindings. This gives us a working `rust4d_scripting` that can spawn entities and run update loops in Lua.

**Swarm B** (Combat): Raycasting + collision events. Pure Rust engine work. Lua bindings for these APIs come in Wave 4.

> P1-A and P1-B can themselves be parallel (different files, shared world.rs needs merge coordination).

---

## Wave 4: Engine Features + Scripting Bindings (Maximum Parallelism)
**Swarms:** 3-4 | **Sessions:** 2-3 (critical path) | **Blocked by:** Wave 3

The biggest parallelism opportunity. Multiple engine features build simultaneously while scripting bindings catch up to wrap the new APIs.

| Swarm | Task | Plan | Estimate |
|-------|------|------|----------|
| **Swarm A** | P2-A: Audio System (rust4d_audio + kira) | [post-split-phase-2-weapons-feedback.md](./post-split-phase-2-weapons-feedback.md) | 1.5-2 |
| **Swarm A** | P2-B: HUD/egui Overlay + LuaHud API | [post-split-phase-2-weapons-feedback.md](./post-split-phase-2-weapons-feedback.md) + [amendments](./lua-phase-amendments.md) | 1-1.5 |
| **Swarm B** | P2-C: Particle System | [post-split-phase-2-weapons-feedback.md](./post-split-phase-2-weapons-feedback.md) | 1.5-2 |
| **Swarm B** | P3-A: Sprite/Billboard Pipeline | [post-split-phase-3-enemies-ai.md](./post-split-phase-3-enemies-ai.md) | 1.5 |
| **Swarm C** | P3-B: Spatial Queries + Area Damage | [post-split-phase-3-enemies-ai.md](./post-split-phase-3-enemies-ai.md) | 0.5-1 |
| **Swarm C** | P4-B: RON Preview Tool | [post-split-phase-4-level-design.md](./post-split-phase-4-level-design.md) | 1-2 |
| **Swarm C** | P4-C: Tween System | [post-split-phase-4-level-design.md](./post-split-phase-4-level-design.md) | 0.5 |
| **Swarm D** | Scripting: Engine API Bindings (Sub-Phase C) | [post-split-phase-scripting.md](./post-split-phase-scripting.md) | 2-3 |
| **Swarm D** | Scripting: Hot-Reload (Sub-Phase D) | [post-split-phase-scripting.md](./post-split-phase-scripting.md) | 1 |

**Swarm A** (Audio + HUD): New rust4d_audio crate + egui overlay + LuaHud API.
**Swarm B** (Visuals): Particle system + sprite/billboard rendering pipeline. Both touch rust4d_render.
**Swarm C** (Tools + Queries): Spatial queries, tweens, RON preview tool. Lower coupling.
**Swarm D** (Scripting): Bindings for math, physics, input, audio, rendering, scene APIs + hot-reload.

> Swarm D binds APIs as Swarms A-C build them. It can start immediately on math/physics/input bindings (already exist from Waves 0-2), then bind audio/particles/sprites as they land.

---

## Wave 5: Integration + Engine Binary + Game Start
**Swarms:** 2-3 | **Sessions:** 2-3 (critical path) | **Blocked by:** Wave 4

Integration wave: wire everything together, build the engine launcher, start the game.

| Swarm | Task | Plan | Estimate |
|-------|------|------|----------|
| **Swarm A** | P2-D: Screen Effects (ScreenShake, TimedEffect) | [post-split-phase-2-weapons-feedback.md](./post-split-phase-2-weapons-feedback.md) | 0.5 |
| **Swarm A** | P4-D: Declarative Trigger System + Lua callbacks | [post-split-phase-4-level-design.md](./post-split-phase-4-level-design.md) + [amendments](./lua-phase-amendments.md) | 1.0 |
| **Swarm A** | Scripting: Game Framework Bindings (Sub-Phase E) | [post-split-phase-scripting.md](./post-split-phase-scripting.md) | 1-2 |
| **Swarm B** | Engine Binary / Launcher | [engine-game-split.md](./engine-game-split.md) Phase 4 | 2-3 |
| **Swarm C** | Game Phase 0: Project Setup + Core Loop | [game-roadmap-lua.md](./game-roadmap-lua.md) | 1 |

**Swarm A** (Integration): Screen effects, trigger system with Lua callbacks, game framework Lua bindings (CharacterController4D, events, tweens).
**Swarm B** (Launcher): Engine binary that parses CLI args, loads game directory, runs the Lua game loop.
**Swarm C** (Game): Create game repo structure, main.lua, basic game state machine, player movement via hot-reload.

> Game Phase 0 can start as soon as the scripting runtime + ECS bindings work (end of Wave 3), but placing it here ensures the engine binary and full binding surface are ready.

---

## Wave 6: Game Development + Engine Polish (Ongoing Parallel)
**Swarms:** 2 | **Sessions:** 4-6 (critical path) | **Blocked by:** Wave 5

Engine and game development run in parallel. Engine builds remaining features; game builds gameplay.

| Swarm | Task | Plan | Estimate |
|-------|------|------|----------|
| **Swarm A (Engine)** | P5-A: Texture Support (triplanar) | [post-split-phase-5-editor-polish.md](./post-split-phase-5-editor-polish.md) | 1.5-2.5 |
| **Swarm A (Engine)** | P5-B: Lighting (point lights + shadows) | [post-split-phase-5-editor-polish.md](./post-split-phase-5-editor-polish.md) | 2 |
| **Swarm A (Engine)** | P5-C: Input Rebinding + Lua API | [post-split-phase-5-editor-polish.md](./post-split-phase-5-editor-polish.md) + [amendments](./lua-phase-amendments.md) | 0.5 |
| **Swarm A (Engine)** | Engine Cleanup | [engine-game-split.md](./engine-game-split.md) Phase 6 | 0.5-1 |
| **Swarm B (Game)** | Game Phase 1: Combat Core | [game-roadmap-lua.md](./game-roadmap-lua.md) | 2-3 |
| **Swarm B (Game)** | Game Phase 2: HUD + Audio + Feedback | [game-roadmap-lua.md](./game-roadmap-lua.md) | 2-3 |

**Swarm A** (Engine Polish): Textures, lighting, input rebinding. These are independent and can be further parallelized internally. Plus engine cleanup.
**Swarm B** (Game): Combat and weapons in Lua. Uses engine P1-P2 APIs via hot-reload.

> P5-A (textures), P5-B (lighting), and P5-C (input rebinding) are independent of each other. If you have 3 worktrees, each can be a separate agent.

---

## Wave 7: Editor + Game Enemies
**Swarms:** 2 | **Sessions:** 4-6 (critical path) | **Blocked by:** Wave 6

The editor framework starts after textures/lighting land. Game continues building.

| Swarm | Task | Plan | Estimate |
|-------|------|------|----------|
| **Swarm A (Engine)** | P5-D: Editor Framework (rust4d_editor) | [post-split-phase-5-editor-polish.md](./post-split-phase-5-editor-polish.md) + [amendments](./lua-phase-amendments.md) | 6-8 |
| **Swarm B (Game)** | Game Phase 3: Enemies | [game-roadmap-lua.md](./game-roadmap-lua.md) | 3-4 |
| **Swarm B (Game)** | Game Phase 4: Level Design | [game-roadmap-lua.md](./game-roadmap-lua.md) | 2-3 |

**Swarm A** (Editor): The largest single task. Entity list, properties, W-slice navigation, scene operations, Lua console, script editing panel. Internally parallelizable after the framework sub-phase.
**Swarm B** (Game): Enemy AI + level design in Lua.

---

## Wave 8: Final Polish
**Swarms:** 1-2 | **Sessions:** 2-4 | **Blocked by:** Wave 7

| Swarm | Task | Plan | Estimate |
|-------|------|------|----------|
| **Swarm A** | Game Phase 5: Polish + Distribution | [game-roadmap-lua.md](./game-roadmap-lua.md) | 2-4 |
| **Swarm A** | Documentation updates | -- | 0.5 |

Menu system, input rebinding UI, save/load, distribution packaging.

---

## Critical Path Summary

```
Wave 0: Foundation + Shapes            ~1 session
Wave 1: ECS Migration                  4-6 sessions
Wave 2: Game Logic Extraction          3-4 sessions
Wave 3: Scripting + Combat (parallel)  4-6 sessions (2 swarms)
Wave 4: Features (max parallel)        2-3 sessions (4 swarms)
Wave 5: Integration + Game Start       2-3 sessions (3 swarms)
Wave 6: Polish + Game Combat           4-6 sessions (2 swarms)
Wave 7: Editor + Game Enemies          4-6 sessions (2 swarms)
Wave 8: Final Polish                   2-4 sessions
                                       ─────────────
Critical Path Total:                   ~26-39 sessions
```

### Sequential Effort (total work if no parallelism)
- Engine: ~50.75-68 sessions
- Game: ~12-18 sessions
- **Total: ~62.75-86 sessions**

### With Parallelism (critical path)
- **~26-39 sessions wall-clock** (2.4x speedup from parallelism)

---

## Swarm Resource Requirements

| Wave | Swarms | Worktrees | Peak Agents |
|------|--------|-----------|-------------|
| 0 | 1 | 1 | 1-2 |
| 1 | 1 | 1 | 2-3 (ECS is big) |
| 2 | 1 | 1 | 2-3 |
| 3 | 2 | 2 | 4-6 |
| 4 | 3-4 | 3-4 | 6-10 |
| 5 | 2-3 | 2-3 | 4-6 |
| 6 | 2 | 2 | 3-5 |
| 7 | 2 | 2 | 3-5 |
| 8 | 1-2 | 1-2 | 1-3 |

**Peak parallelism:** Wave 4 (up to 4 swarms, 10 agents across worktrees)

---

## Pre-Wave Checklist

Before starting each wave:

1. **Verify prior wave is complete**: `cargo test --workspace` passes, all swarm reports written
2. **Read relevant plan documents**: Each task links to its detailed plan
3. **Set up worktrees**: One per swarm (use `/multi-swarm` skill)
4. **Check the hive-mind**: Swarms within a wave should coordinate via shared scratchpad
5. **Commit as you go**: Small, modular commits per feature/fix

---

## Quick Reference: All Plan Documents

| Document | Content |
|----------|---------|
| [00-index.md](./00-index.md) | This index |
| [execution-plan.md](./execution-plan.md) | This execution plan |
| [engine-game-split.md](./engine-game-split.md) | Split + Lua architecture (6 phases) |
| [split-phase-0-foundation.md](./split-phase-0-foundation.md) | Pre-split fixes |
| [post-split-phase-scripting.md](./post-split-phase-scripting.md) | Lua scripting crate (5 sub-phases) |
| [post-split-phase-1-combat-core.md](./post-split-phase-1-combat-core.md) | Raycasting + events |
| [post-split-phase-2-weapons-feedback.md](./post-split-phase-2-weapons-feedback.md) | Audio + HUD + particles |
| [post-split-phase-3-enemies-ai.md](./post-split-phase-3-enemies-ai.md) | Sprites + spatial queries |
| [post-split-phase-4-level-design.md](./post-split-phase-4-level-design.md) | Shapes + tools + triggers |
| [post-split-phase-5-editor-polish.md](./post-split-phase-5-editor-polish.md) | Editor + lighting + textures |
| [lua-phase-amendments.md](./lua-phase-amendments.md) | Lua-specific changes to P1-P5 |
| [lua-migration-analysis.md](./lua-migration-analysis.md) | Migration impact analysis |
| [game-roadmap-lua.md](./game-roadmap-lua.md) | Lua game roadmap |
