# Agent P3: Phase 3 Lua Amendment Merge Report

**Date**: 2026-01-31
**Agent**: P3
**Task**: Merge Lua scripting amendments into the Phase 3 (Enemies & AI) plan

## Summary

Merged all Lua scripting amendments from `lua-phase-amendments.md` (Phase 3 section) into `post-split-phase-3-enemies-ai.md`. The result is a self-contained, cohesive document that no longer requires the amendments file for context.

## Key Changes Made

### Header and Overview
- Added update note with date and summary of changes
- Updated overview to mention Lua bindings and FSM removal
- Updated total session estimate from 4.0 to 3.75-4.5
- Added `rust4d_scripting` crate as a prerequisite
- Updated "What This Phase Delivers" to include Lua bindings and mark FSM as removed

### Engine vs Game Boundary (Section 2)
- Added Lua binding rows to the "Engine Provides" table (sprite API, spatial query wrappers, animation control)
- Rewrote "Game Builds" table to reflect Lua scripts instead of Rust structs
- Added new sub-section: "REMOVED: `StateMachine<S>` in `rust4d_game`" with Lua FSM example code
- Added new sub-section: "What Was Game-Side Rust That Now Needs Lua Bindings" (the boundary shift table)
- Added new sub-section: "What Gets Simpler With Lua"
- Added new sub-section: "What Gets Removed From Engine Scope"
- Updated boundary principle to reference Lua scripts

### Sub-Phase A: Sprites (Section 3)
- Added "Lua Sprite API Bindings" sub-section (~0.25 session)
- Added "Lua animation control" details (~0.1 session)
- Added Lua binding row to session breakdown table
- Added `lua_sprite_api.rs` to new files table
- Added `rust4d_scripting/src/lib.rs` to modified files

### Sub-Phase B: Spatial Queries (Section 4)
- Added "Lua Spatial Query Wrappers" sub-section (~0.15 session)
- Added `lua_spatial_api.rs` to new files table
- Added `rust4d_scripting/src/lib.rs` to modified files

### Sub-Phase C: FSM (Section 5)
- Rewrote entirely as "REMOVED" section
- Included rationale for removal
- Listed what was planned but is no longer being implemented
- Added comprehensive Lua replacement example showing EnemyAI class with metatables

### Sub-Phase D: Area Damage (Section 6)
- Replaced Rust game-side usage example with Lua equivalent

### Particle System (Section 7)
- Replaced Rust game-side effect presets with Lua `particles:define()` examples
- Updated references to mention Lua scripts

### Session Estimates Summary (Section 8)
- Added FSM as "REMOVED" row
- Added three new Lua binding rows with session estimates
- Updated total from 4.0 to 3.75-4.5 with explanation
- Updated game-side work table to reference Lua

### Dependencies (Section 9)
- Removed reference to FSM in P1 mitigation
- Added new "On Scripting Phase" dependency section

### Parallelization (Section 10)
- Added Wave 2.5 for Lua bindings (after scripting phase + Wave 2 Rust APIs)
- Noted FSM removal frees up time in Agent/Session C
- Updated Wave 3 to reference Lua scripts

### Verification Criteria (Section 12)
- Added "Lua Sprite Integration Tests" sub-section (4 items)
- Added "Lua Spatial Query Integration Tests" sub-section (4 items)
- Struck through FSM tests and replaced with Lua FSM verification
- Added "Lua Binding General Tests" sub-section (3 items)

### File Inventory (Section 14)
- Added `lua_sprite_api.rs` and `lua_spatial_api.rs` to new files
- Added "Removed Files" table showing `fsm.rs` removal
- Removed `fsm.rs` from new files
- Removed `rust4d_game/src/lib.rs` FSM export from modified files
- Added `rust4d_scripting/src/lib.rs` to modified files

### Game-Side Reference (Section 15)
- Rewrote engine-to-game mapping table with Lua API calls
- Added FSM row showing Lua replaces `StateMachine<S>`
- Replaced enemy type descriptions with Lua data table examples

## What Was Preserved

All Rust implementation details were preserved exactly:
- Complete `SpriteSheet`, `SpriteInstance`, `SpriteBatch` API
- Complete `SpritePipeline` API
- Billboard WGSL shader code
- W-distance visibility rules table
- Complete `SpatialQueryResult`, `PhysicsWorld::query_sphere()`, etc. API
- Complete `AreaEffectHit`, `PhysicsWorld::query_area_effect()`, `apply_impulse()` API
- Complete `ParticleEmitterConfig`, `ParticleEmitter`, `ParticleSystem` API
- Render pass ordering diagram
- 4D-specific challenges and solutions
- All dependency tables
