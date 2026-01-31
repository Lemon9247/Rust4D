# Phase 0: Pre-Split Foundation Work

**Status**: Planned
**Estimated Effort**: 1-1.5 sessions
**Prerequisite For**: ECS Migration (Phase 1 of the engine/game split plan)

---

## Overview

These five tasks must be completed BEFORE the ECS migration begins. They address bugs, missing infrastructure, and serialization gaps in code that will be significantly refactored during the split. Doing them first means:

1. The ECS migration starts from a cleaner, more correct codebase.
2. Blocking serialization prerequisites are resolved (Rotor4 derives are needed for ECS component serialization).
3. Physics behavior is deterministic before it gets restructured into ECS systems.
4. Quick gameplay bugs are fixed in the current simple architecture rather than debugging them through the ECS rewrite.

All five tasks are **independent of each other** and can be parallelized, though they are small enough for a single session.

---

## Task 1: Rotor4 Serialization (BLOCKING)

**Priority**: A -- This is a prerequisite for ECS component serialization
**Estimate**: 0.25 session
**Dependencies**: None
**Blocks**: Phase 1 ECS Migration (Transform4D contains Rotor4; all ECS components need Serialize/Deserialize)

### Problem

`Rotor4` in `rust4d_math` does not derive `Serialize` or `Deserialize`. A manual workaround (`rotor4_serde` module) exists in `rust4d_core/src/transform.rs` that serializes rotors as `[f32; 8]` arrays. This workaround must be removed before ECS migration because ECS components need clean, direct serialization.

### Files to Modify

1. **`crates/rust4d_math/src/rotor4.rs`**
   - Add `use serde::{Serialize, Deserialize};` import
   - `Rotor4` (line ~36): Add `Serialize, Deserialize` to derive macro
   - `RotationPlane` (line ~15): Add `Serialize, Deserialize` to derive macro (nice-to-have for future editor/config use)

2. **`crates/rust4d_core/src/transform.rs`**
   - Delete lines 8-30: the entire `rotor4_serde` module
   - Change `use serde::{Serialize, Deserialize, Serializer, Deserializer};` to `use serde::{Serialize, Deserialize};`
   - Remove `#[serde(with = "rotor4_serde")]` attribute from the `rotation` field (line ~38)

### Implementation Notes

- The `serde` crate is already a dependency in `rust4d_math/Cargo.toml` (used by `Vec4`), so no new dependencies are needed.
- `Rotor4` derives `Pod` and `Zeroable` (bytemuck). These are compatible with serde derives.

### RON Format Breaking Change

**Important**: This changes how `Rotor4` values appear in RON files:
- **Before** (workaround): `rotation: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]`
- **After** (native derives): `rotation: (s: 1.0, b_xy: 0.0, b_xz: 0.0, b_xw: 0.0, b_yz: 0.0, b_yw: 0.0, b_zw: 0.0, p: 0.0)`

Any existing RON scene files containing `Transform4D` will need to be re-exported. All downstream agents should be aware of this format change.

### Verification

- `cargo test --workspace` passes
- Existing scene files load correctly (or are re-exported to match new format)
- `Transform4D` round-trips through RON serialization without the workaround module

---

## Task 2: Physics Type Serialization Audit

**Priority**: B -- Deferred to during/after ECS migration
**Estimate**: Included in Phase 2 of the split plan (not a separate session)
**Dependencies**: Task 1 (Rotor4 serialization must land first)
**Blocks**: Save/load system, editor scene saving, networking state sync (all future work)

### Problem

The original synthesis estimated "add Serialize/Deserialize to Rotor4, RigidBody4D (1 session)". Agent F's audit revealed this is a cascade of approximately 8 types, not just 2:

| Type | File | Status |
|------|------|--------|
| `RigidBody4D` | `rust4d_physics/src/body.rs` | Missing -- runtime state |
| `StaticCollider` | `rust4d_physics/src/body.rs` | Missing -- runtime state |
| `BodyType` | `rust4d_physics/src/body.rs` | Missing -- needed if bodies serialized |
| `Sphere4D` | `rust4d_physics/src/shapes.rs` | Missing -- needed if colliders serialized |
| `AABB4D` | `rust4d_physics/src/shapes.rs` | Missing -- needed if colliders serialized |
| `Plane4D` | `rust4d_physics/src/shapes.rs` | Missing -- needed if colliders serialized |
| `Collider` | `rust4d_physics/src/shapes.rs` | Missing -- enum wrapping shapes |
| `CollisionFilter` | `rust4d_physics/src/collision.rs` | Missing -- scene/save system |
| `CollisionLayer` | `rust4d_physics/src/collision.rs` | Missing -- bitflags needs serde feature |
| `PhysicsMaterial` | `rust4d_physics/src/material.rs` | Missing -- runtime state |

