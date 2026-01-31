# Agent P4: Phase 4 Lua Amendment Merge Report

**Date:** 2026-01-31
**Task:** Merge Lua scripting amendments into Phase 4 Level Design plan
**Files modified:** `scratchpad/plans/engine-roadmap-2026/post-split-phase-4-level-design.md`

## What was done

Rewrote the Phase 4 Level Design Pipeline plan to fully integrate all Lua scripting amendments from `lua-phase-amendments.md`. The result is a self-contained document that reads cohesively without needing the amendments file.

## Key changes made

### Header and metadata
- Added "Updated 2026-01-31" note with integration reference
- Updated engine effort from 4.5 to 4.75-5.0 sessions with delta breakdown
- Added `rust4d_scripting` as a prerequisite

### Section 1 (Overview)
- Updated game repo description to reference Lua scripts instead of Rust code
- Added "Lua bindings" and "Lua callback integration" to engine-provides list
- Noted door/elevator/pickup are now "trivial in Lua"

### Section 2 (Engine vs Game Boundary) -- major rewrite
- Engine responsibility table: added Tween Lua API and Trigger Lua API rows with `rust4d_scripting` crate
- Game responsibility table: rewritten for Lua scripts (a few lines of Lua vs Rust structs/FSMs)
- Replaced "The GameEvent Escape Hatch" subsection with "The Lua Callback Model" -- explains old approach vs new, includes the door Lua example, documents what gets removed (GameEvent pattern, event handler pattern) and what gets simpler (trigger system, door/elevator, pickup)

### Section 3 (Shapes)
- Added "Lua impact: None" note. No other changes -- pure geometry is unaffected.

### Section 4 (RON Preview)
- Added "Lua impact: None" note. Standalone Rust binary, unaffected.

### Section 5 (Tweens) -- significant additions
- Updated session estimate: 0.5 -> 0.65 (added 0.15 for Lua bindings)
- Added `rust4d_scripting` to dependencies and crate list
- Added full "Lua Tween API" subsection with API surface and elevator cycling Lua example
- Updated engine vs game boundary table with Lua tween API row and Lua script rows
- Added `rust4d_scripting/src/tween_api.rs` to files modified
- Added 3 Lua integration test items to verification criteria

### Section 6 (Triggers) -- major rewrite, the biggest change
- Updated session estimate with breakdown: 0.75 (simplified Rust) + 0.35 (Lua bindings) = 1.1
- RON example: replaced `GameEvent("pickup_health_large")` with `Callback("on_health_pickup")`
- `TriggerAction` enum: replaced `GameEvent(String)` with `Callback(String)` throughout
- Added explanation note about the replacement
- `TriggerRuntime::update()` now takes `lua_ctx: &LuaContext` instead of `event_bus: &mut EventBus`
- Runtime step 4: changed from "fires named event on EventBus" to "calls named Lua function"
- Added full "Lua Trigger Callback System" subsection with Rust dispatch code and 3 Lua callback examples (health pickup, door, secret area)
- Added "Lua Trigger Registration API" subsection with `triggers:register/on_enter/on_exit`
- Added dedicated error handling section for Lua callbacks (missing function, runtime error)
- Added `rust4d_scripting/src/trigger_api.rs` to files modified
- Added 5 Lua integration test items to verification criteria

### Section 7 (4D-Specific)
- Updated W-portal to use `Callback("shift_w")` option alongside TweenPosition
- Updated RON example to use `Callback("on_w_portal")` instead of direct TweenPosition
- Added corresponding Lua example with visual effect + audio

### Section 8 (Session Estimates) -- rewritten
- Expanded table with Wave 3+ (Lua tween), Wave 5+ (Lua trigger callback + registration) rows
- Trigger runtime reduced from 0.5 to 0.25 sessions (simpler without GameEvent)
- Added comparison table showing original vs amended per sub-phase with deltas

### Section 9 (Dependencies)
- Added new "Dependencies on Scripting Phase" subsection
- Removed P1 event system / EventBus dependency from P1 section (no longer needed)
- Updated P5 dependencies to include Lua console access to trigger/tween APIs

### Section 10 (Parallelization)
- Updated ASCII diagram to include Lua binding waves and `rust4d_scripting` dependency
- Updated key observations to mention Lua binding waves as sub-tasks

### Section 11 (Verification) -- expanded
- Sub-Phase C: added 3 Lua integration test checkboxes
- Sub-Phase D: replaced `GameEvent` test with `Callback` test, added 6 Lua integration test checkboxes

### Section 12 (Cross-Phase Coordination)
- P1 section: noted P4 no longer needs EventBus, simplified dependency
- Added new "For Scripting Phase" subsection
- P5 section: added Lua console access to APIs

### Section 13 (Open Questions)
- Removed old question 5 about string-named events (no longer relevant)
- Added new question 5: `Callback` inline Lua vs function names only
- Added new question 6: whether to keep `GameEvent` alongside `Callback` (recommended: no)

### Section 14 (Game Repo) -- major rewrite
- Replaced Rust code examples with Lua examples throughout
- Door/elevator: full Lua example with key-locked door variant and elevator cycling
- Pickup: Lua callback examples for health and ammo
- Level scripting: all examples now use `Callback("func_name")` pattern
- Updated game repo effort table: reduced from 1.5-2.5 to 0.5-0.75 sessions

## What was preserved

All existing Rust implementation details were preserved exactly:
- Shape types (Hyperprism4D, Hypersphere4D) -- unchanged
- RON preview tool -- unchanged
- Interpolatable trait, easing functions, Tween struct, TweenManager -- unchanged
- TriggerDef, TriggerZone, TriggerRepeat -- unchanged
- All code examples for Rust internals -- unchanged
- Shape priority tiers, crate placement, design decisions -- unchanged

## Observations

The Lua migration is a net positive for Phase 4. The trigger system becomes cleaner (direct Lua calls vs string event dispatch), and the game-side implementation effort drops significantly. The engine effort increase is modest (+0.25-0.5 sessions) because the Lua bindings are thin wrappers over existing Rust APIs. The biggest single piece of new work is the Lua trigger callback integration (~0.25-0.5 session), but it replaces the more complex GameEvent dispatch pattern.
