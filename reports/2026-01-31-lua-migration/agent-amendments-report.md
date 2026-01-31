# Agent Amendments -- Completion Report

**Date**: 2026-01-31
**Agent**: Amendments Agent (Lua Migration Swarm)
**Output**: `scratchpad/plans/engine-roadmap-2026/lua-phase-amendments.md`

---

## Task Summary

Read all five post-split engine phase plans (P1-P5) and produced a comprehensive amendment document describing what changes with the Lua scripting approach for each phase.

## Documents Read

1. `scratchpad/reports/2026-01-31-lua-migration/hive-mind.md` -- swarm context and coordination
2. `scratchpad/plans/engine-roadmap-2026/post-split-phase-1-combat-core.md` (874 lines)
3. `scratchpad/plans/engine-roadmap-2026/post-split-phase-2-weapons-feedback.md` (1092 lines)
4. `scratchpad/plans/engine-roadmap-2026/post-split-phase-3-enemies-ai.md` (962 lines)
5. `scratchpad/plans/engine-roadmap-2026/post-split-phase-4-level-design.md` (1016 lines)
6. `scratchpad/plans/engine-roadmap-2026/post-split-phase-5-editor-polish.md` (798 lines)

## Key Findings

### 1. Rust implementations are completely unchanged across all phases
Every phase's internal engine code (physics, rendering, audio, math) is identical with or without Lua. The 4D engine core -- raycasting, collision detection, sprite rendering, particle systems, tween logic, shape types, lighting -- is unaffected. Lua adds a binding layer on top; it does not modify the engine internals.

### 2. Total session impact is moderate: +2.5-6.2 sessions across P1-P5
With minimal scoping, the delta is around +2.5 sessions total. The main contributors:
- **P2 HUD API for Lua** (+0.5-1.0): egui's Rust API does not translate to Lua; a simplified `hud:draw_text/draw_bar/draw_rect` API is needed
- **P5 Editor script panel + Lua console** (+0.5-2.5): New editor features for script editing and runtime evaluation
- **Thin binding wrappers per phase** (+0.25-0.5 each): Each phase needs Lua function registrations for its APIs

### 3. FSM framework becomes unnecessary (P3)
`StateMachine<S>` in `rust4d_game` was designed for Rust game code. Lua natively handles FSM patterns with tables and functions (or coroutines). This saves ~0.25 sessions and removes a Rust type from the engine scope.

### 4. Trigger system becomes dramatically more powerful (P4)
The `GameEvent(String)` escape hatch was an awkward workaround for Rust's lack of late binding. With Lua, triggers call Lua functions directly: `TriggerAction::Callback("on_door_open")`. The entire string-to-event dispatch layer disappears. This is the single biggest architectural improvement from the Lua migration for gameplay scripting.

### 5. HUD API is the most significant new engine work (P2)
egui's immediate-mode Rust API cannot be directly exposed to Lua in a practical way. The engine needs a new simplified HUD drawing API (`hud:draw_text`, `hud:draw_bar`, `hud:draw_rect`, `hud:draw_image`). This is roughly 0.5-1.0 sessions of work not present in the original plan.

### 6. Editor gains two new panel types (P5)
- **Script panel**: At minimum, an error log showing Lua errors with line numbers. At maximum, a syntax-highlighted text editor with hot-reload.
- **Lua console**: A REPL for runtime evaluation. Essential for debugging Lua-scripted games.

Both can be scoped from minimal (0.5 sessions total) to full-featured (2.5+ sessions).

## Decisions Made

1. **Recommended HUD API approach**: Simplified drawing commands (`hud:draw_text`, etc.) rather than exposing full egui Context to Lua. More maintainable, adequate for game HUD needs.

2. **Recommended trigger approach**: `TriggerAction::Callback(String)` replaces `GameEvent(String)`. Keep `TweenPosition`, `DespawnSelf`, `PlaySound` as engine-level actions; game-specific actions use Lua callbacks.

3. **Recommended FSM approach**: Remove `StateMachine<S>` from `rust4d_game`. Lua handles this natively. Provide an example Lua FSM pattern in documentation instead.

4. **Recommended editor scoping**: Start with minimal script features (error log + basic Lua console = 0.5 sessions) and add full-featured versions in a later phase if needed.

5. **Binding pattern**: Expose Rust types as Lua tables (not userdata) for read access, use registered functions for write access. Strings for names/identifiers (Lua-friendly). Sound/texture/particle presets referenced by string name with engine-managed handle mapping.

## Open Questions for Other Agents

1. **For Agent Scripting**: How are Lua bindings organized? One registration function per crate (e.g., `rust4d_physics::register_lua_bindings(lua)`)? Or a central registry in `rust4d_scripting`? The per-phase binding work depends on this architecture.

2. **For Agent Split**: Does `rust4d_game` still make sense as a crate? Many of its planned types (EventBus, GameEvent, FSM) become unnecessary with Lua. It might shrink to just ScreenShake, TimedEffect, and TweenManager -- or those could live elsewhere.

3. **For Agent Game**: The Lua game roadmap should reference these amended engine APIs. The binding signatures described here (e.g., `world:raycast(origin, dir, max_dist, mask)`) should be treated as the canonical Lua API design.

## Time Spent

Single session. The five phase plans totaled ~4700 lines of detailed technical content. Analysis was straightforward because the plans were well-structured with clear engine/game boundaries.
