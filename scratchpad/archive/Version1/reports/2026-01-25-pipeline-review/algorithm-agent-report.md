# Algorithm Agent Report: TRI_TABLE Point Ordering Bug Analysis

**Agent**: Algorithm Agent
**Date**: 2026-01-25
**Task**: Analyze whether the TRI_TABLE's fixed triangulation pattern fails for different prism cases

---

## Executive Summary

**CONFIRMED: The TRI_TABLE point ordering is the ROOT CAUSE of the pinwheel bug.**

The prism triangulation in TRI_TABLE assumes a specific geometric arrangement of 6 intersection points (points 0,1,2 forming one cap; points 3,4,5 forming the other). However, points are collected in **edge index order** (0-9), which produces DIFFERENT geometric arrangements for different cases. Using the same triangulation pattern for all prism cases creates incorrect triangles.

---

## Detailed Analysis

### How Point Collection Works

The shader collects intersection points by iterating through edges 0-9 in order:

```wgsl
for (var edge_idx: u32 = 0u; edge_idx < 10u; edge_idx++) {
    if ((edge_mask & (1u << edge_idx)) != 0u) {
        intersection_points[point_count] = ...;
        point_count++;
    }
}
```

The points are numbered 0,1,2,3,4,5 based on which edges are crossed, in edge index order.

### Edge Definitions (from lookup_tables.rs)

```
Edge 0: v0-v1    Edge 5: v1-v3
Edge 1: v0-v2    Edge 6: v1-v4
Edge 2: v0-v3    Edge 7: v2-v3
Edge 3: v0-v4    Edge 8: v2-v4
Edge 4: v1-v2    Edge 9: v3-v4
```

### TRI_TABLE's Prism Assumption

The TRI_TABLE assumes this fixed triangulation for all 6-point cases:

```rust
let prism_6pts: [i8; 24] = [
    0, 1, 2,  // cap A
    5, 4, 3,  // cap B (opposite winding)
    0, 1, 4,  // side 1a
    0, 4, 3,  // side 1b
    1, 2, 5,  // side 2a
    1, 5, 4,  // side 2b
    2, 0, 3,  // side 3a
    2, 3, 5,  // side 3b
];
```

This assumes:
- Points 0,1,2 are adjacent (forming triangular cap A)
- Points 3,4,5 are adjacent (forming triangular cap B)
- Point 0 connects to point 3, point 1 to point 4, point 2 to point 5

### Case-by-Case Analysis of Prism Cases

Let me trace through several cases to show the discrepancy:

#### Case 3: v0,v1 above (EDGE_TABLE = 0b0001111110)

Crossed edges: 1,2,3,4,5,6

| Point | Edge | From Vertex | To Vertex |
|-------|------|-------------|-----------|
| 0 | 1 | v0 | v2 |
| 1 | 2 | v0 | v3 |
| 2 | 3 | v0 | v4 |
| 3 | 4 | v1 | v2 |
| 4 | 5 | v1 | v3 |
| 5 | 6 | v1 | v4 |

**Geometry**: Points 0,1,2 come from v0's edges to below vertices. Points 3,4,5 come from v1's edges to the SAME below vertices.
- Point 0 and Point 3 both connect to v2 -> they should be paired
- Point 1 and Point 4 both connect to v3 -> they should be paired
- Point 2 and Point 5 both connect to v4 -> they should be paired

**TRI_TABLE assumption MATCHES this case!** The prism template was likely designed for this case.

#### Case 5: v0,v2 above (EDGE_TABLE = 0b0101011010)

Crossed edges: 1,3,4,6,7,9

| Point | Edge | From Vertex | To Vertex |
|-------|------|-------------|-----------|
| 0 | 1 | v0 | v2 | **(both above - WAIT, this is wrong!)**

Let me recalculate. If v0,v2 are above:
- Edge 0 (v0-v1): v0 above, v1 below -> **CROSSED**
- Edge 1 (v0-v2): both above -> not crossed
- Edge 2 (v0-v3): v0 above, v3 below -> **CROSSED**
- Edge 3 (v0-v4): v0 above, v4 below -> **CROSSED**
- Edge 4 (v1-v2): v1 below, v2 above -> **CROSSED**
- Edge 5 (v1-v3): both below -> not crossed
- Edge 6 (v1-v4): both below -> not crossed
- Edge 7 (v2-v3): v2 above, v3 below -> **CROSSED**
- Edge 8 (v2-v4): v2 above, v4 below -> **CROSSED**
- Edge 9 (v3-v4): both below -> not crossed

Crossed edges: 0,2,3,4,7,8 -> edge mask should be 0b0110011101 = 413

Wait, let me verify with the actual EDGE_TABLE computation...

Case 5 binary = 0b00101 (v0=1, v2=1)

| Edge | Vertices | v0 above? | v1 above? | Crossed? |
|------|----------|-----------|-----------|----------|
| 0 | v0-v1 | yes | no | **YES** |
| 1 | v0-v2 | yes | yes | no |
| 2 | v0-v3 | yes | no | **YES** |
| 3 | v0-v4 | yes | no | **YES** |
| 4 | v1-v2 | no | yes | **YES** |
| 5 | v1-v3 | no | no | no |
| 6 | v1-v4 | no | no | no |
| 7 | v2-v3 | yes | no | **YES** |
| 8 | v2-v4 | yes | no | **YES** |
| 9 | v3-v4 | no | no | no |

Crossed edges: 0,2,3,4,7,8

