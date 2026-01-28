# Wave 2: Dead Code Removal

**Effort**: 1-2 sessions
**Priority**: HIGH
**Dependencies**: None (can run parallel with Wave 1)

---

## Overview

Remove dead code identified by the swarm review. This reduces maintenance burden, eliminates compiler warnings, and makes the codebase easier to understand.

**Code to remove**:
- `player.rs` module (~150 lines) - superseded by PhysicsWorld integration
- Legacy Simplex4D pipeline (~800 lines) - tetrahedra pipeline is always used
- `slice.wgsl` shader (~460 lines) - legacy shader never executed
- `thickness` field in Hyperplane4D - set but never read

---

## Task 1: Remove player.rs Module

**Priority**: HIGH
**Effort**: 20 minutes
**Files**:
- `crates/rust4d_physics/src/player.rs` (DELETE)
- `crates/rust4d_physics/src/lib.rs`

### Background
The `PlayerPhysics` struct was the original player physics system. It was superseded when player physics was integrated directly into `PhysicsWorld` via:
- `set_player_body()`
- `player_jump()`
- `apply_player_movement()`
- `player_is_grounded()`

The old module is now completely unused.

### Steps

1. **Verify no usages** (already done by Physics Reviewer):
```bash
# Confirm no external references
grep -r "PlayerPhysics" src/
grep -r "PlayerPhysics" crates/rust4d_core/
grep -r "PlayerPhysics" examples/
grep -r "DEFAULT_PLAYER_RADIUS" src/
grep -r "DEFAULT_JUMP_VELOCITY" src/
```

2. **Remove module declaration** from `crates/rust4d_physics/src/lib.rs`:
```rust
// DELETE these lines:
pub mod player;
pub use player::*;
```

3. **Delete the file**:
```bash
rm crates/rust4d_physics/src/player.rs
```

4. **Verify compilation**:
```bash
cargo build --workspace
cargo test -p rust4d_physics
```

### Verification
- `cargo check` produces no errors
- `cargo test -p rust4d_physics` still passes (110 tests)
- No references to PlayerPhysics remain

---

## Task 2: Remove Legacy Simplex4D Pipeline

**Priority**: HIGH
**Effort**: 45 minutes
**Files**:
- `crates/rust4d_render/src/pipeline/slice_pipeline.rs`
- `crates/rust4d_render/src/pipeline/types.rs`
- `crates/rust4d_render/src/pipeline/lookup_tables.rs`
- `crates/rust4d_render/src/shaders/slice.wgsl` (DELETE)

### Background
The rendering system originally used 5-cell (Simplex4D) decomposition. It was replaced with tetrahedra-based slicing which is always used (`use_tetrahedra: true` is hardcoded). The legacy code is maintained but never executed.

### Dead Code Inventory

**In `slice_pipeline.rs`:**
- Legacy pipeline fields (lines 26-40): `pipeline`, `bind_group_layout_main`, `bind_group_layout_tables`, `edge_table_buffer`, `tri_table_buffer`, `edges_buffer`, `tables_bind_group`, `simplex_buffer`, `simplex_count`, `main_bind_group`
- `upload_simplices()` method
- `run_legacy_slice_pass()` method
- Legacy pipeline creation in `new()`

**In `types.rs`:**
- `Simplex4D` struct

**In `lookup_tables.rs`:**
- 5-cell lookup tables: `EDGES`, `EDGE_TABLE`, `TRI_TABLE`, `TRI_COUNT`
- Helper functions: `edge_count()`, `crossed_edges()`

**In `shaders/`:**
- `slice.wgsl` - entire file

### Steps

1. **Delete `slice.wgsl`**:
```bash
rm crates/rust4d_render/src/shaders/slice.wgsl
```

2. **Remove Simplex4D from `types.rs`**:
```rust
// DELETE the Simplex4D struct entirely
// Keep only Tetrahedron4D and related types
```

