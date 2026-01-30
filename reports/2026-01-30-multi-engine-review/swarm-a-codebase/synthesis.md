# Swarm A Synthesis: Codebase State Review
**Date**: 2026-01-30
**Agents**: A1 (Math & Physics), A2 (Core & Scene), A3 (Render & Input)

---

## Executive Summary

Rust4D is a well-architected engine with strong foundations in 4D math, physics, and rendering. The codebase has **358 tests across 5 crates**, high code quality (consistent 4/5 ratings), and clean separation of concerns. However, it has **zero gameplay systems** -- no shooting, no enemies, no health, no audio, no HUD. The engine is a solid platform for building a game, but the game layer does not exist yet.

---

## Crate-by-Crate Summary

### rust4d_math (59 tests)
- **Rotor4** is the crown jewel -- mathematically rigorous geometric algebra rotors with correct sandwich product for all 6 rotation planes. This is the hardest 4D math and it's done right.
- **Vec4** is GPU-ready (`#[repr(C)]`, `Pod`, `Zeroable`) with solid basic operations.
- **Mat4** has the critical `skip_y()` transform for Engine4D-style camera architecture.
- Two renderable shapes: Tesseract4D and Hyperplane4D, both using Kuhn triangulation.
- **Critical gap**: No raycasting primitives at all (no `Ray4D`, no intersection functions).
- Missing utility: `reflect`, `project_onto`, `distance`, `slerp`, `look_at`.

### rust4d_physics (97 tests)
- Three collision primitives (Sphere4D, AABB4D, Plane4D) with four collision pair functions.
- **Collision layer system is FPS-ready**: 7 layers (PLAYER, ENEMY, STATIC, TRIGGER, PROJECTILE, PICKUP) with bitflag masks and preset filters.
- Player physics with kinematic body, gravity, jumping, grounded detection, edge-falling prevention.
- Physics materials with 5 presets and geometric mean friction combining.
- O(n^2) collision detection -- no broadphase/spatial partitioning.
- **Critical gaps**: No raycasting, no trigger event callbacks, no CCD, no character controller (no capsule, no step-up).

### rust4d_core (202 tests, 6142 lines)
- **Entity**: 7 fields, 2 Optional -- clean and lean but not extensible. No ECS/component system.
- **World**: SlotMap with generational keys. O(1) name lookup, O(n) tag lookup. Physics integration syncs transforms.
- **Hierarchy**: Production-quality parent-child with cycle detection, transform accumulation, recursive deletion.
- **Scene pipeline**: Template (RON) -> ActiveScene -> World. Async loading, scene stack, overlays, transitions, validation.
- **Asset cache**: Type-erased `Arc<dyn Any>`, path dedup, dependency tracking, GC, hot reload via timestamp.
- **Critical gaps**: Only 2 shape types (Tesseract, Hyperplane). No event system. No spawning. No ECS extensibility.

### rust4d_render (42 tests)
- Clean two-stage GPU pipeline: compute shader (4D slicing) -> render pipeline (3D with lighting).
- **Camera4D** with Engine4D-style pitch/rotation separation -- exactly right for 4D FPS.
- GPU-driven rendering with indirect draw and atomic counters. Compute workgroup size 64.
- Single-pass only. No shadows, no multi-pass, no post-processing, no textures, no MSAA.
- Back-face culling disabled ("for debugging"). Entire world re-uploaded as single buffer on any change.
- Lookup table divergence between Rust and WGSL (maintenance risk, not runtime bug).

### rust4d_input (37 tests)
- CameraController with builder pattern, exponential mouse smoothing.
- CameraControl trait cleanly abstracts camera operations.
- Hardcoded key bindings (WASD/QE/Space/Shift). No rebinding, no gamepad, no action abstraction.
- Key conflict: E = kata but traditionally E = interact in FPS.
- Diagonal movement not normalized (sqrt(2) speed).

### src/ (main binary)
- 3 systems: Window, Render, Simulation. Clean separation.
- Layered config via figment (default.toml / user.toml / env vars).
- No fixed-timestep update -- physics tied to frame rate.
- No game systems at all.

---

## Cross-Crate Findings

### Strengths
1. **4D math is production-quality**: Rotor4, SkipY, Camera4D architecture. This is the hardest part of a 4D engine and it works correctly.
2. **Clean crate architecture**: Each crate has clear boundaries. Math knows nothing about physics; physics knows nothing about rendering. This enables parallel development.
3. **Test culture is strong**: 358 tests with meaningful coverage. Edge cases tested (edge-falling oscillation, pitch clamping, transform accumulation). MockCamera pattern in tests.
4. **Scene infrastructure is mature**: Async loading, transitions, overlays, validation, hot reload. Ready for game content.
5. **Collision layers designed for FPS**: PLAYER, ENEMY, PROJECTILE, PICKUP, TRIGGER layers with symmetric mask checks.

### Weaknesses
1. **No raycasting anywhere**: The single biggest gap. Cannot shoot, cannot do LOS, cannot pick up items by proximity.
2. **No gameplay systems**: Zero health, weapons, enemies, audio, HUD, events, triggers, AI.
3. **Monolithic Entity**: Not extensible without modifying the struct. Will become painful with 10+ gameplay component types.
4. **Single-pass rendering**: No particle effects, no HUD overlay, no weapon viewmodel, no post-processing.
5. **No fixed timestep**: Physics behavior varies with frame rate. Affects reproducibility and future networking.
6. **Only 2 shape types**: Tesseract and Hyperplane. Cannot build varied level geometry.

### Architecture Quality Ratings

| Crate | Features | Quality | Tests | FPS Ready |
|-------|----------|---------|-------|-----------|
| rust4d_math | 3/5 | 4.5/5 | 4/5 | 2/5 |
| rust4d_physics | 3.5/5 | 4/5 | 4.5/5 | 2.5/5 |
| rust4d_core | 3/5 | 4/5 | 4/5 | 2/5 |
| rust4d_render | 3/5 | 4/5 | 4/5 | 1/5 |
| rust4d_input | 2/5 | 4/5 | 5/5 | 2/5 |
| src/ (binary) | 2/5 | 4/5 | 2/5 | 2/5 |

**Overall FPS readiness: 2/5** -- strong engine foundations, absent gameplay layer.

---

## Key Recommendations

1. **Raycasting first** -- it unblocks shooting, LOS, and proximity detection.
2. **Event system second** -- it unblocks trigger zones, damage events, pickup notifications.
3. **Address the Entity extensibility problem** before adding many gameplay components (partial ECS or ComponentStore approach).
4. **Fix the diagonal speed bug** and **re-enable back-face culling** -- quick wins.
5. **Add fixed timestep** to decouple physics from frame rate.

---

## Source Reports
- [A1: Math & Physics](./a1-math-physics.md)
- [A2: Core & Scene](./a2-core-scene.md)
- [A3: Render & Input](./a3-render-input.md)
