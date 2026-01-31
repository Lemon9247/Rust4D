# Lua Migration Analysis - Final Synthesis Report

**Date**: 2026-01-31
**Task**: Adapt Rust4D roadmap from "game in Rust" to "game in Lua"

---

## Executive Summary

Five agents analyzed the full Rust4D roadmap and produced updated plans for a Lua scripting approach. The architectural shift replaces the compiled Rust game binary with Lua scripts loaded by the engine runtime. Engine-internal work (physics, rendering, math, collision) is 95% unchanged. The main new cost is the Lua scripting layer (~8-11 sessions for `rust4d_scripting` crate). Game-side work shrinks from 13-21 sessions (Rust) to 12-18 sessions (Lua) due to faster iteration. The net effect on total effort is roughly neutral, while gaining hot-reload, modding support, and clearer engine/game separation.

---

## Documents Produced

### New Files
| File | Size | Agent | Purpose |
|------|------|-------|---------|
| `lua-migration-analysis.md` | 45KB | Analysis | Per-document impact analysis, binding surface area, risk assessment |
| `engine-game-split.md` | 45KB | Split | Complete rewrite: 6-phase plan for Lua-based split (14.5-22 sessions) |
| `post-split-phase-scripting.md` | 73KB | Scripting | New phase: rust4d_scripting crate, mlua, bindings, hot-reload (8-11 sessions) |
| `lua-phase-amendments.md` | 39KB | Amendments | What changes in P1-P5 for Lua (+2.5 to +6.2 sessions total) |
| `game-roadmap-lua.md` | 66KB | Game | Full Lua game roadmap with example scripts (12-18 sessions) |

### Modified Files
| File | Change |
|------|--------|
| `engine-game-split.md` | Full rewrite (was Rust binary, now Lua scripts) |

## Key Architectural Decisions

| Decision | Rationale |
|----------|-----------|
| **mlua with Lua 5.4** | Most actively maintained Rust Lua crate, good userdata/metatable support, Lua 5.4 has integers and generational GC |
| **Game directory model** | Engine binary takes `--game ./path/` argument. Game repo is just that directory. No Cargo.toml needed. |
| **Callback-based lifecycle** | Scripts register on_init, on_update, on_fixed_update, on_event, on_shutdown, on_reload |
| **ScriptComponent wrapper** | Lua-defined components stored as `ScriptComponent(LuaTable)` in hecs, enabling ECS queries that include script data |
| **Hot-reload via notify** | File watcher clears `package.loaded`, re-requires modules, calls on_reload(). Old module continues on failure. |
| **LuaHud API instead of raw egui** | Exposing egui to Lua is impractical. Engine provides simplified `hud:draw_text()`, `hud:draw_bar()`, etc. |
| **FSM removed from rust4d_game** | Lua tables/functions/coroutines handle state machines natively. Rust FSM is unnecessary overhead. |
| **Triggers call Lua directly** | `TriggerAction::Callback("function_name")` replaces `GameEvent(String)` escape hatch. More powerful. |

## Session Estimates

### Split + Scripting (replaces old split plan)
| Phase | Sessions | Change from Original |
|-------|----------|---------------------|
| Phase 1: ECS Migration | 4-6 | Unchanged |
| Phase 2: Game Logic Extraction | 3-4 | Unchanged (scene helpers merged in) |
| Phase 3: Lua Scripting Integration | 4-6 | **NEW** |
| Phase 4: Engine Binary/Launcher | 2-3 | **NEW** (replaces "Create Game Repo") |
| Phase 5: Game Repo Setup | 1-2 | Simpler (Lua + data, no Cargo.toml) |
| Phase 6: Engine Cleanup | 0.5-1 | Unchanged |
| **Total** | **14.5-22** | Was 9.5-14 (+5 sessions for scripting) |

### Post-Split Engine Phases (amended)
| Phase | Original | With Lua | Delta |
|-------|----------|----------|-------|
| P1: Combat Core | 1.75 | 2.0-2.25 | +0.25-0.5 (binding wrappers) |
| P2: Weapons & Feedback | 4.5-5.5 | 5.25-6.5 | +0.75-1.0 (LuaHud API) |
| P3: Enemies & AI | 4.0 | 3.75-4.25 | -0.25 to +0.25 (FSM removed, bindings added) |
| P4: Level Design | 4.5 | 4.75-5.5 | +0.25-1.0 (Lua trigger callbacks) |
| P5: Editor & Polish | 10-12.5 | 10.5-15 | +0.5-2.5 (script editor panel) |
| **Total** | **24.75-28.25** | **26.25-33.5** | +1.5 to +5.25 |

### Game (Lua)
| Phase | Sessions |
|-------|----------|
| Phase 0: Setup + Core Loop | 1 |
| Phase 1: Combat Core | 2-3 |
| Phase 2: HUD + Audio + Feedback | 2-3 |
| Phase 3: Enemies | 3-4 |
| Phase 4: Level Design | 2-3 |
| Phase 5: Polish + Distribution | 2-4 |
| **Total** | **12-18** (was 13-21 in Rust) |

### Grand Total
| | Original (Rust) | With Lua |
|--|-----------------|----------|
| Split/Setup | 9.5-14 | 14.5-22 |
| Post-Split Engine | 24.75-28.25 | 26.25-33.5 |
| Game | 13-21 | 12-18 |
| **Total** | **47.25-63.25** | **52.75-73.5** |
| **Critical path** | ~22-29 | ~28-37 |

The increase is concentrated in the scripting layer. Game development starts earlier (as soon as scripting runtime works) and iterates faster (hot-reload vs recompile).

## What Gets Better with Lua

1. **Hot-reload** -- edit Lua, save, see changes immediately. No compile step for game logic.
2. **Modding** -- players can modify Lua scripts. Comes nearly free.
3. **Clear separation** -- language boundary enforces engine/game split. Can't accidentally couple them.
4. **Lower barrier** -- Lua is simpler than Rust for gameplay programming.
5. **Triggers** -- direct Lua callbacks instead of string-based event dispatch.
6. **State machines** -- native Lua tables/coroutines. No Rust FSM framework needed.

## What Gets Harder

1. **Binding maintenance** -- every new engine API needs a Lua wrapper.
2. **Debugging** -- Lua stack traces instead of Rust compiler errors. Need good error formatting.
3. **Type safety** -- dynamic typing in Lua. Runtime errors instead of compile-time.
4. **Performance boundary** -- must be careful about what crosses the Rust/Lua bridge in hot paths.
5. **Two-language codebase** -- developers need to know both Rust and Lua.

## Sources
- [Agent Analysis Report](./agent-analysis-report.md)
- [Agent Split Report](./agent-split-report.md)
- [Agent Scripting Report](./agent-scripting-report.md)
- [Agent Amendments Report](./agent-amendments-report.md)
- [Agent Game Report](./agent-game-report.md)