3. **Clean up `lookup_tables.rs`**:
```rust
// DELETE the 5-cell tables:
// - pub const EDGES: [[usize; 2]; 10]
// - pub const EDGE_TABLE: [u16; 32]
// - pub const TRI_TABLE: [[i8; 12]; 32]
// - pub const TRI_COUNT: [u8; 32]
// - pub fn edge_count(case: usize) -> usize
// - pub fn crossed_edges(case: usize) -> Vec<[usize; 2]>

// KEEP the tetrahedra tables:
// - TETRA_EDGES
// - TETRA_EDGE_TABLE
// - TETRA_TRI_TABLE
// - TETRA_TRI_COUNT
// - tetra_edge_count()
// - tetra_crossed_edges()
```

4. **Simplify `slice_pipeline.rs`**:

Remove from struct:
```rust
pub struct SlicePipeline {
    // DELETE these legacy fields:
    // pipeline: wgpu::ComputePipeline,
    // bind_group_layout_main: wgpu::BindGroupLayout,
    // bind_group_layout_tables: wgpu::BindGroupLayout,
    // edge_table_buffer: wgpu::Buffer,
    // tri_table_buffer: wgpu::Buffer,
    // edges_buffer: wgpu::Buffer,
    // tables_bind_group: wgpu::BindGroup,
    // simplex_buffer: wgpu::Buffer,
    // simplex_count: u32,
    // main_bind_group: Option<wgpu::BindGroup>,

    // KEEP tetrahedra fields:
    tetra_pipeline: wgpu::ComputePipeline,
    tetra_bind_group_layout: wgpu::BindGroupLayout,
    // ... etc
}
```

Remove methods:
```rust
// DELETE:
// pub fn upload_simplices(&mut self, ...)
// fn run_legacy_slice_pass(&self, ...)

// KEEP:
// pub fn upload_tetrahedra(&mut self, ...)
// pub fn run(&self, ...)  // Remove legacy branch
```

5. **Remove `use_tetrahedra` flag**:
Since we're removing legacy, the flag is no longer needed:
```rust
// In run() method, remove the if/else:
// Before:
if self.use_tetrahedra {
    self.run_tetra_slice_pass(...);
} else {
    self.run_legacy_slice_pass(...);  // Never called
}

// After:
self.run_tetra_slice_pass(...);
```

6. **Update shader includes in `new()`**:
Remove the legacy shader loading.

7. **Verify compilation and tests**:
```bash
cargo build --workspace
cargo test -p rust4d_render
cargo run  # Visual verification
```

### Verification
- `cargo check` produces no errors
- `cargo test -p rust4d_render` still passes
- App renders correctly with tetrahedra pipeline
- No references to Simplex4D, slice.wgsl remain

---

## Task 3: Remove thickness Field from Hyperplane4D

**Priority**: MEDIUM
**Effort**: 20 minutes
**Files**:
- `crates/rust4d_math/src/hyperplane.rs`
- `crates/rust4d_core/src/shapes.rs`
- `scenes/*.ron`
- `docs/user-guide.md` (if referenced)

### Problem
The `thickness` field is set in the constructor but never read, causing a compiler warning on every build:
```
warning: field `thickness` is never read
  --> crates/rust4d_math/src/hyperplane.rs:33:5
```

### Solution
Remove it entirely. No shims, no keeping parameters for "API compatibility". If it's not used, it's gone.

### Steps

1. **Remove field from struct** in `crates/rust4d_math/src/hyperplane.rs`:
```rust
pub struct Hyperplane4D {
    y: f32,
    size: f32,
    subdivisions: usize,
    cell_size: f32,
    // REMOVE: thickness: f32,
    vertices: Vec<Vec4>,
    tetrahedra: Vec<[usize; 4]>,
}
```

2. **Remove parameter from constructor**:
```rust
impl Hyperplane4D {
    pub fn new(y: f32, size: f32, subdivisions: usize, cell_size: f32) -> Self {
        // Remove thickness parameter and any code that used it
    }
}
```