### Why Deferred

- These types represent runtime physics state, not scene data. They are needed for save/load and editor features, not for the ECS migration itself.
- `CollisionLayer` uses `bitflags!` which requires enabling the `serde` feature in the `bitflags` dependency -- a Cargo.toml change.
- Several of these types will be restructured during ECS migration anyway (e.g., `RigidBody4D` might become ECS components).
- Agent P5 (Editor) confirmed this timeline is compatible with their plans.

### Implementation Approach (when the time comes)

1. Enable `serde` feature on the `bitflags` dependency in `rust4d_physics/Cargo.toml`
2. Add derives bottom-up: shapes first, then `Collider` enum, then `CollisionFilter`/`CollisionLayer`, then `RigidBody4D`/`StaticCollider`
3. `PhysicsMaterial` is straightforward (simple struct with f32 fields)

### Verification

- All physics types round-trip through RON serialization
- Physics simulation produces identical results after deserialization

---

## Task 3: Fixed Timestep for Physics

**Priority**: A -- Physics is currently frame-rate dependent
**Estimate**: 0.5 session
**Dependencies**: None
**Blocks**: Deterministic physics (future networking depends on this), consistent gameplay feel

### Problem

`src/systems/simulation.rs` passes variable `dt` (capped at 33ms) directly to `physics.step(dt)`. There is no accumulator-based fixed timestep. This causes:

- Gravity and velocity integration produce different results at different frame rates
- Collision resolution quality varies (larger steps = more tunneling risk)
- Player jump height varies by frame rate
- At 60fps: physics steps at ~16.6ms; at 144fps: ~6.9ms; at 30fps: ~33ms

### Files to Modify

1. **`crates/rust4d_physics/src/world.rs`**
   - Add `fixed_dt: f32` and `accumulator: f32` fields to `PhysicsWorld`
   - Add `fixed_dt: f32` field to `PhysicsConfig` (default: `1.0/60.0`)
   - Add new `pub fn update(&mut self, dt: f32)` method with accumulator pattern:
     ```rust
     pub fn update(&mut self, dt: f32) {
         self.accumulator += dt;
         while self.accumulator >= self.fixed_dt {
             self.step(self.fixed_dt);
             self.accumulator -= self.fixed_dt;
         }
     }
     ```
   - Add `pub fn interpolation_alpha(&self) -> f32` for render smoothing (optional, can defer)
   - Update `with_config()` to initialize new fields from `PhysicsConfig`
   - Add tests for fixed timestep behavior

2. **`src/systems/simulation.rs`**
   - Update physics call to use new `update(dt)` API instead of direct `step(dt)`
   - The existing dt cap (`raw_dt.min(1.0 / 30.0)`) can be relaxed to `0.25` (250ms) since the accumulator subdivides into fixed steps; the cap only protects against extreme stalls

### Implementation Approach

Use the standard accumulator pattern (Option 1 from Agent F: self-contained in `PhysicsWorld`). The `step()` method remains available for direct fixed-step usage. The new `update()` method wraps it with the accumulator.

### Tests to Add

- Physics produces identical results regardless of frame time subdivision (e.g., 1x16ms vs 2x8ms)
- Accumulator correctly handles large dt values (multiple sub-steps)
- Single sub-step when dt < fixed_dt (remainder accumulates for next frame)
- Zero sub-steps when accumulated time < fixed_dt

### Verification

- `cargo test --workspace` passes
- Physics behavior is consistent regardless of frame rate
- Existing physics tests continue to work (they call `step()` directly)

---

## Task 4: Diagonal Movement Normalization

**Priority**: B -- Affects gameplay feel, not a blocker
**Estimate**: 0.1 session
**Dependencies**: None
**Blocks**: Nothing directly, but affects gameplay feel immediately

### Problem

`src/systems/simulation.rs` line 81 constructs `move_dir` from the sum of up to 3 unit vectors (forward, right, ana) without normalizing:

```rust
let move_dir = forward_xzw * forward_input + right_xzw * right_input + ana_xzw * w_input;
```

When inputs are {-1, 0, 1}:
- 1 axis active: length 1.0 (correct)
- 2 axes active: length sqrt(2) = ~1.414 (**41% faster**)
- 3 axes active: length sqrt(3) = ~1.732 (**73% faster**)

This is worse in 4D than in traditional 3D games because the player has 3 simultaneous movement axes (forward/back, left/right, ana/kata), not just 2 (forward/back, left/right).

