# Agent F Report: Foundation Phase Implementation Plan
**Date**: 2026-01-30
**Agent**: F (Foundation)
**Phase**: Foundation (pre-ECS, pre-split)

---

## Executive Summary

The Foundation phase from the cross-swarm synthesis identified three items: serialization gaps, fixed timestep, and quick fixes. After reading the actual source code, I can confirm these are real issues, but the picture is more nuanced than the synthesis suggested.

**Key findings:**

1. **Serialization gap is real but narrow.** `Rotor4` is the only type missing `Serialize`/`Deserialize`. A manual workaround already exists in `rust4d_core::transform` via a `#[serde(with = "rotor4_serde")]` module. The proper fix is adding derives directly to `Rotor4`. `RigidBody4D` and most physics types also lack serialization but this is less urgent (runtime state, not scene data).

2. **Fixed timestep is absent.** `simulation.rs` uses raw `dt` (capped at 33ms) passed directly to `physics.step(dt)`. There is no accumulator-based fixed timestep. This causes frame-rate-dependent physics behavior.

3. **Diagonal normalization is missing.** `simulation.rs:81` constructs `move_dir` from forward + right + ana inputs without normalizing, so diagonal movement is ~41% faster (sqrt(2) factor in 2D, worse in 3D with W-axis).

4. **Back-face culling is disabled.** `render_pipeline.rs:97` explicitly sets `cull_mode: None` with a comment "Disabled for debugging - was Some(wgpu::Face::Back)".

5. **All four items are ENGINE work** that should be done BEFORE the ECS migration (Phase 1 of the split plan). They touch `rust4d_math`, `rust4d_physics`, `rust4d_render`, and `src/systems/` -- all of which will be significantly refactored during ECS migration. Doing foundation fixes first means the ECS migration starts from a cleaner base.

