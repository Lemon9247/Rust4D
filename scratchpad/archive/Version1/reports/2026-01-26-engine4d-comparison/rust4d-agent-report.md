# Rust4D Agent Report: Cross-Section Rendering Implementation

**Agent**: Rust4D Agent
**Date**: 2026-01-26
**Task**: Document the current 4D cross-section rendering implementation in Rust4D

---

## Executive Summary

This report provides a thorough documentation of how Rust4D implements 4D cross-section rendering. The system uses a compute shader to slice 4D simplices (5-cells) with a W-hyperplane, producing 3D triangles for rendering. Key components include:

1. **Tesseract decomposition** into 24 5-cells via Kuhn triangulation
2. **Lookup tables** (EDGE_TABLE and TRI_TABLE) for fast case determination
3. **Compute shader** (slice.wgsl) for parallel cross-section computation
4. **Point sorting logic** for prism cases to ensure correct triangulation

The implementation has known issues with stray triangles, which this analysis helps diagnose.

---

## Overall Algorithm Flow

### Step 1: Tesseract Decomposition (tesseract.rs)

The tesseract (4D hypercube) is decomposed into **24 simplices** using **Kuhn triangulation**:

```rust
// Each simplex corresponds to a permutation of dimensions [0,1,2,3]
// For each permutation, create a path through vertices where each step
// flips exactly one coordinate from -h to +h
```

Each simplex has 5 vertices:
- Starts at vertex 0 (all coordinates -h): `[-h, -h, -h, -h]`
- Ends at vertex 15 (all coordinates +h): `[h, h, h, h]`
- Intermediate vertices are determined by the permutation order

**Key property**: Each simplex visits the same start/end points but takes a different path through the 4D space.

### Step 2: Case Classification (slice.wgsl lines 234-244)

For each simplex, the shader determines which vertices are above/below the slice plane:

```wgsl
var case_idx: u32 = 0u;
if (transformed[0].w > slice_w) { case_idx |= 1u; }
if (transformed[1].w > slice_w) { case_idx |= 2u; }
if (transformed[2].w > slice_w) { case_idx |= 4u; }
if (transformed[3].w > slice_w) { case_idx |= 8u; }
if (transformed[4].w > slice_w) { case_idx |= 16u; }
```

This produces a 5-bit case index (0-31) where each bit represents one vertex.

### Step 3: Edge Crossing Detection (lookup_tables.rs)

The `EDGE_TABLE` precomputes which of the 10 simplex edges are crossed for each case:

```rust
pub const EDGE_TABLE: [u16; 32] = compute_edge_table();
// Bit i is set if edge i is crossed (endpoints on opposite sides)
```

Edge definitions:
```
Edge 0: v0-v1    Edge 5: v1-v3
Edge 1: v0-v2    Edge 6: v1-v4
Edge 2: v0-v3    Edge 7: v2-v3
Edge 3: v0-v4    Edge 8: v2-v4
Edge 4: v1-v2    Edge 9: v3-v4
```

Cross-section types by vertex count above:
- **0 or 5 above**: No intersection (cases 0 and 31)
- **1 or 4 above**: **Tetrahedron** (4 intersection points, 4 triangles)
- **2 or 3 above**: **Triangular Prism** (6 intersection points, 8 triangles)

### Step 4: Intersection Point Computation (slice.wgsl lines 250-280)

For each crossed edge, compute the intersection point via linear interpolation:

```wgsl
fn edge_intersection(p0: vec4<f32>, p1: vec4<f32>, c0: vec4<f32>, c1: vec4<f32>, slice_w: f32) -> Vertex3D {
    let w0 = p0.w;
    let w1 = p1.w;
    let dw = w1 - w0;
    let t = select((slice_w - w0) / dw, 0.5, abs(dw) < 0.0001);  // Division-by-zero protection
    let pos = mix(p0, p1, t);
    let color = mix(c0, c1, t);
    // ... return Vertex3D
}
```

Points are collected in **edge index order** (0-9), which is critical for understanding the triangulation.

The shader also tracks which "above" and "below" vertex each intersection point connects to:

```wgsl
var above_vertex: array<u32, 10>;  // Which "above" vertex this point connects to
var below_vertex: array<u32, 10>;  // Which "below" vertex this point connects to
```

