# Shader Agent Debug Report - Tesseract Cross-Section Bug

## Executive Summary

After thorough analysis of the slice.wgsl compute shader and lookup_tables.rs, I have identified **three distinct issues** that together cause the "pinwheel of triangles" visual artifact when slicing a tesseract at w=0.

## Issue 1: Insufficient Triangles for 6-Point Cross-Sections (CRITICAL)

**Location**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/lookup_tables.rs`

### The Problem

When 2 or 3 vertices of a 5-cell are above the slice plane, 6 edges are crossed, producing 6 intersection points. These 6 points form a **triangular prism** in 3D space. The surface of a triangular prism requires:
- 2 triangular caps = 2 triangles
- 3 rectangular sides (each split into 2 triangles) = 6 triangles
- **Total: 8 triangles**

However, the current `TRI_TABLE` is sized for only 4 triangles per case (12 indices = 4 * 3):

```rust
let prism_6pts: [i8; 12] = [0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5];
```

This fan triangulation only produces 4 triangles, treating the 6 points as a 2D polygon rather than the surface of a 3D prism. **Half the surface is not rendered.**

### Evidence from Tests

Running `cargo test --workspace test_triangle_table -- --nocapture` shows warnings for all 20 cases with 6 intersection points:

```
WARNING: Case 3 has 6 points (prism) but only 4 triangles (need 8 for closed surface)
WARNING: Case 5 has 6 points (prism) but only 4 triangles (need 8 for closed surface)
... (18 more cases)
```

### Impact on Tesseract Slice

At w=0, all 24 simplices are sliced. The distribution of cases:
- 6 simplices hit case 16 or equivalent (1 vertex above, 4 triangles each - OK)
- 6 simplices hit case 30 or equivalent (4 vertices above, 4 triangles each - OK)
- 12 simplices hit cases with 2 or 3 vertices above (6 points, need 8 triangles, only get 4)

**Half the triangles from 12 simplices are missing**, causing visible gaps in the cube surface.

### Fix Required

Expand `TRI_TABLE` to support 8 triangles (24 indices) per case:

```rust
// Change table type from [i8; 12] to [i8; 24]
pub const TRI_TABLE: [[i8; 24]; 32] = compute_tri_table();

// And in the shader:
@group(1) @binding(1) var<storage, read> tri_table: array<i32, 768>;  // 32 * 24
```

Then compute proper triangulations for each 6-point case based on the specific geometric arrangement.

---

## Issue 2: Fan Triangulation Assumes Convex Coplanar Points

**Location**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/lookup_tables.rs`, line 144

### The Problem

Even if we only needed 4 triangles, the fan triangulation pattern:
```rust
[0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5]
```

Assumes:
1. Points 0-5 are coplanar (they're not - it's a 3D prism)
2. Points form a convex polygon when projected
3. Point 0 is a good fan center

For a triangular prism cross-section, the 6 points form two parallel triangles connected by 3 edges. They are NOT coplanar, so fan triangulation produces degenerate or self-intersecting triangles.

### Fix Required

Compute proper 3D prism triangulation:
- Identify which 3 points form each cap triangle
- Create the 3 quad faces connecting the caps
- This requires analyzing the edge crossing order to determine point correspondence

---

## Issue 3: Winding Order Normalization May Fail for Non-Convex Configurations

**Location**: `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl`, lines 293-300

### Current Implementation

```wgsl
let tri_center = (p0 + p1 + p2) / 3.0;
if (dot(normal, tri_center) < 0.0) {
    // Flip winding by swapping v1 and v2
    ...
}
```

### The Problem

This heuristic assumes the object is convex and centered at the origin. It uses the triangle's center position to determine if the normal points "outward."

For the Kuhn triangulation of a tesseract:
1. The tesseract is centered at origin (OK)
2. But individual simplices extend in various directions
3. Internal triangles (shared between simplices) may have centers closer to origin than boundary triangles
4. The heuristic may incorrectly flip internal triangles

### Partial Mitigation

The current fix is approximately correct for convex objects centered at origin, which the tesseract is. However, it may cause issues if:
- The 4D camera matrix moves the tesseract off-center before slicing
- The object is non-convex

---

## Verification of Correct Components

### Edge Intersection Logic (CORRECT)
```wgsl
let dw = w1 - w0;
let t = select((slice_w - w0) / dw, 0.5, abs(dw) < 0.0001);
```
The interpolation formula is mathematically correct. The division-by-zero protection is properly implemented.

### Case Index Calculation (CORRECT)
```wgsl
if (transformed[i].w > slice_w) { case_idx |= (1u << i); }
```
Correctly identifies which vertices are above the slice plane.

### Triangle Table Indexing (CORRECT)
```wgsl
let tri_base = case_idx * 12u;
let i0 = tri_table[tri_base + tri_idx];
```
Correctly accesses the flattened 2D array.

### Back-Face Culling (OK, but depends on normals)
The render pipeline uses `cull_mode: Some(wgpu::Face::Back)` which is correct for a properly-oriented mesh. If normals point inward, triangles will be invisible.

---

## Recommended Fixes (Priority Order)

### Priority 1: Expand TRI_TABLE for 8 Triangles

This requires:
1. Change Rust table type: `[[i8; 24]; 32]`
2. Change shader buffer: `array<i32, 768>`
3. Implement proper prism triangulation for cases 3,5,6,7,9,10,11,12,13,14,17,18,19,20,21,22,24,25,26,28

### Priority 2: Compute Per-Case Triangulations

Each 6-point case has a different geometric arrangement. The triangulation must account for which edges are crossed and in what order. This requires analyzing the EDGE_TABLE masks to determine point correspondence.

For example, case 3 (v0, v1 above):
- Edges crossed: 1,2,3,4,5,6
- Points form a prism with caps near v0 and v1
- Need to identify which 3 points are on each cap

### Priority 3: Test with Visual Validation

Add a test that:
1. Slices tesseract at w=0
2. Counts total triangles generated
3. Verifies it matches expected cube surface (approximately 12 unique visible triangles, though overlapping triangles from adjacent simplices are expected)
4. Optionally renders and compares against reference image

---

## Appendix: Case Analysis for w=0 Slice

For a tesseract with vertices at +-1 in all dimensions:

**Vertices with w=-1 (below w=0)**: 0,1,2,3,4,5,6,7
**Vertices with w=+1 (above w=0)**: 8,9,10,11,12,13,14,15

Each of the 24 Kuhn simplices visits vertex 0 first and vertex 15 last. The intermediate vertices depend on the permutation order.

When dimension 3 (w) is:
- **Position 0** (flipped first): Case 30 - vertices 1,2,3,4 above (4 triangles)
- **Position 1** (flipped second): Case 28 or similar - 3 vertices above (6 points, need 8 triangles)
- **Position 2** (flipped third): Case 24 or similar - 2 vertices above (6 points, need 8 triangles)
- **Position 3** (flipped last): Case 16 - only vertex 4 above (4 triangles)

With 6 simplices per position category, we have:
- 12 simplices producing 4 triangles each = 48 triangles (correct)
- 12 simplices producing 4 triangles each instead of 8 = 48 triangles (MISSING 48 triangles)

**Expected total: 96 triangles. Actual: 48 triangles.**

The missing 48 triangles explain the "pinwheel" appearance - half the prism surfaces are not rendered.

---

## Files Modified

None yet. This report documents the analysis. Implementation of fixes requires:

1. `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/lookup_tables.rs` - Expand and fix TRI_TABLE
2. `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl` - Update buffer size and loop limit

---

*Shader Agent Report Complete*
