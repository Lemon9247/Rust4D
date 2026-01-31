# Agent P5: Phase 5 Lua Amendment Merge Report

**Date**: 2026-01-31
**Task**: Merge Lua scripting amendments into Phase 5 (Editor & Polish) plan
**Status**: Complete

---

## What I Did

Rewrote `/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/post-split-phase-5-editor-polish.md` to integrate all Lua migration amendments from the "Phase 5: Editor & Polish -- Amendments" section of `lua-phase-amendments.md`.

## Merge Approach

Rather than appending Lua content at the end, I wove amendments into the existing document structure:

1. **Header**: Updated with Lua-integrated status note, revised session estimates (10.85-13.75 minimal, up to 11.5-15 full-featured), added `rust4d_scripting` as a prerequisite.

2. **Overview**: Added paragraph explaining the Lua migration's impact on Phase 5, with the +0.85-1.25 session (minimal) to +1.35-2.85 session (full) delta.

3. **Engine vs Game Boundary**: Expanded the component table to include script error panel, Lua console, and Lua bindings. Added three new subsections:
   - "Lua Boundary Shift" table showing what was game-side Rust now needing Lua bindings
   - "What Gets Simpler with Lua" (input rebinding UI, game-specific editor panels, pause menu)
   - "What Gets Removed or Reduced" (editor extension API, complex rebinding UI)

4. **Sub-Phase A (Textures)**: Added "Lua Texture/Material API" subsection (+0.1 session) with `textures:load()` and `entity:set_material()`. Updated task breakdown table with 5.3L task. Updated sub-phase total to 1.6-2.6.

5. **Sub-Phase B (Lighting)**: Added "Lua Lighting API" subsection (+0.1 session) with `entity:add_light()` and `entity:set_light()`. Updated task breakdown with 5.1L task. Updated sub-phase total to 2.1.

6. **Sub-Phase C (Input Rebinding)**: Reworked "Game-Side Responsibilities" to reflect Lua scripting instead of compiled Rust. Added "Lua Input Rebinding API" subsection (+0.15 session) with `input:bind()`, `input:save()`, etc. Updated task breakdown with 5.5L task. Updated sub-phase total to 0.65.

7. **Sub-Phase D (Editor Framework)**:
   - Added `script_panel.rs` and `lua_console.rs` to crate structure
   - Added `rust4d_scripting` to dependencies
   - Extended `EditorHost` trait with `lua_runtime()` / `lua_runtime_mut()` methods returning `Option`
   - Added "Wave 5: Lua Development Tools" to MVE features
   - Added full subsections for Script Error Panel (0.25-1.5 sessions) and Lua Console Panel (0.25-1.0 session) with minimal/full-featured scoping
   - Updated "What the Game Builds On Top" for Lua workflow
   - Added tasks 5.10 and 5.11 to breakdown. Updated sub-phase total to 6.5-8.5.

8. **Complete Session Estimates**: Added 5.1L, 5.3L, 5.5L, 5.10, 5.11 to the master table. Added comparison table (original / pre-Lua / post-Lua minimal / post-Lua full). Updated critical path analysis.

9. **Dependencies**: Added `rust4d_scripting` as hard dependency. Added P1-P4 Lua bindings as soft dependency. Added Scripting Phase to the dependency chain diagram. Added scripting coordination note.

10. **Parallelization**: Updated Wave 1 agent descriptions to include Lua API tasks. Updated Wave 2 to include script panel and console. Updated internal dependency graphs.

11. **Verification**: Added Lua integration test sections to each sub-phase (textures, lighting, input, editor).

12. **Risks**: Updated editor scope creep risk to note Lua increases it. Added new "Lua Console Security" low risk.

13. **APIs Delivered / Game Builds**: Added Lua-specific APIs and updated game feature table for Lua workflow.

14. **Files**: Added `script_panel.rs`, `lua_console.rs`, and `rust4d_scripting/src/bindings/` to file lists.

15. **Summary**: Rewritten to reflect the Lua-integrated version of Phase 5.

## What I Preserved

All original Rust implementation details remain untouched:
- Triplanar mapping shader code and design rationale
- PointLight4D component, GPU types, shader code
- W-distance attenuation algorithm
- Shadow mapping approach and implementation details
- InputMap API (rebind, unbind, conflicts, TOML persistence)
- EditorApp/EditorHost design and API
- All egui integration details
- W-slice navigation (slider + thumbnails)
- RON scene operations
- All risk assessments (with Lua-specific additions)
- Cross-phase coordination notes

## Key Decisions

- Used the **minimal Lua scoping** (error log + basic console = +0.5 sessions) as the primary estimate, with full-featured as an explicit Phase 6 deferral
- Made `EditorHost::lua_runtime()` return `Option` so the editor works without Lua for pure engine tests
- Kept the original task numbering (5.1-5.9) and added Lua tasks as 5.1L/5.3L/5.5L/5.10/5.11 to make the delta from the original plan obvious
- The document is fully self-contained -- no need to reference `lua-phase-amendments.md`