### Step 5: Point Sorting for Prism Cases (slice.wgsl lines 283-398)

For 6-point prism cases, the points must be sorted to match the TRI_TABLE's expected arrangement.

#### 2-Above Case Sorting (3 below vertices)

```wgsl
// For 2 above vertices: iterate over 3 below vertices
// Arrange: for each below vertex, place point from above_v1, then above_v2
for (var bi: u32 = 0u; bi < 3u; bi++) {
    let tb = below_verts[bi];
    // First: point from above_v1 to this below vertex
    // Second: point from above_v2 to this below vertex
}
```

**Expected result**:
- Points 0,1: both connect to below_verts[0]
- Points 2,3: both connect to below_verts[1]
- Points 4,5: both connect to below_verts[2]

#### 3-Above Case Sorting (2 below vertices)

```wgsl
// For 3 above vertices: iterate over 3 above vertices
// Arrange: for each above vertex, place point to below_v1, then below_v2
for (var ai: u32 = 0u; ai < 3u; ai++) {
    let ta = above_verts[ai];
    // First: point from this above vertex to below_v1
    // Second: point from this above vertex to below_v2
}
```

**Expected result**:
- Points 0,1: both from above_verts[0]
- Points 2,3: both from above_verts[1]
- Points 4,5: both from above_verts[2]

This means:
- Cap A = 0,2,4 (all connecting to below_v1)
- Cap B = 1,3,5 (all connecting to below_v2)
- Pairs 0-1, 2-3, 4-5 share the same above vertex

### Step 6: Triangle Generation (TRI_TABLE)

The `TRI_TABLE` defines triangulation patterns:

**Tetrahedron (4 points)**:
```rust
let tetra_4pts: [i8; 24] = [
    0, 1, 2,  // face 0
    0, 2, 3,  // face 1
    0, 3, 1,  // face 2
    1, 3, 2,  // face 3
    -1, ...   // unused
];
```

**Prism (6 points)**:
```rust
let prism_6pts: [i8; 24] = [
    0, 2, 4,  // cap A
    1, 5, 3,  // cap B (opposite winding)
    0, 2, 3,  // side 1a
    0, 3, 1,  // side 1b
    2, 4, 5,  // side 2a
    2, 5, 3,  // side 2b
    4, 0, 1,  // side 3a
    4, 1, 5,  // side 3b
];
```

The prism triangulation assumes:
- Cap A vertices: 0, 2, 4 (every other point starting at 0)
- Cap B vertices: 1, 3, 5 (every other point starting at 1)
- Adjacent pairs (0-1, 2-3, 4-5) share the same "controlling" vertex (above for 3-above, below for 2-above)

### Step 7: Normal Orientation (slice.wgsl lines 427-439)

Normals are oriented to point outward from the simplex:

```wgsl
let simplex_centroid = (transformed[0].xyz + transformed[1].xyz + transformed[2].xyz +
                        transformed[3].xyz + transformed[4].xyz) / 5.0;

let tri_center = (p0 + p1 + p2) / 3.0;
let to_centroid = simplex_centroid - tri_center;
if (dot(normal, to_centroid) > 0.0) {
    // Normal points toward simplex interior, flip to point outward
    let temp = v1;
    v1 = v2;
    v2 = temp;
    normal = -normal;
}
```

This ensures consistent outward-facing normals by checking if the normal points toward the simplex's centroid (interior).

---

## Detailed Analysis: 2-Above vs 3-Above Prism Cases

### The Key Difference

Both 2-above and 3-above cases produce 6 intersection points forming a triangular prism, but the **point correspondence differs**:

#### 2-Above Case (e.g., case 3: v0,v1 above)

- **Above vertices**: 2 (e.g., v0, v1)
- **Below vertices**: 3 (e.g., v2, v3, v4)
- **6 edges crossed**: Each above vertex connects to each below vertex

**Sorting strategy**: Group by **below vertex**
- Pairs share the same **below** vertex
- The two points in each pair come from the two different **above** vertices

#### 3-Above Case (e.g., case 7: v0,v1,v2 above)

- **Above vertices**: 3 (e.g., v0, v1, v2)
- **Below vertices**: 2 (e.g., v3, v4)
- **6 edges crossed**: Each above vertex connects to each below vertex

