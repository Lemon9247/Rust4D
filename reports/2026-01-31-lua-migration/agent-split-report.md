# Agent Split: Completion Report

**Date**: 2026-01-31
**Agent**: Split Agent
**Task**: Rewrite engine/game split plan for Lua scripting approach

## What I Did

Completely rewrote `scratchpad/plans/engine-roadmap-2026/engine-game-split.md` to reflect the Lua scripting architecture. This was a full replacement -- no content from the original plan was preserved verbatim, though the ECS migration and game logic extraction phases remain structurally similar since they are engine-internal changes that must happen regardless of the scripting approach.

## Key Changes from Original Plan

### Structural Changes
- **6 phases** (up from 5): Added Phase 3 (Lua Scripting Integration) and Phase 4 (Engine Binary/Launcher). Old Phase 3 (Pluggable Scenes) merged into Phase 2.
- **Total session estimate**: 14.5-22 sessions (up from 9.5-14). The Lua binding phase adds 4-6 sessions, and the launcher adds 2-3 sessions, but the game repo setup phase drops from 1-2 to 1-2 (simpler since it is just scripts + data).
- **Phase 3 has internal parallelism**: Bindings for math, physics, ECS, input, and game modules are independent and could be done by parallel agents.

### Architectural Changes
- **Engine ships a binary** (`rust4d` launcher) instead of being library-only. The binary loads a game directory via CLI argument.
- **New `rust4d_scripting` crate**: Owns the Lua VM (mlua), all bindings, script lifecycle, hot-reload, and error handling.
- **`rust4d_game` role changes**: Still provides CharacterController4D, EventBus, FSM in Rust, but these are exposed TO Lua via UserData rather than imported by a Rust game binary.
- **Game repo has no Cargo.toml**: It is a directory of Lua scripts + RON scenes + TOML config + assets. Running the game is `rust4d --game ./path/`.
- **Game directory model**: Engine binary defaults to `./game/` or takes `--game <path>`. For development, game repo can be a subdirectory (gitignored) or sibling directory.

### Design Decisions Made
1. **mlua with Lua 5.4** (not LuaJIT): integers, mature stdlib, active development. LuaJIT can be evaluated later.
2. **Script lifecycle via callbacks**: `on_init`, `on_update`, `on_fixed_update`, `on_event`, `on_reload`. Explicit, no magic.
3. **Hot-reload via notify**: File watcher, module cache invalidation, `persist` table for state preservation. Errors during reload keep old module.
4. **Script-defined components**: Lua tables stored in hecs via generic `ScriptComponent` wrapper. Engine does not need to know game component types.
5. **ECS from Lua**: String-based queries (`world:query("Transform4D", "Health")`), table-based component bundles for spawning.

### New Risks Identified
- **Lua binding surface area** (~50-80 functions/methods to bind): The biggest new engineering effort. Must be maintained in sync with Rust API.
- **ECS bridge complexity**: Exposing hecs queries to Lua without Rust's type system requires careful design.
- **Debugging Lua**: No compiler errors, need good error messages with file/line/stack trace.
- **Hot-reload edge cases**: Module dependency tracking, global state, closure captures.

### Upside Noted
- **Two-repo coordination is simpler**: No Cargo version resolution, no git URL syncing, no cross-repo compilation. Game just needs a compatible engine binary.
- **Post-split feature phases** each need a binding task (0.5-1 session) when new Rust APIs are added, but game-side work iterates much faster (no recompilation).

## What I Preserved from the Original Plan
- Phase 1 (ECS Migration): Essentially unchanged. Internal engine refactor.
- Phase 2 (Game Logic Extraction): Same scope plus scene helpers merged in. CharacterController4D, event system, input refactor all proceed the same way.
- Code refactoring tasks (PhysicsWorld player methods, PhysicsConfig.jump_velocity, scene setup, etc.): Same actions needed regardless of scripting approach.
- ECS migration design (hecs components, World wrapper): Unchanged.
- Risk items about hecs API differences, test porting, RenderableGeometry coupling: Still apply.

## Files Modified
- `scratchpad/plans/engine-roadmap-2026/engine-game-split.md` -- full rewrite
- `scratchpad/reports/2026-01-31-lua-migration/agent-split-report.md` -- this report
