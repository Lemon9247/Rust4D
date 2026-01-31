# Agent Game: Completion Report

**Date**: 2026-01-31
**Agent**: Agent Game (Lua game roadmap)
**Task**: Create a complete game roadmap for building the 4D boomer shooter in Lua instead of Rust

## What Was Done

Created `scratchpad/plans/engine-roadmap-2026/game-roadmap-lua.md` -- a comprehensive Lua-based game roadmap that replaces the Rust-based `game-roadmap-summary.md`.

## Document Structure

The roadmap contains 11 sections:

1. **Overview** -- The architectural shift from Rust game repo to Lua scripts + data, what stays in Rust vs what moves to Lua
2. **Game Directory Structure** -- Full directory layout for `rust4d-shooter/` with conventions for systems, entities, states, scenes, and assets
3. **Development Workflow** -- The edit-save-play loop, hot-reload behavior, launching, debugging tools, distribution model
4. **Game Phases** (0-5) -- Detailed task lists for each development phase with session estimates and engine prerequisites
5. **W-Axis Gameplay Design** -- Preserved and expanded W-axis notes (strafing, explosions, hitscan alignment, layered levels, flanking, cognitive overload mitigation)
6. **Example Lua Scripts** -- Five substantial, realistic scripts showing what the actual game code looks like:
   - `main.lua` (entry point, system registration, state machine, hot-reload handler)
   - `scripts/systems/combat.lua` (damage processing, collision event handling, death)
   - `scripts/entities/weapons/shotgun.lua` (hitscan weapon with spread, falloff, particles, sound)
   - `scripts/entities/enemies/rusher.lua` (full AI state machine in Lua, spawning, damage handling)
   - `scripts/systems/hud.lua` (health bar, ammo, crosshair, W-indicator with threat detection)
7. **Session Estimates** -- Per-phase estimates, dependency table, critical path analysis
8. **Advantages of Lua Approach** -- Hot-reload, no compile times, modding, separation, accessibility, prototyping speed
9. **Risks and Mitigations** -- Performance, debugging, type safety, state management, API coverage, distribution, Lua version
10. **Lua API Design Principles** -- Namespacing, tables vs userdata, error messages, return patterns, Vec4 as first-class
11. **Summary** -- Consolidated overview

## Key Decisions

1. **Session estimate: 12-18 sessions** (down from 13-21 in the Rust version). Lua is slightly faster for game logic due to no compilation and faster iteration, but slightly slower for debugging due to dynamic typing.

2. **Game Phase 0 can start as soon as the scripting runtime works** -- it does not need to wait for P1-P5 engine phases. This means game development can begin in parallel with engine work much earlier than in the Rust approach.

3. **State on entities, logic in scripts** -- The key pattern for surviving hot-reload. All mutable game state (health, ammo, AI state, cooldown timers) lives on ECS entities as Lua tables. Script modules contain only logic and constant definitions. This way, hot-reloading a script does not lose game state.

4. **LuaJIT recommended over Lua 5.4** for performance (JIT compilation). The tradeoff is Lua 5.1 compatibility only, but for game scripting this is well-tested territory.

5. **All engine APIs namespaced under `engine.*`** -- Clean, discoverable, prevents pollution of the global Lua namespace.

6. **Example scripts serve as API specification** -- The five example scripts use `engine.ecs`, `engine.physics`, `engine.audio`, `engine.ui`, `engine.particles`, `engine.screen_shake`, `engine.events`, `engine.camera`, `engine.vec4`, and `engine.config`. These calls define the minimum binding surface the scripting phase must implement.

## Relationship to Other Agents

- **Agent Analysis**: The analysis of what changes per document informed which systems move to Lua
- **Agent Scripting**: The scripting phase plan defines the `rust4d_scripting` crate and Lua runtime that this game roadmap depends on. The `engine.*` API calls in the example scripts must match what Agent Scripting designs.
- **Agent Amendments**: The per-phase amendment notes identify which engine APIs need Lua bindings. This roadmap's dependency table aligns with those amendments.
- **Agent Split**: The engine-game split document is superseded by the Lua approach -- there is no separate game Cargo.toml. The split is now engine binary (Rust) + game directory (Lua + data).

## Open Questions

1. **LuaJIT vs Lua 5.4**: Recommended LuaJIT for performance, but this should be confirmed with the scripting phase plan. mlua supports both via feature flags.

2. **ECS component storage for Lua tables**: How exactly does `engine.ecs:set(entity, "Health", { current = 50, max = 100 })` work under the hood? The scripting crate needs to store arbitrary Lua tables as ECS components. This is a design question for Agent Scripting.

3. **Modding sandboxing**: The roadmap mentions modding support, but does not address sandboxing (preventing mods from accessing the filesystem, network, etc.). This is an engine concern for a future phase.

4. **Bundled distribution**: The roadmap describes a "game directory next to engine binary" model. Whether to embed Lua scripts into the binary for single-file distribution is deferred.