3. **Update ShapeTemplate** in `crates/rust4d_core/src/shapes.rs`:
```rust
pub enum ShapeTemplate {
    Tesseract { size: f32 },
    Hyperplane {
        y: f32,
        size: f32,
        subdivisions: usize,
        cell_size: f32,
        // REMOVE: thickness: f32,
    },
}

impl ShapeTemplate {
    pub fn build(&self) -> Box<dyn ConvexShape4D> {
        match self {
            ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size } => {
                Box::new(Hyperplane4D::new(*y, *size, *subdivisions, *cell_size))
            }
            // ...
        }
    }
}
```

4. **Update all scene files** in `scenes/`:
```ron
// Before:
Hyperplane(y: -2.0, size: 20.0, subdivisions: 10, cell_size: 2.0, thickness: 0.1)

// After:
Hyperplane(y: -2.0, size: 20.0, subdivisions: 10, cell_size: 2.0)
```

5. **Update documentation** if thickness is mentioned anywhere.

6. **Verify**:
```bash
cargo check --workspace 2>&1 | grep -i thickness
# Should return nothing

cargo test --workspace
# All tests pass
```

---

## Task 4: Clean Up Other Dead Code

**Priority**: MEDIUM
**Effort**: 20 minutes
**Files**: Various

### Items to Remove

1. **Remove `#[allow(dead_code)]` markers** - after cleanup, if something still triggers dead code warnings, remove that code instead of suppressing the warning:
   - `crates/rust4d_render/src/pipeline/slice_pipeline.rs:19`
   - `crates/rust4d_render/src/pipeline/render_pipeline.rs:21`

2. **Remove unused utility functions** in `renderable.rs`:
   - `blended_color()` - exported but never called - DELETE
   - `ColorFn` type alias - defined but unused - DELETE

3. **Remove unused render methods**:
   - `RenderContext::render_clear()` - never called - DELETE
   - `RenderPipeline::render_direct()` - documented for debugging but unused - DELETE

4. **Remove unused shader entry points** from `render.wgsl`:
   - `fs_wireframe` - DELETE
   - `fs_normals` - DELETE
   - `fs_w_depth_only` - DELETE
   - (If we want these later, we can add them back when actually implementing the feature)

### Steps

1. Delete each unused item
2. Remove any `#[allow(dead_code)]` markers
3. Build and verify no new warnings:
```bash
cargo check --workspace 2>&1 | grep -i warning
# Should return only expected warnings (if any)
```

---

## Checklist

- [ ] Verify no external usages of PlayerPhysics
- [ ] Remove `player.rs` module
- [ ] Remove `pub mod player` from lib.rs
- [ ] Delete `slice.wgsl` shader file
- [ ] Remove `Simplex4D` struct from types.rs
- [ ] Remove 5-cell lookup tables
- [ ] Remove legacy pipeline fields from SlicePipeline
- [ ] Remove `upload_simplices()` method
- [ ] Remove `run_legacy_slice_pass()` method
- [ ] Remove `use_tetrahedra` conditional
- [ ] Add `thickness()` getter to Hyperplane4D
- [ ] Clean up `#[allow(dead_code)]` markers
- [ ] Run full test suite
- [ ] Visual verification that rendering still works

---

## Commits

1. `Remove dead player.rs module from rust4d_physics`
2. `Remove legacy Simplex4D pipeline and slice.wgsl shader`
3. `Add thickness() getter to Hyperplane4D to fix compiler warning`
4. `Clean up dead code markers and unused utilities`

---

## Impact Assessment

### Lines Removed (Approximate)
- `player.rs`: ~150 lines
- `slice.wgsl`: ~460 lines
- Legacy pipeline code: ~200 lines
- Legacy lookup tables: ~50 lines
- `thickness` field + parameter: ~10 lines
- Unused utility functions: ~50 lines
- Unused shader entry points: ~30 lines

**Total**: ~950 lines of dead code removed

### Risk Assessment
- **Low risk**: All code being removed is confirmed unused
- **Mitigation**: Each removal is a separate commit for easy revert
- **Testing**: Full test suite + visual verification after each task

### Philosophy
No legacy code preservation. No shims. No "keeping for API compatibility". If code is unused, it gets deleted. If we need it later, we can write it again (git history exists if we really need to recover something).