**Sorting strategy**: Group by **above vertex**
- Pairs share the same **above** vertex
- The two points in each pair go to the two different **below** vertices

### Why This Matters for TRI_TABLE

The TRI_TABLE's prism pattern assumes:
- Cap A = points 0, 2, 4
- Cap B = points 1, 3, 5
- Pairs (0-1), (2-3), (4-5) share a common vertex

For **2-above cases**: After sorting, pairs share the same **below** vertex
- Cap A points all connect to different below vertices (one each)
- Cap B points all connect to different below vertices (one each)
- This means Cap A and Cap B are "parallel" triangles at opposite ends of the prism

For **3-above cases**: After sorting, pairs share the same **above** vertex
- Cap A points all connect to below_v1
- Cap B points all connect to below_v2
- This means Cap A and Cap B are the triangles formed by edges going to each below vertex

**The same TRI_TABLE pattern should work for both** because the sorting normalizes the point arrangement.

---

## Potential Issues Identified

### Issue 1: Sorting Logic Asymmetry

The 2-above and 3-above sorting strategies produce **different geometric arrangements**:

**2-above arrangement**:
```
Points:  0    1    2    3    4    5
Below:   b0   b0   b1   b1   b2   b2
Above:   a0   a1   a0   a1   a0   a1
```

**3-above arrangement**:
```
Points:  0    1    2    3    4    5
Above:   a0   a0   a1   a1   a2   a2
Below:   b0   b1   b0   b1   b0   b1
```

The TRI_TABLE pattern:
- Cap A = 0, 2, 4
- Cap B = 1, 3, 5

For **2-above**: Cap A has points from below vertices {b0, b1, b2} - **correct, forms a triangle**
For **3-above**: Cap A has points from above vertices {a0, a1, a2} going to b0 - **correct, forms a triangle**

**Both should work with the same pattern!**

### Issue 2: Potential Vertex Order Inconsistency

The sorting finds above/below vertices by iterating 0-4:
```wgsl
for (var v: u32 = 0u; v < 5u; v++) {
    if ((case_idx & (1u << v)) != 0u) {
        above_verts[ac] = v;
        ac++;
    }
}
```

This means above_verts and below_verts are always in **ascending vertex index order** (0 before 1 before 2, etc.). This should provide deterministic behavior.

### Issue 3: Point Search May Fail

The sorting logic uses nested loops to find points:
```wgsl
for (var i: u32 = 0u; i < 6u; i++) {
    if (below_vertex[i] == tb && above_vertex[i] == above_v1) {
        sorted_points[si] = intersection_points[i];
        si++;
        break;
    }
}
```

**Potential issue**: If no matching point is found, `si` won't increment but the loop continues. This could leave gaps in `sorted_points` with uninitialized data.

However, mathematically, for a valid prism case:
- 2-above: 2 above * 3 below = 6 crossed edges, each should have exactly one point
- 3-above: 3 above * 2 below = 6 crossed edges, each should have exactly one point

So the search should always succeed. **But there's no error handling if it doesn't.**

### Issue 4: Cap Winding Order

The TRI_TABLE has:
```rust
0, 2, 4,  // cap A
1, 5, 3,  // cap B (opposite winding)
```

Cap B uses `1, 5, 3` instead of `1, 3, 5` to reverse the winding. This is necessary because the two caps face opposite directions.

**Potential issue**: If the normal orientation logic (Issue 5) also flips winding, it could double-flip Cap B triangles, making them face inward.

### Issue 5: Normal Orientation Using Simplex Centroid

The normal orientation uses the simplex centroid:
```wgsl
let simplex_centroid = (transformed[0].xyz + ... + transformed[4].xyz) / 5.0;
```

**Potential issue**: The simplex centroid may not represent the "inside" direction for the cross-section surface. The cross-section is part of a larger surface (the boundary of the sliced object), and the correct "outward" direction depends on the overall object geometry, not just the individual simplex.

For adjacent simplices that share an internal face:
- Their cross-section triangles should have **opposite** orientations (facing away from each other)
- But if both use their own centroid for orientation, they might end up with the same orientation

