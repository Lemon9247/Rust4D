# Agent Media: Phase 2 (Weapons & Feedback) Planning Document

**Agent**: Media
**Date**: 2026-01-31
**Task**: Create detailed planning document for post-split Phase 2 (Weapons & Feedback)

## Output

Created `scratchpad/plans/engine-roadmap-2026/post-split-phase-2-weapons-feedback.md`

## What I Did

Synthesized Agent P2's detailed report into a working planning document, incorporating:

1. **Full API designs** preserved from Agent P2 for all four subsystems:
   - `AudioEngine4D` and all methods (rust4d_audio crate)
   - `OverlayRenderer` for egui-wgpu integration (rust4d_render)
   - `ParticleSystem`, `ParticleEmitterConfig`, billboard shader design (rust4d_render)
   - `ScreenShake`, `TimedEffect` helpers (rust4d_game)

2. **Cross-phase coordination** from the hive-mind file:
   - Render pass ordering confirmed with P3: geometry -> sprites -> particles -> egui overlay -> editor overlay
   - Particle system shared between P2 (weapons) and P3 (enemies) via `spawn_burst()` API
   - Audio triggers come from game events, not engine collision events (coordinated with P1)
   - egui integration front-loads dependency needed by P5 editor

3. **Engine/game boundary** from the split plan:
   - Weapon system is 100% game-side
   - Screen shake in rust4d_game (camera offset, not post-processing)
   - Damage flash via egui overlay (no post-processing pipeline needed)
   - All game-side usage examples preserved for reference

4. **Parallelization plan**: Three-agent parallel wave for audio/egui/particles, followed by integration wave. Critical path: 2-2.5 sessions if fully parallelized.

5. **Complete verification criteria** for all four sub-phases.

## Key Details Preserved

- Kira vs rodio rationale (6 reasons)
- 4D spatial audio projection design (4D Euclidean distance -> kira 3D spatial)
- W-distance filtering behavior and open question about low-pass vs volume-only
- 3D-not-4D particles rationale (visual effects in sliced space, not physical 4D objects)
- CPU-not-GPU particle rationale (hundreds not millions)
- Billboard shader WGSL structure
- Depth buffer sharing details (TEXTURE_BINDING already enabled)
- winit version compatibility risk for egui-winit
- All file paths and crate organization details
- Game-side example code for HUD, audio, effects, and particle presets

## Document Structure

14 sections covering overview, boundary, all four sub-phases with full detail, render pipeline diagram, crate organization, session estimates, dependencies, parallelization, verification, risks, and design rationale.
