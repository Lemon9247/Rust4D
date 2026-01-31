# Hive Mind: Lua Migration Analysis

## Task Overview
Adapt the Rust4D roadmap from "game in Rust" to "game in Lua". The engine becomes a Lua-scriptable runtime: it provides a binary that loads Lua scripts + RON scenes + assets, exposes comprehensive Lua bindings to all engine APIs, and supports hot-reload of scripts. The game repo becomes Lua scripts + data, not a Rust binary.

## The Architectural Shift
- **BEFORE**: Game repo is a Rust binary (`Cargo.toml` + `src/`) depending on engine crates via git URL. All game logic (weapons, enemies, AI, health) is compiled Rust.
- **AFTER**: Engine provides a `rust4d_scripting` crate with Lua runtime (via mlua). Engine ships a binary/launcher that loads game scripts. Game repo contains Lua scripts + RON scenes + TOML config + assets. Hot-reload enables edit-save-test workflow without recompilation.

## Key Implications
1. **Engine/game split changes fundamentally**: No game Cargo.toml, no compiled game code. Game is data + scripts.
2. **Engine must expose much more API surface**: Everything that was "game implements in Rust" now needs Lua bindings.
3. **rust4d_game crate changes role**: Instead of Rust types the game imports, it becomes Rust implementations exposed TO Lua (CharacterController4D, FSM, events all need Lua wrappers).
4. **Declarative triggers get simpler**: GameEvent(String) escape hatch becomes "call this Lua function", which is more powerful.
5. **Editor gains script editing**: P5 editor needs a script panel with syntax highlighting, error display, hot-reload button.
6. **New engine work**: Lua runtime, binding generation, script lifecycle, error handling, hot-reload, debugging support.

## Agents
1. **Agent Analysis** - Reads all current plans, produces `lua-migration-analysis.md` identifying what changes per document
2. **Agent Split** - Rewrites `engine-game-split.md` for the scripting approach
3. **Agent Scripting** - Creates `post-split-phase-scripting.md` -- the new Lua scripting engine phase
4. **Agent Amendments** - Reads post-split phases P1-P5, produces amendment notes for each
5. **Agent Game** - Creates `game-roadmap-lua.md` -- the Lua-based game roadmap

## Coordination Notes
- Agent Analysis should run first or in parallel -- its output informs other agents
- Agent Split and Agent Scripting can run in parallel
- Agent Amendments depends on understanding the Lua approach (can read hive-mind for context)
- Agent Game depends on understanding what the engine exposes (can read hive-mind for context)
- All agents should write to `scratchpad/plans/engine-roadmap-2026/` for plan docs
- All agents should write completion reports to this folder

## Key Design Questions
1. **mlua vs rlua**: mlua is more actively maintained, supports Lua 5.4 + LuaJIT, has better async support. Likely winner.
2. **How scripts interact with ECS**: Scripts should be able to query/modify components, spawn/despawn entities, register systems. Need to decide on the binding model (userdata vs tables).
3. **Script lifecycle**: When do scripts run? Per-frame update? Event callbacks? Both?
4. **Hot-reload granularity**: Reload individual scripts? All scripts? What about script state?
5. **Performance boundary**: What stays in Rust (physics step, rendering, collision detection) vs what's scriptable (game logic, AI, events)?

## Status
- [ ] Agent Analysis: Pending
- [x] Agent Split: Complete -- rewrote engine-game-split.md for Lua approach (6 phases, 14.5-22 sessions)
- [ ] Agent Scripting: Pending
- [x] Agent Amendments: Complete -- produced lua-phase-amendments.md with per-phase analysis (P1-P5)
- [ ] Agent Game: Pending
- [ ] Final synthesis: Pending

## Reports Generated
- `agent-split-report.md` - Split agent completion report
- `agent-amendments-report.md` - Amendments agent completion report

## Key Findings
- **Split agent**: Total effort increases from 9.5-14 to 14.5-22 sessions. The Lua scripting phase (4-6 sessions) is the main new work. Game repo setup is simpler (1-2 sessions vs 1-2 sessions for Rust). Phase 3 (Lua Scripting Integration) has internal parallelism -- bindings for different modules are independent.
- **Split agent**: Answered key design questions: mlua with Lua 5.4, callbacks for lifecycle, `persist` table + module invalidation for hot-reload, string-based ECS queries from Lua, `ScriptComponent` wrapper for Lua-defined components in hecs.
- **Split agent**: Old Phase 3 (Pluggable Scenes) merged into Phase 2 since scene helpers fit naturally in the rust4d_game crate extraction.
- **Amendments agent**: Total P1-P5 session impact is +2.5-6.2 sessions (with minimal scoping: +2.5). All Rust implementations unchanged; Lua adds binding layer on top.
- **Amendments agent**: Biggest new item is HUD API for Lua (P2, +0.5-1.0 sessions) -- egui's Rust API cannot be directly exposed to Lua.
- **Amendments agent**: FSM framework (P3) becomes unnecessary -- Lua handles state machines natively. Saves ~0.25 sessions.
- **Amendments agent**: Trigger system (P4) dramatically improved -- `GameEvent(String)` replaced by `TriggerAction::Callback(String)` calling Lua functions directly.
- **Amendments agent**: Editor (P5) needs script editing panel + Lua console. Minimal scope: +0.5 sessions; full scope: +2.5 sessions.
- **Amendments agent**: Open question for Agent Split: does `rust4d_game` still make sense? Many planned types (EventBus, GameEvent, FSM) become unnecessary with Lua.