| Point | Edge | From Vertex | To Vertex |
|-------|------|-------------|-----------|
| 0 | 0 | v0 | v1 |
| 1 | 2 | v0 | v3 |
| 2 | 3 | v0 | v4 |
| 3 | 4 | v1 | v2 |
| 4 | 7 | v2 | v3 |
| 5 | 8 | v2 | v4 |

**Analyzing the geometry:**
- Points 0,1,2 come from: v0->v1, v0->v3, v0->v4 (all from v0)
- Points 3,4,5 come from: v1->v2, v2->v3, v2->v4 (mixed!)

Now which points should pair?
- v1 is below. Which edges go to v1? Point 0 (v0->v1) and Point 3 (v1->v2) -> **0 pairs with 3**
- v3 is below. Which edges go to v3? Point 1 (v0->v3) and Point 4 (v2->v3) -> **1 pairs with 4**
- v4 is below. Which edges go to v4? Point 2 (v0->v4) and Point 5 (v2->v4) -> **2 pairs with 5**

**This ALSO matches the TRI_TABLE!** The pairing (0-3, 1-4, 2-5) is coincidentally correct.

#### Case 6: v1,v2 above (0b00110)

| Edge | Vertices | Crossed? |
|------|----------|----------|
| 0 | v0-v1 | **YES** (v0 below, v1 above) |
| 1 | v0-v2 | **YES** (v0 below, v2 above) |
| 2 | v0-v3 | no |
| 3 | v0-v4 | no |
| 4 | v1-v2 | no (both above) |
| 5 | v1-v3 | **YES** (v1 above, v3 below) |
| 6 | v1-v4 | **YES** (v1 above, v4 below) |
| 7 | v2-v3 | **YES** (v2 above, v3 below) |
| 8 | v2-v4 | **YES** (v2 above, v4 below) |
| 9 | v3-v4 | no |

Crossed edges: 0,1,5,6,7,8

| Point | Edge | From Vertex | To Vertex |
|-------|------|-------------|-----------|
| 0 | 0 | v0 | v1 |
| 1 | 1 | v0 | v2 |
| 2 | 5 | v1 | v3 |
| 3 | 6 | v1 | v4 |
| 4 | 7 | v2 | v3 |
| 5 | 8 | v2 | v4 |

**Analyzing pairings:**
- v0 is below. Which edges go to v0? Point 0 (v0->v1) and Point 1 (v0->v2) -> **0 pairs with 1**
- v3 is below. Which edges go to v3? Point 2 (v1->v3) and Point 4 (v2->v3) -> **2 pairs with 4**
- v4 is below. Which edges go to v4? Point 3 (v1->v4) and Point 5 (v2->v4) -> **3 pairs with 5**

**MISMATCH!** TRI_TABLE expects:
- 0 pairs with 3
- 1 pairs with 4
- 2 pairs with 5

But the actual geometry has:
- 0 pairs with 1
- 2 pairs with 4
- 3 pairs with 5

**This case will produce WRONG triangles!**

---

## Root Cause Confirmed

The TRI_TABLE uses a single prism triangulation pattern that assumes points are arranged as:
- Cap A: points 0,1,2 (from one "above" vertex)
- Cap B: points 3,4,5 (from other "above" vertex)
- With 0-3, 1-4, 2-5 forming the prism sides

But different cases have different edge crossing patterns, which means:
1. Points may come from DIFFERENT vertices than expected
2. The pairing relationships between cap A and cap B points vary by case
3. Using the same triangulation pattern creates triangles that connect wrong points

---

## Impact Analysis

For the w=0 slice of a tesseract (expecting a cube):

1. **Some simplices produce correct triangles** (cases where geometry matches TRI_TABLE)
2. **Some simplices produce incorrect triangles** (cases where geometry differs)
3. **Incorrect triangles create the "pinwheel" pattern** - triangles that don't form proper cube faces but instead create twisted/diagonal surfaces

---

## Solution Approach

### Option 1: Case-Specific TRI_TABLE
Create a separate triangulation pattern for each of the 20 prism cases, computed based on which specific edges are crossed and how they relate geometrically.

### Option 2: Runtime Point Reordering
In the shader, after collecting points, reorder them based on which "below" vertex each point connects to, ensuring the standard prism template works.

### Option 3: Compute Triangulation Dynamically
Instead of a lookup table, compute the correct triangulation at runtime based on point connectivity.

**Recommendation**: Option 2 (Runtime Point Reordering) is likely the most practical:
- Keeps the TRI_TABLE simple
- Single template for all prism cases
- Reordering logic adds minimal overhead
- Easier to verify correctness

The reordering should group points by which "below" vertex they connect to, then arrange them so points connecting to the same below vertex are in adjacent positions within their cap.

---

## Verification

To verify this analysis:
1. Add debug output for case 6 showing actual point positions
2. Visualize the triangles generated for case 6 vs expected
3. Compare with case 3 (which should work correctly)

---

## Files Analyzed

- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/geometry/tesseract.rs` - Simplex decomposition
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/pipeline/lookup_tables.rs` - TRI_TABLE definition
- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl` - Slicing shader

---

## Conclusion

**The bug is definitively in the TRI_TABLE's assumption that all prism cases have the same point ordering.** Case 6 (and likely several other cases) have different geometric arrangements that require different triangulation patterns.

This explains the pinwheel pattern: some simplices render correctly while others render with twisted triangles, creating a chaotic visual instead of clean cube faces.
