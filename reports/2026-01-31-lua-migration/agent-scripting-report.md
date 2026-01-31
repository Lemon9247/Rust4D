# Agent Scripting: Completion Report

**Agent**: Scripting Agent
**Task**: Create detailed plan for the Lua scripting engine phase
**Date**: 2026-01-31
**Output**: `scratchpad/plans/engine-roadmap-2026/post-split-phase-scripting.md`

---

## What I Did

Created a comprehensive plan for the `rust4d_scripting` crate -- the new Lua scripting engine that replaces compiled Rust game code. The plan covers:

1. **Architecture**: Full crate structure with 10 source files across `lib.rs`, `vm.rs`, `loader.rs`, `lifecycle.rs`, `hot_reload.rs`, `error.rs`, `dev_console.rs`, and 8 binding modules
2. **5 Sub-Phases** (A through E) with detailed API designs, Rust binding code examples, Lua usage examples, file lists, tests, and session estimates
3. **4 complete example Lua scripts** showing what real game code looks like: entity spawner, hitscan shotgun weapon, enemy AI state machine, and trigger handler
4. **Performance analysis** identifying what stays in Rust vs what runs in Lua, with specific overhead minimization strategies

## Key Design Decisions

### 1. mlua with Lua 5.4 (LuaJIT optional via feature flag)
Lua 5.4 is the safe default with generational GC and integer support. LuaJIT can be enabled with `--features luajit` for production builds. The API is identical either way through mlua's abstraction.

### 2. ECS bridge via dynamic components
Since hecs uses Rust's type system and Lua is dynamically typed, we use a two-tier approach: engine-defined components (Transform4D, RigidBody4D) get optimized fast-path bindings, while script-defined components (Health, Weapon, EnemyAI) use a `LuaComponent` wrapper with table serialization. This avoids forcing scripts to know about Rust types while keeping hot-path components fast.

### 3. Global module pattern for engine singletons
Systems like `physics`, `audio`, `input`, `world`, `particles` are exposed as global Lua tables, matching the convention used by Love2D, Defold, and other Lua game frameworks. This is more idiomatic than requiring scripts to construct or receive these as parameters.

### 4. Event system as the primary game<->engine bridge
Rather than polling collision events directly, scripts register event handlers via `events.on("trigger_enter", callback)`. The engine forwards physics events, trigger events, and game events through this unified system. This is cleaner than having scripts call `physics.drain_events()` directly.

### 5. Hot-reload via package.loaded clearing
When a file changes, we clear `package.loaded[module_name]` and re-execute the module. The global `main.lua` callbacks naturally pick up the new module functions. Failed reloads keep the old version running -- the developer sees the error and fixes the script.

## Session Estimates

| Sub-Phase | Sessions | Critical Path? |
|-----------|----------|---------------|
| A: Core Runtime | 2 | Yes (blocks everything) |
| B: ECS Bindings | 2-3 | Yes (blocks game logic) |
| C: Engine API Bindings | 2-3 | Partially (incremental as P1-P4 complete) |
| D: Hot-Reload | 1 | No (parallel with B) |
| E: Game Framework Bindings | 1-2 | Yes (last in chain) |
| **Total** | **8-11** | **Critical path: ~7.5 sessions** |

## Dependencies Identified

- **Hard blocker**: Engine/game split must be complete (ECS with hecs, `rust4d_game` crate exists)
- **Incremental**: Sub-Phase C bindings roll out as P1-P4 APIs arrive -- physics bindings after P1, audio after P2, sprites/FSM after P3, triggers/tweens after P4
- **Parallelizable**: Sub-Phases B and D can run in parallel after A completes

## What I Noticed

1. **The superseded scripting plan was per-entity**: It attached scripts to individual entities (`entity.attach_script(script)`). The new plan uses a **global script architecture** where `main.lua` orchestrates everything and scripts query/modify entities through the ECS. This is more powerful and matches how modern Lua games (Factorio, Defold) work.

2. **Component serialization is the hardest part of ECS bindings**: Converting between Lua tables and hecs components requires careful design. The fast-path for Transform4D avoids serialization overhead for the most frequently accessed component.

3. **The HUD binding is deliberately simplified**: Full egui exposure to Lua would be enormously complex. Instead, we expose a simple `hud.text()` / `hud.rect()` API that covers 90% of HUD needs. Advanced layouts can be done in Rust if needed.

4. **Timer system can be pure Lua**: No Rust binding needed for `timers.after()` and `timers.every()` -- it's a simple table of callbacks checked each frame. This is provided as a standard library Lua file bundled with the engine.

## Open Questions for Other Agents

1. **For Agent Split**: Does the engine binary (`rust4d`) become just a Lua launcher? Or do we keep the current binary and add a launcher mode? The scripting plan assumes a launcher that loads scripts from a configurable directory.

2. **For Agent Amendments**: P1-P4 plans reference game-side Rust code. All of that becomes Lua scripts. The APIs the engine exposes remain the same, but the consumption changes from `PhysicsWorld::raycast()` in Rust to `physics.raycast()` in Lua.

3. **For Agent Game**: The game roadmap should now reference Lua scripts instead of Rust game code. The weapon system, enemy AI, HUD, triggers -- all become `.lua` files in the game repo's `scripts/` directory.
