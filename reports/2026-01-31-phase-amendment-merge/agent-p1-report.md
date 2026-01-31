# Agent P1 Report: Phase 1 Combat Core -- Lua Amendment Merge

**Date**: 2026-01-31
**Task**: Merge Lua scripting amendments into the Phase 1 Combat Core plan
**File**: `scratchpad/plans/engine-roadmap-2026/post-split-phase-1-combat-core.md`

## What I Did

Rewrote the Phase 1 Combat Core plan to integrate all Lua scripting amendments from `lua-phase-amendments.md` into a single cohesive document. The merged document stands alone without needing the amendments file.

## Key Changes Made

### Structural changes
- Added new **Sub-Phase C: Lua Bindings for Combat Core APIs** (Section 5) with full design details for raycasting wrappers, collision event dispatch, and trigger Lua callbacks
- Updated section numbering: Session Estimates moved to Section 6, Dependencies to Section 7, Parallelization to Section 8, Verification to Section 9, Cross-Phase Dependencies to Section 10, Open Questions to Section 11
- Added Wave 2 to the Parallelization section for the sequential Lua binding work

### Header and metadata
- Updated header with "Updated 2026-01-31: Integrated Lua scripting amendments" note
- Engine estimate updated from 1.75 to 2.25-2.5 sessions
- Game estimate rewritten to reflect Lua scripts replacing compiled Rust
- Added `rust4d_scripting` as a dependency in the "Depends On" line

### Overview section
- Rewrote event system bullet to reference Lua callbacks instead of Rust EventBus
- Rewrote trigger callbacks bullet to reference Lua function invocation
- Added "What the Lua architecture changes" paragraph
- Added "What gets simpler with Lua" section (trigger system, EventBus removal)
- Added "What is removed from engine scope" section (EventBus, GameEvent enum, StateMachine dependency)

### Engine vs Game Boundary (Section 2)
- Split "Engine Provides" into two subsections: Rust implementation and Lua bindings
- Added "Lua Binding Boundary Details" table showing the Rust-to-Lua migration
- Updated "Game Implements" to specify Lua scripts instead of Rust code
- Removed references to Rust-side EventBus and GameEvent from game-side scope

### Sub-Phase B (Section 4)
- Added "With Lua" note to the Drain API design section explaining engine-managed dispatch
- Updated game-side usage pattern to show Lua callback flow instead of manual drain

### Session Estimates (Section 6)
- Added Sub-Phase C row to the summary table
- Added comparison table (Original vs Amended vs Delta)
- Updated game-side context table to reference Lua instead of Rust types

### Dependencies (Section 7)
- Added "On Lua Scripting Infrastructure" subsection with full dependency list
- Clarified that Sub-Phases A and B have no scripting dependency
- Added mlua dependency for Sub-Phase C

### Verification (Section 9)
- Added complete Sub-Phase C verification checklist (14 items)
- Updated Integration section to reference Lua callback dispatch

### Cross-Phase Dependencies (Section 10)
- Updated all phase references to use Lua API calls instead of Rust method calls
- Added Scripting Phase dependency subsection

### Open Questions (Section 11)
- Added questions 6-8 covering Lua-specific concerns: callback dispatch overhead, single vs per-type callbacks, and error recovery

## What I Preserved

All existing Rust implementation details are completely untouched:
- All code blocks in Sub-Phase A (Ray4D, ray-shape intersections, PhysicsWorld::raycast)
- All code blocks in Sub-Phase B (CollisionEvent, drain API, trigger detection, enter/exit tracking)
- All file lists for Sub-Phases A and B
- All test requirements for Sub-Phases A and B
- All session estimates for Sub-Phases A and B (1.0 and 0.75 respectively)
- Bug analysis for the trigger system
- Vec4 utility additions and sphere_vs_sphere visibility fix

## Decision Notes

- Chose to make Sub-Phase C a full separate section rather than appending Lua binding subsections within A and B. Reasoning: the Lua work has different dependencies (requires rust4d_scripting), different parallelization characteristics (must be sequential after A+B), and is conceptually a binding layer rather than core implementation.
- Kept the drain API design in Sub-Phase B because it is still the Rust-level mechanism. Added a "With Lua" note explaining how the engine manages the drain-dispatch cycle internally.
- Updated cross-phase references to use Lua API syntax (e.g., `world:raycast()`) to make it clear these are the APIs game code will actually call.
