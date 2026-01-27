# Session Report: Physics-Ready Architecture Refactor

**Date:** 2026-01-27
**Branch:** `refactor/physics-ready-architecture`

## Summary

Implemented a major architectural refactor to decouple geometry from rendering and prepare the codebase for future physics integration. This transforms the engine from a rendering-focused codebase to a proper entity-based architecture.

## Changes Made

### Step 1: Extract Pure Geometry to rust4d_math

Created new shape system in `rust4d_math`:

- **`shape.rs`**: `ConvexShape4D` trait and `Tetrahedron` struct
  - Trait provides `vertices()` and `tetrahedra()` for any 4D shape
  - Tetrahedron holds vertex indices, not colors

- **`tesseract.rs`**: `Tesseract4D` - pure 4D hypercube geometry
  - Moved from rust4d_render, stripped colors
  - Implements `ConvexShape4D` trait
  - Uses Kuhn triangulation for tetrahedra decomposition

- **`hyperplane.rs`**: `Hyperplane4D` - pure floor/ground plane
  - Moved from rust4d_render, stripped colors
  - Grid-based geometry with W extent for slicing

### Step 2: Create rust4d_core Crate

New crate for shared game types:

- **`transform.rs`**: `Transform4D` - position, rotation, scale
  - Uses Rotor4 for rotation
  - Methods: `transform_point()`, `inverse()`, `compose()`

- **`entity.rs`**:
  - `Material` - base color RGBA (minimal for now)
  - `ShapeRef` - enum for Shared (Arc) or Owned (Box) shapes
  - `Entity` - transform + shape + material

- **`world.rs`**: `World` container
  - Owns all entities
  - `add_entity()`, `get_entity()`, `iter()`
  - Placeholder `update(dt)` for future physics

### Step 3: Update rust4d_render

- **`renderable.rs`**: Bridges World/Entity to GPU buffers
  - `RenderableGeometry` - collects vertices and tetrahedra
  - `from_world()`, `from_entity()` methods
  - Custom color functions: `position_gradient_color`, `blended_color`
  - `CheckerboardGeometry` utility for floor patterns

- Removed old `geometry/` module (tesseract.rs, hyperplane.rs, mod.rs)
- Re-exports core types from rust4d_core for convenience

### Step 4: Update main.rs

- Uses new World/Entity API
- Creates entities with `Entity::with_material()` and `ShapeRef::shared()`
- Builds geometry with `RenderableGeometry::from_world()`
- Calls `world.update(dt)` in render loop (no-op for now)

## Architecture After Refactor

```
rust4d_math (no deps)
  - Vec4, Rotor4, Mat4
  - ConvexShape4D trait
  - Tesseract4D, Hyperplane4D
       │
       ▼
rust4d_core (depends on math)
  - Transform4D
  - Material, Entity, ShapeRef
  - World, EntityHandle
       │
       ├──────────────────┐
       ▼                  ▼
rust4d_render        rust4d_input
  - RenderableGeometry
  - SlicePipeline
  - RenderPipeline
       │                  │
       └────────┬─────────┘
                ▼
            main.rs
```

## Test Results

All 132 tests pass:
- rust4d_math: 54 tests
- rust4d_core: 29 tests
- rust4d_render: 48 tests
- Doc tests: 1 test

## Future Physics Integration

With this architecture, physics can be added by:

1. Create `rust4d_physics` crate with `RigidBody4D`, `PhysicsWorld`
2. Add `rigid_body: Option<RigidBodyHandle>` to Entity
3. Implement `World::update()` to step physics and sync transforms

## Files Changed

| Action | File |
|--------|------|
| Create | `crates/rust4d_math/src/shape.rs` |
| Create | `crates/rust4d_math/src/tesseract.rs` |
| Create | `crates/rust4d_math/src/hyperplane.rs` |
| Update | `crates/rust4d_math/src/lib.rs` |
| Create | `crates/rust4d_core/Cargo.toml` |
| Create | `crates/rust4d_core/src/lib.rs` |
| Create | `crates/rust4d_core/src/transform.rs` |
| Create | `crates/rust4d_core/src/entity.rs` |
| Create | `crates/rust4d_core/src/world.rs` |
| Create | `crates/rust4d_render/src/renderable.rs` |
| Delete | `crates/rust4d_render/src/geometry/` (moved to math) |
| Update | `crates/rust4d_render/src/lib.rs` |
| Update | `crates/rust4d_render/Cargo.toml` |
| Update | `Cargo.toml` (workspace) |
| Update | `src/main.rs` |

## Observations

1. **Clean separation achieved**: Shapes are now purely mathematical, entities own transforms, materials handle visual properties

2. **Flexible ownership**: ShapeRef enum allows both shared (Arc) and owned (Box) shapes - most entities will share shapes, but unique copies are possible

3. **Color functions work well**: The custom color function approach (position gradient, checkerboard) is flexible and clean

4. **Minor warning**: `Hyperplane4D::thickness` field is stored but never read. Could add a getter or remove if truly unused.

5. **No visual regression**: The refactored code produces identical visuals to before

## Open Questions for Future

1. Should entity transforms apply at geometry collection time or at render time? (Currently: collection time, which is simpler but requires rebuilding when transforms change)

2. How to handle dynamic entities efficiently? (Currently: would need to rebuild RenderableGeometry)

3. Per-vertex color functions vs per-entity materials - what's the right abstraction for complex materials?
