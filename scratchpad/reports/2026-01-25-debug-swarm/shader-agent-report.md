# Shader Agent Debug Report

## Summary

I identified **multiple issues** in the cross-section rendering code that together cause the "pinwheel of triangles" problem when slicing a tesseract at w=0.

## Key Insight: Internal Simplex Faces

The Kuhn triangulation decomposes the tesseract into 24 simplices that share internal faces. When slicing at w=0:

1. Each simplex edge that crosses w=0 produces an intersection point
2. There are 27 unique intersection points (not 8!) because the simplices include internal diagonal edges
3. The surface of the cross-section is formed by triangles from BOUNDARY faces only
4. Internal faces should have matching triangles from adjacent simplices that cancel out

The "pinwheel" effect likely comes from inconsistent normal directions on internal faces, preventing proper cancellation.

## Bug 1: TRI_TABLE Uses Incorrect Triangulation Indices (Critical)

**Location**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/lookup_tables.rs`

**The Problem**: The TRI_TABLE uses the same triangulation pattern (`tetra_4pts`) for all cases with 4 intersection points, but the geometric arrangement of those points differs depending on which vertex is above/below the slice plane.

### Detailed Analysis

The shader stores intersection points in **edge order** (by iterating through crossed edges 0-9). However, the geometric position of these points depends on which edges are crossed.

**Example: Case 1 vs Case 2**

Case 1 (v0 above, edges 0,1,2,3 crossed):
```
Point 0: on edge 0 (v0-v1) - from center toward +x
Point 1: on edge 1 (v0-v2) - from center toward +y
Point 2: on edge 2 (v0-v3) - from center toward +z
Point 3: on edge 3 (v0-v4) - from center toward +w
```

Case 2 (v1 above, edges 0,4,5,6 crossed):
```
Point 0: on edge 0 (v0-v1) - from corner toward center
Point 1: on edge 4 (v1-v2) - different direction
Point 2: on edge 5 (v1-v3) - different direction
Point 3: on edge 6 (v1-v4) - different direction
```

The **same index pattern** `[0, 1, 2, 0, 2, 3, 0, 1, 3, 1, 2, 3]` applied to these different point arrangements produces triangles with **inconsistent winding order**.

### Why This Causes Pinwheel Effect

When winding order is inconsistent:
- Some triangles face outward (rendered correctly)
- Some triangles face inward (back-face culled or lit from wrong side)
- The result looks like a broken "pinwheel" pattern

### The Fix

Each case needs its own triangulation that accounts for the geometric arrangement of intersection points. This requires computing proper triangulation per-case.

**Option A**: Pre-compute correct triangulation indices for all 32 cases (tedious but fast at runtime)

**Option B**: Compute winding at runtime by checking normal direction against a known "outward" direction

I recommend **Option A** for correctness and performance.

Here's what the fix looks like for the tetrahedron cases:

```rust
// Case 1: v0 above (edges 0,1,2,3)
// Points form tetrahedron near v0, need CCW winding from outside
table[1] = [0, 1, 2, 0, 2, 3, 1, 3, 2, 0, 3, 1];

// Case 2: v1 above (edges 0,4,5,6)
// Different geometry - needs different triangulation
table[2] = [0, 2, 1, 0, 3, 2, 1, 2, 3, 0, 1, 3];

// ... and so on for each case
```

Actually, the cleanest fix is to normalize the winding at runtime. Add this to the shader after computing the normal:

```wgsl
// Ensure consistent outward-facing normals
// Compute triangle center
let center = (vertex_position(v0) + vertex_position(v1) + vertex_position(v2)) / 3.0;

// If normal points inward (toward origin), flip the triangle winding
if (dot(normal, center) < 0.0) {
    // Swap v1 and v2 to flip winding
    let temp = v1;
    v1 = v2;
    v2 = temp;
    normal = -normal;
}
```

## Bug 2: Division by Zero Risk (Minor)

**Location**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl`, line 141

**The Problem**:
```wgsl
let t = (slice_w - w0) / (w1 - w0);
```

If an edge is parallel to the slice plane (`w0 == w1`), this produces division by zero (NaN).

**The Fix**:
```wgsl
let dw = w1 - w0;
let t = select((slice_w - w0) / dw, 0.5, abs(dw) < 0.0001);
```

This falls back to the edge midpoint when the edge is nearly parallel.

## Algorithm Verification

### Edge Intersection Logic (Correct)
The interpolation `t = (slice_w - w0) / (w1 - w0)` is mathematically correct. When we substitute back:
- `w_result = w0 + t * (w1 - w0) = w0 + (slice_w - w0) = slice_w`

### Case Index Calculation (Correct)
The bit packing is correct:
```wgsl
if (transformed[i].w > slice_w) { case_idx |= (1u << i); }
```

### Triangle Table Indexing (Correct)
The flattened array access `tri_table[case_idx * 12 + offset]` correctly maps 2D indices to 1D.

## Recommendations

1. **Priority 1**: Fix TRI_TABLE winding order or add runtime winding normalization
2. **Priority 2**: Add division-by-zero protection
3. **Testing**: Add a unit test that verifies the slice of a tesseract at w=0 produces a cube with 6 faces

## Files to Modify

1. `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/lookup_tables.rs` - Fix TRI_TABLE
2. `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl` - Add winding normalization and division protection

---

## Fixes Applied

### Fix 1: Division by Zero Protection (Applied)

Modified `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl` line ~141:

```wgsl
// Before:
let t = (slice_w - w0) / (w1 - w0);

// After:
let dw = w1 - w0;
let t = select((slice_w - w0) / dw, 0.5, abs(dw) < 0.0001);
```

### Fix 2: Runtime Winding Normalization (Applied)

Modified `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl` in the triangle generation loop:

```wgsl
// Compute normal using helper functions
let p0 = vertex_position(v0);
let p1 = vertex_position(v1);
let p2 = vertex_position(v2);
var normal = compute_normal(p0, p1, p2);

// Ensure consistent outward-facing normals
// The cross-section of a convex shape should have outward-facing normals
// We use the triangle center as a proxy: if normal points toward origin,
// the winding is backwards
let tri_center = (p0 + p1 + p2) / 3.0;
if (dot(normal, tri_center) < 0.0) {
    // Flip winding by swapping v1 and v2
    let temp = v1;
    v1 = v2;
    v2 = temp;
    normal = -normal;
}
```

This ensures all triangles face outward from the origin, which should give consistent winding for a convex cross-section.

## Build Status

The project builds successfully after these changes:
```
cargo build --package rust4d_render
   Compiling rust4d_render v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s)
```

## Test Status

All 30 tests in rust4d_render pass after these fixes.

The Geometry Agent appears to have updated the cross-section tests with better expectations:
- `test_cross_section_at_w0_simplex_edges_analysis` - analyzes the internal edge structure
- `test_cross_section_geometry_cube_corners_present` - verifies cube corners are present among intersection points

## Next Steps for Verification

1. Run the visual demo to see if the pinwheel effect is fixed
2. If still broken, the issue may be in the render pipeline (depth testing, back-face culling settings)
3. Consider adding a test that counts the number of visible surface triangles (should be 12 for a cube: 6 faces * 2 triangles each)
