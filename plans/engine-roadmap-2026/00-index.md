# Rust4D Engine Roadmap 2026

**Created:** 2026-01-27
**Updated:** 2026-01-31
**Status:** Active Development -- Phases 1-5 complete, preparing for Lua-scripted engine/game split

---

## Overview

Rust4D is a 4D game engine written in Rust. The engine targets a 4D boomer shooter ("Rust4D-Shooter") as its first game. The game will be written in **Lua scripts** loaded by the engine runtime -- no compiled game code. The engine exposes all APIs to Lua via `rust4d_scripting` (mlua), supports hot-reload, and ships as a binary that loads a game directory.

The roadmap has three stages:
1. **Completed original phases (1-5)** -- Foundation, scene management, documentation, architecture, advanced features
2. **Engine/game split + Lua scripting** -- ECS migration, crate restructuring, Lua runtime, engine binary
3. **Post-split feature development** -- Combat, audio, enemies, level tools, editor (each with Lua bindings)

**Centralized execution plan:** [execution-plan.md](./execution-plan.md) -- Breaks all work into parallel waves for multi-swarm execution.

---

## Part I: Completed Work (Original Phases 1-5)

These phases are **COMPLETE**. They established the engine's foundation: 4D math (Rotor4 geometric algebra), GPU hyperplane slicing, physics, scene management, asset caching, and entity hierarchy.

| Phase | Plan | Sessions | Status |
|-------|------|----------|--------|
| 1 | [Foundation](./completed/phase-1-foundation.md) | 4 | **COMPLETE** |
| 2 | [Scene Management](./completed/phase-2-scene-management.md) | 3 | **COMPLETE** (via EntityTemplate) |
| 3 | [Documentation](./completed/phase-3-documentation.md) | 4-5 | **COMPLETE** |
| 4 | [Architecture](./completed/phase-4-architecture.md) | 4 | **COMPLETE** |
| 5 | [Advanced Features](./completed/phase-5-advanced-features.md) | 5-6 | **COMPLETE** |

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

## Part II: Engine/Game Split + Lua Scripting

The engine will be refactored into a Lua-scriptable runtime. The game is Lua scripts + RON scenes + assets loaded by the engine binary.

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

### Stage 2: Engine/Game Split (14.5-22 sessions)

**Full plan:** [engine-game-split.md](./engine-game-split.md)

| Phase | Name | Sessions | Depends On |
|-------|------|----------|------------|
| 1 | ECS Migration (hecs) | 4-6 | Phase 0 Foundation |
| 2 | Game Logic Extraction + rust4d_game | 3-4 | Phase 1 |
| 3 | Lua Scripting Integration | 4-6 | Phase 2 |
| 4 | Engine Binary / Launcher | 2-3 | Phase 3 |
| 5 | Game Repo Setup | 1-2 | Phase 4 |
| 6 | Engine Cleanup | 0.5-1 | Phase 3 |

**Key decisions:**
- Full hecs ECS migration (not partial ComponentStore approach)
- Lua 5.4 via mlua for scripting (not Rhai, not LuaJIT)
- New `rust4d_scripting` crate for Lua runtime, bindings, hot-reload
- New `rust4d_game` crate for CharacterController4D, events, scene helpers (exposed to Lua)
- Engine ships a binary: `rust4d --game ./path/to/game/`
- Game repo is Lua scripts + RON scenes + TOML config + assets (no Cargo.toml)
- Input refactored to action/axis abstraction (InputAction, InputMap)

**Target architecture:**
```
Rust4D/                              # Engine repo
  crates/
    rust4d_math                      # Pure 4D math (Vec4, Rotor4, shapes)
    rust4d_physics                   # Generic 4D physics
    rust4d_core                      # ECS via hecs, scenes, assets
    rust4d_game                      # CharacterController4D, events (Lua-wrapped)
    rust4d_render                    # GPU rendering pipeline
    rust4d_input                     # Action/axis input abstraction
    rust4d_scripting                 # NEW: Lua runtime, bindings, hot-reload
  src/main.rs                        # Engine binary / launcher

Rust4D-Shooter/                      # Game repo (Lua + data)
  main.lua                           # Entry point
  scripts/                           # Game logic (Lua)
  scenes/                            # RON scene files
  config.toml                        # Game configuration
  assets/                            # Sprites, sounds, textures
```

---

## Part III: Post-Split Engine Features

After the split is complete, these phases add engine capabilities. Each phase plan includes Lua binding work integrated directly alongside the core Rust implementation.

| Phase | Plan | Sessions (with Lua) | Status | Key Dependencies |
|-------|------|---------------------|--------|------------------|
| Scripting | [post-split-phase-scripting.md](./post-split-phase-scripting.md) | 8-11 | Planned | Split Phase 2 (rust4d_game exists) |
| P1: Combat Core | [post-split-phase-1-combat-core.md](./post-split-phase-1-combat-core.md) | 2.25-2.5 | Planned | Split complete |
| P2: Weapons & Feedback | [post-split-phase-2-weapons-feedback.md](./post-split-phase-2-weapons-feedback.md) | 5.5-7.0 | Planned | P1 (raycasting) |
| P3: Enemies & AI | [post-split-phase-3-enemies-ai.md](./post-split-phase-3-enemies-ai.md) | 3.75-4.5 | Planned | P1 (raycasting for LOS) |
| P4: Level Design | [post-split-phase-4-level-design.md](./post-split-phase-4-level-design.md) | 4.25-5.0 | Planned | P1 (trigger events) |
| P5: Editor & Polish | [post-split-phase-5-editor-polish.md](./post-split-phase-5-editor-polish.md) | 11.5-15 | Planned | All prior phases |