### Files to Modify

1. **`src/systems/simulation.rs`**
   - After line 81 (the `move_dir` construction), add normalization:
     ```rust
     let move_dir = if move_dir.length_squared() > 1.0 {
         move_dir.normalized()
     } else {
         move_dir
     };
     ```
   - The `length_squared() > 1.0` check avoids normalizing sub-unit-length movement (when only one axis is active, length is already 1.0).

### Engine vs Game Boundary

This is currently **game code** in `src/systems/simulation.rs`. It will move to the game repo in Phase 4 of the split plan. When `rust4d_game` is created (Phase 2), the `CharacterController4D` should handle normalization internally as a standard feature.

Fix it in place now; the game repo inherits the corrected behavior.

### Verification

- Cardinal movement speed matches previous behavior
- Diagonal (2-axis) movement speed equals cardinal speed
- Tri-axial (3-axis) movement speed equals cardinal speed
- Sub-unit inputs (only one partial axis) are not re-normalized

---

## Task 5: Re-enable Back-Face Culling

**Priority**: B -- Performance and visual quality improvement
**Estimate**: 0.1 session
**Dependencies**: None
**Blocks**: Nothing directly, but affects rendering performance and visual correctness

### Problem

`crates/rust4d_render/src/pipeline/render_pipeline.rs` line 97 has back-face culling explicitly disabled:

```rust
cull_mode: None, // Disabled for debugging - was Some(wgpu::Face::Back)
```

Back-face culling:
- Halves the number of triangles the GPU rasterizes
- Removes visual artifacts from seeing internal faces of geometry
- Is standard practice for opaque solid geometry

### Files to Modify

1. **`crates/rust4d_render/src/pipeline/render_pipeline.rs`**
   - Line 97: Change `cull_mode: None` to `cull_mode: Some(wgpu::Face::Back)`

### Risk: Winding Order Issues

The 4D slicing compute shader (`slice_tetra.wgsl`) generates triangles from tetrahedra cross-sections. The winding order of these generated triangles may not be consistent, which could be WHY culling was originally disabled. If re-enabling culling causes visible triangles to disappear:

1. **Immediate mitigation**: Revert the change and document the winding order issue
2. **Proper fix** (separate task): Fix winding order in the compute shader output

### Verification

- Visual inspection: run the tech demo and verify geometry appears correctly
- No missing faces or flickering
- If geometry disappears, revert and file as a separate winding order issue
- Performance improvement observable (fewer triangles rasterized)

---

## Summary

| # | Task | Estimate | Priority | Files |
|---|------|----------|----------|-------|
| 1 | Rotor4 Serialization | 0.25 session | A (BLOCKING) | `rust4d_math/src/rotor4.rs`, `rust4d_core/src/transform.rs` |
| 2 | Physics Type Serialization | Deferred | B | `rust4d_physics/src/*.rs` (multiple files) |
| 3 | Fixed Timestep | 0.5 session | A | `rust4d_physics/src/world.rs`, `src/systems/simulation.rs` |
| 4 | Diagonal Normalization | 0.1 session | B | `src/systems/simulation.rs` |
| 5 | Back-Face Culling | 0.1 session | B | `rust4d_render/src/pipeline/render_pipeline.rs` |
| | **Total (excluding deferred)** | **~1 session** | | |

### Parallelization

All five tasks (excluding the deferred Task 2) are fully independent with no shared file modifications. They could theoretically be assigned to 4 parallel agents:

- Agent A: Task 1 (Rotor4 serialization + Transform4D cleanup)
- Agent B: Task 3 (Fixed timestep in PhysicsWorld)
- Agent C: Task 4 (Diagonal normalization in simulation.rs)
- Agent D: Task 5 (Back-face culling in render_pipeline.rs)

However, the total effort is only ~1 session, so a single agent can handle all tasks sequentially in one context window.

### Sequencing After Completion

```
Phase 0 (this document) -- 1 session
  |
  v
Phase 1: ECS Migration -- 4-6 sessions (see split-phases-1-5.md)
  |
  v
Phase 2: Game Logic Extraction + rust4d_game -- 3-4 sessions
  |
  v
Phases 3-5: Pluggable Scenes, Game Repo, Engine Cleanup
```

### What This Enables

After Phase 0 completion:
- `Rotor4` and `Transform4D` serialize cleanly, unblocking ECS component types
- Physics is frame-rate independent, providing a stable base for ECS restructuring
- Movement feels correct in 4D (no speed exploits)
- Rendering performance is improved (or winding order issue is documented)
- The codebase is ready for the ECS migration to begin
