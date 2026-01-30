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

### A2 (Core & Scene) - Key Findings

**rust4d_core** (202 tests, 6142 lines):
- Entity has 7 fields (name, tags, transform, shape, material, physics_body, dirty). Tags are `HashSet<String>`. No ECS/component extensibility -- adding gameplay features means modifying Entity struct directly. Biggest architectural concern.
- World uses SlotMap with generational keys. Name index for O(1) lookup. Tag lookup is O(n) scan. Physics integration syncs transforms and marks dirty only on actual change.
- Hierarchy: Full parent-child with cycle detection, reparenting, recursive delete, transform accumulation. Production-quality code.
- Scene pipeline: Template (RON) -> ActiveScene -> World. Async loading via worker thread. SceneManager has scene stack, overlay stack, transitions (fade/crossfade/slide), and validation.
- Asset cache: Type-erased via `Arc<dyn Any>`, path dedup, dependency tracking, GC, hot reload via file timestamp comparison.
- ShapeTemplate only has 2 variants: Tesseract and Hyperplane. No spheres, cylinders, custom meshes. Severely limits level design.
- FPS gaps: No spatial queries, no event system, no spawning, no inventory/weapons, no ECS. Infrastructure is solid but gameplay layer is absent.
- Ratings: Feature completeness 3/5, Code quality 4/5, Test coverage 4/5, FPS readiness 2/5.

### A3 (Render, Input & Main Binary) - Key Findings

**rust4d_render** (42 tests across modules):
- Two-stage GPU pipeline: compute shader slices 4D tetrahedra into 3D triangles, then render pipeline displays them with lighting and W-depth coloring. Architecture is clean.
- Camera4D uses Engine4D-style pitch/rotation separation: pitch stored separately, 4D rotations operate in XZW hyperplane via SkipY, Y axis always preserved. This is exactly right for 4D FPS movement.
- GPU-driven rendering: indirect draw with atomic counters avoids CPU-GPU readback. Compute workgroup size 64.
- Lookup table divergence: Rust-side `TETRA_TRI_TABLE` uses (0,1,2),(0,2,3) quad triangulation, WGSL shader uses (0,1,3),(0,3,2). Rust tables only used in tests, not a runtime bug but maintenance risk.
- Single-pass rendering only. No shadows, no multi-pass, no post-processing, no textures, no MSAA, no frustum culling.
- Entire world re-uploaded as single flat buffer on any change. No per-entity GPU state, no instancing.
- Back-face culling disabled ("for debugging") -- should be re-enabled.

**rust4d_input** (37 tests):
- CameraController with builder pattern, exponential mouse smoothing, FPS-style free look + 4D rotation mode (right-click).
- CameraControl trait cleanly abstracts camera operations. Camera4D implements it.
- Hardcoded key bindings (WASD/QE/Space/Shift). No rebinding, no gamepad, no action/axis abstraction.
- Key conflict: E = kata (4D movement) but traditionally E = interact in FPS. Will need rethinking.
- Diagonal movement not normalized (W+D gives sqrt(2) speed).
- No game actions: no shoot, no weapon switch, no interact.

**src/ (main binary)** - 3 systems (Window, Render, Simulation):
- SimulationSystem: delta time capped at 33ms, physics-driven movement with camera direction projection to XZW hyperplane, double camera position sync to discard controller movement while keeping rotation. Clever but fragile.
- No fixed-timestep update -- physics tied to frame rate.
- Layered config via figment (default.toml / user.toml / env vars). All config values flow to runtime behavior.
- Scene loading via SceneManager with RON files, player spawn support.
- InputMapper separates special keys (Escape/R/F/G) from movement keys.
- No game systems: no health, no weapons, no AI, no audio, no HUD.
- Ratings: render 3/5 features 4/5 quality 4/5 tests 1/5 FPS-ready; input 2/5 features 4/5 quality 5/5 tests 2/5 FPS-ready; main 2/5 features 4/5 quality 2/5 tests 2/5 FPS-ready.