**Note:** The Scripting phase builds the core Lua runtime and foundational bindings. P1-P5 each add bindings for their specific APIs as they're implemented. Binding work is incremental -- it rolls out alongside each phase.

### Game Roadmap (Lua)

**Plan:** [game-roadmap-lua.md](./game-roadmap-lua.md)
**Estimate:** 12-18 sessions (runs in parallel with engine post-split phases)

| Game Phase | Sessions | Engine Prerequisite |
|------------|----------|---------------------|
| Phase 0: Project Setup + Core Loop | 1 | Scripting runtime works |
| Phase 1: Combat Core | 2-3 | Engine P1 + Lua bindings |
| Phase 2: HUD + Audio + Feedback | 2-3 | Engine P2 + Lua bindings |
| Phase 3: Enemies | 3-4 | Engine P3 + Lua bindings |
| Phase 4: Level Design | 2-3 | Engine P4 + Lua bindings |
| Phase 5: Polish + Distribution | 2-4 | Engine P5 |

Game development starts as soon as the scripting runtime is functional and iterates via hot-reload.

---

## Session Summary

| Stage | Sessions | Notes |
|-------|----------|-------|
| Completed Phases 1-5 | 20-22 | DONE |
| Pre-Split Foundation | 1 | All tasks independent |
| Engine/Game Split + Lua | 14.5-22 | Includes scripting integration |
| Post-Split Scripting | 8-11 | rust4d_scripting crate |
| Post-Split P1-P5 (with Lua) | 27.25-34 | Includes Lua binding work per phase |
| **Engine Total (remaining)** | **50.75-68** | |
| Game (Lua, parallel) | 12-18 | Starts after scripting works |

See [execution-plan.md](./execution-plan.md) for the parallelized wave breakdown and critical path estimate.

---

## Legacy Plans (Superseded)

| Plan | Status | Superseded By |
|------|--------|---------------|
| [ECS Migration](./superseded/long-term-ecs.md) | Superseded | Split Phase 1 (full hecs migration) |
| [Visual Scene Editor](./superseded/long-term-visual-editor.md) | Superseded | Post-Split P5 Sub-Phase D |
| [Scripting System](./superseded/long-term-scripting.md) | Superseded | Split Phase 3 + post-split-phase-scripting.md |
| [Networking](./superseded/long-term-networking.md) | Deferred | Not in current roadmap |
| [Advanced Rendering](./superseded/long-term-rendering.md) | Superseded | Post-Split P5 Sub-Phases A+B |
| [Rust Game Roadmap](./superseded/game-roadmap-summary.md) | Superseded | game-roadmap-lua.md |
| [Rust Split Summary](./superseded/split-phases-1-5.md) | Superseded | engine-game-split.md (rewritten for Lua) |
| [Lua Phase Amendments](./superseded/lua-phase-amendments.md) | Superseded | Merged into each phase plan (P1-P5) on 2026-01-31 |

### Reference Documents

| Document | Purpose |
|----------|---------|
| [lua-migration-analysis.md](./lua-migration-analysis.md) | Analysis of what changes with Lua approach (binding surface, risks) |

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-27 | RON for scenes, TOML for config | RON handles Rust types well; TOML is human-friendly for settings |
| 2026-01-27 | Figment for config loading | Hierarchical overrides, env var support |
| 2026-01-27 | Architecture graphs required | ARCHITECTURE.md must include Mermaid diagrams |
| 2026-01-30 | Full ECS (hecs) over partial ECS | hecs provides real query iteration, component bundles, proper ECS semantics |
| 2026-01-30 | Engine/game split decided | Engine becomes generic 4D library; game code is separate |
| 2026-01-30 | kira chosen over rodio for audio | Built-in spatial audio, tweens, mixer/tracks, game-focused design |
| 2026-01-30 | Triplanar mapping before UV pipeline | No compute shader changes needed; works with all shapes |
| 2026-01-30 | egui for both HUD and editor | Front-loads egui in P2 (HUD) that P5 (editor) needs |
| 2026-01-30 | Particles are 3D, not 4D | Exist in sliced output space, bypass compute shader |
| 2026-01-30 | Collision layer presets stay in engine | Useful defaults for any game |
| 2026-01-31 | **Lua scripting over compiled Rust game** | Hot-reload, modding support, clearer separation, lower barrier for gameplay code |
| 2026-01-31 | **Lua over Rhai** | Proven in games, larger ecosystem, better tooling, community familiarity for modding |
| 2026-01-31 | **mlua with Lua 5.4** | Active maintenance, good userdata/metatable support, integers, generational GC |
| 2026-01-31 | **Game directory model** | Engine binary loads game from path arg. Game repo is just that directory. |
| 2026-01-31 | **LuaHud API instead of raw egui** | egui API too complex for Lua. Simplified draw API (hud:draw_text, hud:draw_bar) |
| 2026-01-31 | **FSM removed from rust4d_game** | Lua tables/functions/coroutines handle state machines natively |
| 2026-01-31 | **Triggers call Lua directly** | TriggerAction::Callback replaces GameEvent(String). More powerful. |

---

## Source Documents

### Review Swarms
- `scratchpad/reports/2026-01-30-multi-engine-review/` -- 7-agent codebase + roadmap + feature review
- `scratchpad/reports/2026-01-30-engine-roadmap/` -- 6-agent phase planning swarm

### Synthesis
- `scratchpad/reports/2026-01-31-roadmap-synthesis/` -- Consolidation into plan documents
- `scratchpad/reports/2026-01-31-lua-migration/` -- Lua migration analysis and plan updates
