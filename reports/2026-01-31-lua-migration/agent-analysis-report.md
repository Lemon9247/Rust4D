# Agent Report: Lua Migration Analysis

**Agent**: Analysis Agent
**Swarm**: Lua Migration Swarm
**Date**: 2026-01-31
**Duration**: Single session
**Status**: Complete

---

## Mission

Read all nine current roadmap plan documents and produce a comprehensive analysis of what changes when the game shifts from "compiled Rust binary" to "Lua scripts + assets."

## Documents Analyzed

1. `engine-game-split.md` -- Current split plan (Rust binary game)
2. `split-phase-0-foundation.md` -- Pre-split foundation
3. `split-phases-1-5.md` -- Split plan summary
4. `post-split-phase-1-combat-core.md` -- Combat core
5. `post-split-phase-2-weapons-feedback.md` -- Weapons & feedback
6. `post-split-phase-3-enemies-ai.md` -- Enemies & AI
7. `post-split-phase-4-level-design.md` -- Level design
8. `post-split-phase-5-editor-polish.md` -- Editor & polish
9. `game-roadmap-summary.md` -- Current game roadmap (Rust-based)

## Output

Wrote comprehensive analysis to:
`scratchpad/plans/engine-roadmap-2026/lua-migration-analysis.md`

## Key Findings

### 1. Engine work is 95% unchanged
Physics, rendering, math, audio, particles, sprites, editor, shapes -- all internal engine work stays exactly the same. The Lua migration does not affect engine internals.

### 2. The binding layer is the biggest new work item
~80-100 engine functions/methods need Lua exposure. Estimated at 5-7 sessions total, spread across the roadmap. This is the primary cost of the migration.

### 3. Two-repo architecture is eliminated
Phase 4 (Create Game Repository) of the split plan is removed entirely. The `.cargo/config.toml` hack, git URL dependencies, and cross-repo coordination friction disappear.

### 4. Game-side Rust work converts to Lua at ~0.65x ratio
13-21 sessions of Rust game code become ~8.5-14 sessions of Lua. Lua is faster to write for game logic (simpler syntax, no borrow checker, dynamic typing for game components).

### 5. The `GameEvent(String)` trigger pattern is ideal for Lua
The original plan struggled with bridging typed Rust events and string-named game events. Lua naturally handles string-dispatched events -- this is actually simpler than the Rust approach.

### 6. A new `LuaHud` API is needed
Exposing raw egui to Lua is impractical (closures, immediate-mode patterns). The engine needs a simplified HUD API: `hud:text()`, `hud:bar()`, `hud:rect()`, `hud:button()`, `hud:slider()`.

### 7. FSM can be pure Lua
The engine's `StateMachine<S>` is ~30 lines of Rust with a generic type parameter. Reimplementing this in pure Lua is simpler than creating bindings. Ship a Lua FSM utility with the engine.

### 8. Performance is not a concern for the target game
With <100 entities, per-frame Lua overhead for AI, events, and HUD is in the microsecond range. All heavy computation (physics, rendering, particles) stays in Rust.

### 9. Net session impact is roughly neutral
Original total: 44-60 sessions. Revised total: 42.5-56 sessions. The migration shifts work between engine and game but does not significantly change the total.

## Risks Identified

1. **Binding complexity (HIGH)** -- 80-100 functions is substantial, mitigated by incremental delivery
2. **HUD API design (MEDIUM)** -- new abstraction with no precedent in the plans
3. **Debugging across Lua/Rust boundary (MEDIUM)** -- standard for scripted engines
4. **Performance (LOW)** -- negligible for target entity counts
5. **Hot-reload stability (LOW-MEDIUM)** -- solvable with clean reload boundaries

## Recommendations for Other Agents

- **Architecture Agent**: Focus on the Lua runtime crate structure, game loop hooks, and the boundary between "engine drives" and "Lua drives"
- **Binding Agent**: Prioritize Tier 1-2 bindings (Vec4, Transform4D, physics) before Tier 3+ (audio, particles, HUD)
- **Plan Agent**: Phase 4 (Create Game Repo) should be replaced by "Lua Runtime Integration" phase; Phase 5 (Engine Cleanup) should include `EditorHost` implementation for the runtime
