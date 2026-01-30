# Hive-Mind: Swarm A - Codebase State Review
**Date**: 2026-01-30

## Shared Context
We are reviewing the Rust4D 4D game engine codebase. The engine renders 3D cross-sections of 4D geometry.

### Crate Structure
- `rust4d_math` - 4D math primitives (Vec4, Rotor4, shapes, hyperplanes)
- `rust4d_physics` - 4D physics (rigid bodies, collision detection)
- `rust4d_core` - Core types (Entity, World, Scene, Assets, Hierarchy)
- `rust4d_render` - wgpu-based 4D→3D rendering pipeline
- `rust4d_input` - Camera controller and input handling
- `rust4d` (root) - Binary with main.rs and system modules

### Review Focus
Aim: Assess readiness for building a **4D boomer shooter** (Doom-like FPS).

For each area, report on:
1. **Features**: What exists, what works
2. **Architecture**: Quality, modularity, extensibility
3. **Testing**: Coverage, quality, gaps
4. **Boomer Shooter Gaps**: What's missing for an FPS game

## Agent Discoveries
(Agents: write key findings here for cross-pollination)

### A1 (Math & Physics) - Key Findings

**rust4d_math** (59 tests, all pass):
- Rotor4 is mathematically rigorous -- full geometric algebra with correct sandwich product for all 6 rotation planes. This is the hardest 4D math and it works.
- Vec4 is GPU-ready (`#[repr(C)]`, `Pod`, `Zeroable`). Supports all basic vector ops but lacks `reflect`, `project_onto`, `distance`.
- Mat4 is a type alias `[[f32; 4]; 4]`, not a struct. Has `skip_y()` (Engine4D compat) but no `inverse()`.
- Two renderable shapes: `Tesseract4D` and `Hyperplane4D`, both use Kuhn triangulation (code duplicated between them).
- BIGGEST GAP: No raycasting primitives at all. No `Ray4D`, no ray-shape intersection.

**rust4d_physics** (97 tests, all pass):
- Three collision primitives: Sphere4D, AABB4D, Plane4D. Four collision pair functions (sphere-plane, sphere-aabb, aabb-plane, aabb-aabb).
- Collision layer system is FPS-ready: 7 layers (DEFAULT, PLAYER, ENEMY, STATIC, TRIGGER, PROJECTILE, PICKUP) with bitflag masks.
- PhysicsWorld has player-specific logic baked in (grounded detection, jumping, edge-falling). Works but couples physics to gameplay.
- Physics materials with 5 presets (ICE, RUBBER, METAL, WOOD, CONCRETE). Geometric mean friction, max restitution combining.
- O(n^2) collision detection with no broadphase/spatial partitioning.
- No trigger event system (layer exists but no callback mechanism).
- No raycasting, no CCD, no character controller (no capsule collider, no step-up).

**Architecture note**: Dual shape systems are intentional -- `rust4d_math` has tetrahedra-based shapes for 4D slicing, `rust4d_physics` has analytical primitives for collision. This is correct separation.