**Total estimated effort: 1.5-2 sessions** (revised from the synthesis's 2 sessions).

---

## Item 1: Serialization Gap

### Current State

#### Rotor4 (rust4d_math) -- MISSING Serialize/Deserialize

**File**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_math/src/rotor4.rs`

`Rotor4` derives `Clone, Copy, Debug, Pod, Zeroable` but NOT `Serialize, Deserialize`.

```rust
// Line 36-37
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Rotor4 {
    pub s: f32,
    pub b_xy: f32,
    pub b_xz: f32,
    pub b_xw: f32,
    pub b_yz: f32,
    pub b_yw: f32,
    pub b_zw: f32,
    pub p: f32,
}
```

The `serde` crate is already a dependency in `rust4d_math/Cargo.toml` (used by `Vec4`), so adding the derives is a one-line change.

#### Workaround in Transform4D

**File**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_core/src/transform.rs`

Lines 8-30 contain a `rotor4_serde` module that manually serializes `Rotor4` as `[f32; 8]`. The comment on line 10 explicitly states: "Since Rotor4 is defined in rust4d_math and doesn't have Serialize/Deserialize". Once `Rotor4` gets the derives, this workaround can be removed and the `#[serde(with = "rotor4_serde")]` attribute on `Transform4D.rotation` can be dropped.

#### RotationPlane (rust4d_math) -- MISSING Serialize/Deserialize

**File**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_math/src/rotor4.rs`

```rust
// Line 15
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationPlane { ... }
```

`RotationPlane` is not currently serialized anywhere but will be needed for the editor (saving rotation plane configurations). Should add `Serialize, Deserialize` while we are touching the file.

#### Physics Types -- Serialization Audit

All physics types that lack `Serialize`/`Deserialize`:

| Type | File | Has Serialize? | Needed? |
|------|------|---------------|---------|
| `Rotor4` | `rust4d_math/src/rotor4.rs` | NO | YES -- blocks scene serialization without workaround |
| `RotationPlane` | `rust4d_math/src/rotor4.rs` | NO | NICE-TO-HAVE -- for editor/config |
| `RigidBody4D` | `rust4d_physics/src/body.rs` | NO | DEFERRED -- runtime state, not scene data |
| `StaticCollider` | `rust4d_physics/src/body.rs` | NO | DEFERRED -- runtime state |
| `BodyType` | `rust4d_physics/src/body.rs` | NO | DEFERRED -- needed if bodies are serialized |
| `Sphere4D` | `rust4d_physics/src/shapes.rs` | NO | DEFERRED -- needed if colliders serialized |
| `AABB4D` | `rust4d_physics/src/shapes.rs` | NO | DEFERRED -- needed if colliders serialized |
| `Plane4D` | `rust4d_physics/src/shapes.rs` | NO | DEFERRED -- needed if colliders serialized |
| `Collider` | `rust4d_physics/src/shapes.rs` | NO | DEFERRED -- enum wrapping shapes |
| `CollisionFilter` | `rust4d_physics/src/collision.rs` | NO | DEFERRED -- scene/save system |
| `CollisionLayer` | `rust4d_physics/src/collision.rs` | NO | DEFERRED -- bitflags need serde support |
| `PhysicsMaterial` | `rust4d_physics/src/material.rs` | NO | DEFERRED -- runtime state |
| `PhysicsConfig` | `rust4d_physics/src/world.rs` | YES | Already done |
| `Vec4` | `rust4d_math/src/vec4.rs` | YES | Already done |
| `Transform4D` | `rust4d_core/src/transform.rs` | YES (with workaround) | Works, cleanup after Rotor4 fix |
| `Material` | `rust4d_core/src/entity.rs` | YES | Already done |

### Engine vs Game Boundary

- **Rotor4 serialization**: Pure engine work (`rust4d_math`). Unblocks all downstream serialization.
- **Physics type serialization**: Engine work (`rust4d_physics`). Needed for save/load and editor, but not urgently. Can be done during or after ECS migration since the physics types will be refactored anyway.
- **Transform4D workaround removal**: Engine work (`rust4d_core`). Cleanup after Rotor4 fix.

### What the Split Plan Already Covers

The split plan's Phase 1 (ECS migration) will create new component types that all need `Serialize`/`Deserialize`. However, that work depends on `Rotor4` being serializable (since `Transform4D` contains `Rotor4`). So the `Rotor4` fix is a **prerequisite** for the split plan.

### Implementation Plan

**Priority A (do now, 0.25 session):**

1. Add `Serialize, Deserialize` to `Rotor4` in `rust4d_math/src/rotor4.rs`:
   - Change line 36 to: `#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]`
   - Add `use serde::{Serialize, Deserialize};` import

2. Add `Serialize, Deserialize` to `RotationPlane`:
   - Change line 15 to: `#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]`

3. Remove the `rotor4_serde` workaround in `rust4d_core/src/transform.rs`:
   - Delete lines 8-30 (the `rotor4_serde` module)
   - Remove `Serializer, Deserializer` from the `use serde` import
   - Remove `#[serde(with = "rotor4_serde")]` from the `rotation` field

4. Run `cargo test --workspace` to verify nothing breaks.

**Priority B (deferred to during/after ECS migration):**

- Add serialization to all physics types (`RigidBody4D`, `StaticCollider`, `BodyType`, collision shapes, `CollisionFilter`, `CollisionLayer`, `PhysicsMaterial`). This is a larger effort because:
  - `CollisionLayer` uses `bitflags!` which needs the `serde` feature enabled in the `bitflags` dependency
  - `Collider` is an enum that wraps shapes, each needing serialization
  - `RigidBody4D` contains `Collider` and `CollisionFilter`
  - Some of these types will be restructured during ECS migration

---

## Item 2: Fixed Timestep for Physics

### Current State

**File**: `/home/lemoneater/Projects/Rust4D/src/systems/simulation.rs`

Lines 57-62:
```rust
let now = Instant::now();
let raw_dt = (now - self.last_frame).as_secs_f32();
let dt = raw_dt.min(1.0 / 30.0);  // Cap at 33ms
self.last_frame = now;
```

Line 103:
```rust
scene_manager.update(dt);
```

The physics is called with variable `dt` each frame. There is no accumulator. This means:
- At 60fps, physics steps at ~16.6ms per frame
- At 30fps, physics steps at ~33ms per frame
- At 144fps, physics steps at ~6.9ms per frame

This causes frame-rate-dependent behavior:
- Gravity and velocity integration produce different results at different frame rates
- Collision resolution quality varies (larger steps = more tunneling risk)
- Player jump height varies by frame rate

### Engine vs Game Boundary

The fixed timestep should be in the **engine**, not the game. Every game needs deterministic physics regardless of frame rate. There are two places it could live:

1. **In `PhysicsWorld::step()`** -- the physics world itself runs with a fixed internal timestep
2. **In a new engine-level game loop utility** -- the engine provides a fixed-update scheduler

Option 1 is simpler and self-contained. Option 2 is more flexible but requires more architecture.

**Recommendation**: Option 1 for now. Add an accumulator-based fixed step to `PhysicsWorld`.

### What the Split Plan Already Covers

The split plan does NOT mention fixed timestep anywhere. It is focused on ECS migration and code extraction. This is genuinely new work that the synthesis correctly identified.

### Implementation Plan (0.5 session)

**Engine changes (`rust4d_physics/src/world.rs`):**

1. Add fixed timestep fields to `PhysicsWorld`:
   ```rust
   pub struct PhysicsWorld {
       // ... existing fields ...
       fixed_dt: f32,       // Fixed timestep (default: 1/60)
       accumulator: f32,    // Time accumulator
   }
   ```

2. Add `fixed_dt` to `PhysicsConfig`:
   ```rust
   pub struct PhysicsConfig {
       pub gravity: f32,
       pub jump_velocity: f32,
       pub fixed_dt: f32,      // NEW: fixed timestep (default: 1/60)
   }
   ```

3. Add a new `update(dt: f32)` method that uses the accumulator pattern:
   ```rust
   pub fn update(&mut self, dt: f32) {
       self.accumulator += dt;
       while self.accumulator >= self.fixed_dt {
           self.step(self.fixed_dt);
           self.accumulator -= self.fixed_dt;
       }
   }
   // Keep step() as-is for direct fixed-step usage
   ```

4. Return the interpolation alpha for rendering smoothness (optional, can defer):
   ```rust
   pub fn interpolation_alpha(&self) -> f32 {
       self.accumulator / self.fixed_dt
   }
   ```

**Game changes (`src/systems/simulation.rs`):**

5. Change line 103 from `scene_manager.update(dt)` to use the new `update()` method. This may require `SceneManager` or `World` to expose the new physics update method.

**Note**: The game code in `src/systems/simulation.rs` will move to the game repo in Phase 4 of the split plan. But since we're doing this BEFORE the split, we fix it in place. The game repo will inherit the fixed-timestep-aware simulation.

**Tests to add:**
- Physics produces identical results regardless of frame time subdivision
- Accumulator correctly handles large dt values
- Multiple sub-steps per frame when dt > fixed_dt
- Single sub-step when dt < fixed_dt (accumulates for next frame)

### Dependency on ECS

None. The fixed timestep is internal to `PhysicsWorld` and does not depend on ECS at all. It should be done BEFORE ECS migration since:
- The ECS migration will rewrite how `World.update()` calls physics
- Having a clean `PhysicsWorld::update(dt)` API makes the ECS integration simpler
- All existing physics tests continue to work (they call `step()` directly)

---

## Item 3: Quick Fixes

### 3a: Diagonal Movement Normalization

**File**: `/home/lemoneater/Projects/Rust4D/src/systems/simulation.rs`

Line 81:
```rust
let move_dir = forward_xzw * forward_input + right_xzw * right_input + ana_xzw * w_input;
```

Line 89:
```rust
physics.apply_player_movement(move_dir * move_speed);
```

The movement direction is a sum of up to 3 unit vectors multiplied by {-1, 0, 1} inputs. When moving diagonally (e.g., forward + right), `move_dir` has length sqrt(2) ~ 1.414. When moving in all three axes (forward + right + ana), length is sqrt(3) ~ 1.732. This means diagonal movement is 41-73% faster than cardinal movement.

**Fix:**

After line 81, normalize `move_dir` if its length exceeds 1.0:
```rust
let move_dir = forward_xzw * forward_input + right_xzw * right_input + ana_xzw * w_input;
// Clamp diagonal movement to prevent exceeding unit speed
let move_dir = if move_dir.length_squared() > 1.0 {
    move_dir.normalized()
} else {
    move_dir
};
```

Note: We normalize only if length > 1.0 because we want to preserve sub-unit-length movement (e.g., when only one axis is active, length is already 1.0, and normalizing would be a wasted operation).

**Engine vs Game:** This is currently **game code** (`src/systems/simulation.rs`). However, it represents a pattern that any game using the engine will need. Two options:
1. Fix it in `simulation.rs` now (game code, will move to game repo)
2. Add a `normalize_movement_input()` utility to `rust4d_game` (engine utility)

**Recommendation**: Fix it in `simulation.rs` now. When `rust4d_game` is created in Phase 2 of the split plan, the `CharacterController4D` should handle normalization internally.

### 3b: Re-enable Back-Face Culling

**File**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/render_pipeline.rs`

Line 97:
```rust
cull_mode: None, // Disabled for debugging - was Some(wgpu::Face::Back)
```

This was explicitly disabled for debugging. The comment indicates it was working before. Back-face culling:
- Halves the number of triangles the GPU rasterizes
- Removes visual artifacts from seeing internal faces of geometry
- Is standard practice for opaque solid geometry

**Fix:** Change to:
```rust
cull_mode: Some(wgpu::Face::Back),
```

**Engine vs Game:** Pure engine work (`rust4d_render`).

**Risk:** The 4D slicing pipeline generates triangles from tetrahedra cross-sections. The winding order of these generated triangles may not be consistent, which could be WHY culling was disabled. If re-enabling culling causes visible triangles to disappear, the fix is in the compute shader (`slice_tetra.wgsl`), not the render pipeline.

**Recommendation:** Re-enable culling, run the tech demo, and visually verify. If geometry disappears, revert and note it as a deeper issue requiring compute shader winding order fixes. Do NOT ship with culling disabled long-term.

### 3c: Additional Quick Fix Discovered -- dt Cap Value

**File**: `/home/lemoneater/Projects/Rust4D/src/systems/simulation.rs`

Line 61:
```rust
let dt = raw_dt.min(1.0 / 30.0); // Max 33ms per frame
```

This cap is reasonable for preventing huge physics steps but the fixed timestep (Item 2) makes this partially redundant. Once fixed timestep is implemented, this cap can be relaxed to something like `0.25` (250ms) since the accumulator will subdivide into fixed steps anyway. The cap would only protect against extreme stalls.

This is a minor cleanup that can happen alongside Item 2.

---

## Synthesis Gaps Identified

The original synthesis missed one item and underestimated another:

### Missed: Input Normalization Scope

The diagonal normalization issue is broader than just "normalize the direction vector." In 4D, the player has 3 movement axes (forward/back, left/right, ana/kata). The normalization needs to handle all combinations correctly:
- 1 axis active: length 1.0 (no normalization needed)
- 2 axes active: length sqrt(2) ~ 1.414 (normalize to 1.0)
- 3 axes active: length sqrt(3) ~ 1.732 (normalize to 1.0)

The synthesis listed this as "diagonal normalization" but the actual 4D case is "triagonal" -- movement in 3 simultaneous directions. The fix is the same (normalize if > 1.0) but the impact is larger in 4D.

### Missed: Physics Type Serialization Cascade

The synthesis said "add Serialize/Deserialize to Rotor4, RigidBody4D (1 session)". But `RigidBody4D` serialization requires serializing `Collider` (an enum wrapping `Sphere4D`, `AABB4D`, `Plane4D`), `CollisionFilter` (which contains `CollisionLayer` bitflags), and `PhysicsMaterial`. This is a cascade of ~8 types, not just 2.

The good news: for the immediate Foundation phase, we only need `Rotor4` serialization. The physics type cascade is needed for save/load and editor features, which come much later.

---

## Dependencies and Sequencing

```
Foundation Items (do BEFORE ECS migration):
  1. Rotor4 Serialize/Deserialize  (0.25 session)
     └─ Removes Transform4D workaround
     └─ Prerequisite for ECS component serialization

  2. Fixed timestep in PhysicsWorld  (0.5 session)
     └─ No dependencies
     └─ Makes ECS physics integration cleaner

  3. Diagonal normalization in simulation.rs  (0.1 session)
     └─ No dependencies
     └─ Will move to game repo; CharacterController4D should handle it

  4. Re-enable back-face culling  (0.1 session)
     └─ No dependencies
     └─ May reveal winding order issue in compute shader

Then: ECS Migration (Phase 1 of split plan, 4-6 sessions)
Then: Physics type serialization cascade (during Phase 2 or later)
```

### Parallelism

Items 1-4 are fully independent and can all be done in parallel if desired. However, they're small enough that a single session handles them all.

### What Blocks What

- **Rotor4 serialization** blocks: ECS component types that contain Transform4D, scene serialization improvements, editor work (Phase 5 of synthesis)
- **Fixed timestep** blocks: nothing directly, but is a prerequisite for deterministic physics which networking would eventually need (Phase 6 of synthesis)
- **Diagonal normalization** blocks: nothing, but affects gameplay feel immediately
- **Back-face culling** blocks: nothing, but affects rendering performance and visual quality

---

## Session Estimates

| Item | Estimate | Notes |
|------|----------|-------|
| Rotor4 + RotationPlane serialization + workaround removal | 0.25 session | Add derives, remove workaround, run tests |
| Fixed timestep | 0.5 session | New accumulator in PhysicsWorld, new tests, update simulation.rs |
| Diagonal normalization | 0.1 session | One-line fix + test |
| Back-face culling | 0.1 session | One-line change, visual verification, possible revert |
| **Total** | **~1 session** | Could stretch to 1.5 if culling reveals winding order issues |

The synthesis estimated 2 sessions for the full Foundation (including "Partial ECS" which is now superseded). Without Partial ECS, the actual Foundation work is approximately **1-1.5 sessions**.

---

## Exact Changes Summary

### File: `crates/rust4d_math/src/rotor4.rs`
- Add `use serde::{Serialize, Deserialize};`
- Line 15: Add `Serialize, Deserialize` to `RotationPlane` derives
- Line 36: Add `Serialize, Deserialize` to `Rotor4` derives

### File: `crates/rust4d_core/src/transform.rs`
- Delete lines 8-30 (entire `rotor4_serde` module)
- Change line 6 `use serde::{Serialize, Deserialize, Serializer, Deserializer};` to `use serde::{Serialize, Deserialize};`
- Remove `#[serde(with = "rotor4_serde")]` attribute from `rotation` field (line 38)

### File: `crates/rust4d_physics/src/world.rs`
- Add `fixed_dt: f32` and `accumulator: f32` fields to `PhysicsWorld`
- Add `fixed_dt: f32` field to `PhysicsConfig` (default: 1.0/60.0)
- Add `pub fn update(&mut self, dt: f32)` method with accumulator loop
- Add `pub fn interpolation_alpha(&self) -> f32` method
- Update `with_config()` to initialize new fields
- Add tests for fixed timestep behavior

### File: `crates/rust4d_render/src/pipeline/render_pipeline.rs`
- Line 97: Change `cull_mode: None` to `cull_mode: Some(wgpu::Face::Back)`

### File: `src/systems/simulation.rs`
- After line 81, add movement direction normalization:
  ```rust
  let move_dir = if move_dir.length_squared() > 1.0 {
      move_dir.normalized()
  } else {
      move_dir
  };
  ```
- Update physics call to use new `update()` API if available through SceneManager

---

## Open Questions for Other Agents

1. **For Agent P1 (Combat Core)**: Raycasting will need the fixed timestep to be in place. Ray-based weapon hits should be deterministic regardless of frame rate. Does your plan assume fixed timestep is done?

2. **For Agent P5 (Editor)**: The editor will need full physics type serialization for saving/loading scenes with physics objects. My recommendation is to defer this to Phase 2 of the split plan. Does that timing work for your editor plans?

3. **For all agents**: The `Rotor4` serialization fix changes how rotors appear in RON files. Currently they serialize as `[f32; 8]` arrays (via the workaround). After the fix, they'll serialize as struct fields `{ s: 1.0, b_xy: 0.0, ... }`. This is a breaking change for any existing RON files that contain rotors. Existing scene files that use `Transform4D` will need to be re-exported. All agents should be aware of this.

---

*Report generated by Agent F (Foundation)*