This could cause some triangles to face the wrong way, appearing as "stray" triangles that catch light incorrectly or are culled when they shouldn't be.

---

## Stray Triangle Root Cause Analysis

Based on this analysis, the most likely causes of stray triangles are:

### Hypothesis A: 3-Above Sorting Bug

The 3-above case sorting may not match what TRI_TABLE expects.

Looking at the sorting result for 3-above:
- Points 0,1 from above_verts[0]: to (below_v1, below_v2)
- Points 2,3 from above_verts[1]: to (below_v1, below_v2)
- Points 4,5 from above_verts[2]: to (below_v1, below_v2)

TRI_TABLE expects:
- Cap A (0,2,4): These go to (below_v1, below_v1, below_v1) - all to the same below vertex
- Cap B (1,3,5): These go to (below_v2, below_v2, below_v2) - all to the same below vertex

**This is correct!** Cap A and Cap B are triangles formed by all edges going to the same below vertex.

### Hypothesis B: 2-Above Sorting Bug

The 2-above case sorting produces:
- Points 0,1 to below_verts[0]: from (above_v1, above_v2)
- Points 2,3 to below_verts[1]: from (above_v1, above_v2)
- Points 4,5 to below_verts[2]: from (above_v1, above_v2)

TRI_TABLE expects:
- Cap A (0,2,4): from above_v1 to (below_verts[0], below_verts[1], below_verts[2])
- Cap B (1,3,5): from above_v2 to (below_verts[0], below_verts[1], below_verts[2])

**This is also correct!** Cap A and Cap B are triangles formed by edges from each above vertex.

### Hypothesis C: Normal Orientation Interference

The normal orientation logic may flip some triangles incorrectly:

1. Some triangles may have their centroid-to-simplex-centroid direction inconsistent with the desired outward direction
2. The flip logic modifies winding, which could cause visible artifacts

Since TRI_TABLE already has different windings for Cap A vs Cap B, additional flipping might corrupt this.

### Hypothesis D: Edge Cases at Degenerate Configurations

When the slice plane is exactly at w=0 and vertices are at w=+/-h:
- The `transformed[i].w > slice_w` comparison uses strict inequality
- Vertices exactly at slice_w would be classified as "below"
- But mathematically they should be "on" the plane

This shouldn't cause issues in practice since floating-point rarely produces exact equality.

---

## Verification Tests

To confirm the implementation is correct, the following tests would be valuable:

1. **Point sorting verification**: For a known case (e.g., case 3 and case 7), verify that sorted points match expected arrangement
2. **Triangle count verification**: Slice at w=0 should produce exactly N triangles (N depends on simplices affected)
3. **Watertight mesh check**: All triangles should form a closed surface with no gaps
4. **Normal direction check**: All normals should point outward from the slice surface

---

## Summary Table

| Component | Location | Status | Notes |
|-----------|----------|--------|-------|
| Tesseract decomposition | tesseract.rs | OK | 24 simplices via Kuhn triangulation |
| Edge definitions | lookup_tables.rs | OK | 10 edges correctly defined |
| EDGE_TABLE | lookup_tables.rs | OK | Precomputed edge masks |
| TRI_TABLE | lookup_tables.rs | OK | 8 triangles for prisms, 4 for tetrahedra |
| Case classification | slice.wgsl | OK | 5-bit case index |
| Edge intersection | slice.wgsl | OK | Linear interpolation with div-by-zero protection |
| Above/below tracking | slice.wgsl | OK | Tracks vertex relationships |
| 2-above sorting | slice.wgsl | **REVIEW** | Logic appears correct but needs verification |
| 3-above sorting | slice.wgsl | **REVIEW** | Logic appears correct but needs verification |
| Normal orientation | slice.wgsl | **SUSPECT** | Simplex centroid may not be ideal reference |

---

## Recommendations

1. **Add debug output** to verify sorted point arrangements for specific cases
2. **Test with a single simplex** to isolate issues from multi-simplex interference
3. **Review normal orientation logic** - consider using the slice plane normal or a global centroid instead of per-simplex centroid
4. **Add assertions/checks** for point sorting success (verify si == 6 after sorting)
5. **Visualize individual triangles** with distinct colors to identify which specific triangles are misplaced

---

*Rust4D Agent Report Complete*
