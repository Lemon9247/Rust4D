# Reference Agent Report: 4D Cross-Section Rendering Research

**Date:** 2026-01-25
**Task:** Investigate how other 4D engines and academic resources handle cross-section rendering

## Executive Summary

The tesseract cross-section at w=0 should indeed show a cube. If a pinwheel pattern is appearing instead, this indicates a problem with either:
1. Vertex winding order in the triangulation
2. The lookup table for tetrahedron-hyperplane intersection cases
3. Face orientation/normal computation

This report summarizes the standard approaches used by established 4D engines and academic literature.

---

## 1. Industry Standard: Tetrahedron-Based Slicing

### The Universal Approach

All major 4D engines (Miegakure, 4D Toys, 4D Golf/Engine4D, hypervis) use the **same fundamental approach**:

1. **Decompose 4D objects into tetrahedra** (4-simplices embedded in 4D)
2. **Slice each tetrahedron** with a 3D hyperplane
3. **Emit 0, 1, or 2 triangles** per tetrahedron based on intersection

This approach was chosen because slicing a tetrahedron produces a simple, predictable result: either nothing, a triangle, or a quadrilateral.

### Source References
- [Miegakure](https://miegakure.com/) - Marc ten Bosch's seminal 4D game
- [Engine4D](https://github.com/HackerPoet/Engine4D) - CodeParade's Unity toolkit for 4D Golf
- [hypervis](https://github.com/t-veor/hypervis) - Open-source 4D physics engine
- [four](https://github.com/mwalczyk/four) - 4D renderer with detailed documentation

---

## 2. The 16-Case Lookup Table

### Why Lookup Tables?

A tetrahedron has 4 vertices. Each vertex can be either above or below the slicing hyperplane. This gives 2^4 = **16 possible configurations**.

The standard approach encodes which side each vertex is on:
```
s0 = vertex 0 above hyperplane ? 0 : 1
s1 = vertex 1 above hyperplane ? 0 : 1
s2 = vertex 2 above hyperplane ? 0 : 1
s3 = vertex 3 above hyperplane ? 0 : 1

result_code = s0 + (s1 << 1) + (s2 << 2) + (s3 << 3)
```

This produces a value 0-15 that indexes into a lookup table.

### The 16 Cases (Reduced by Symmetry to 8)

Due to symmetry (swapping "above" and "below" gives equivalent intersection), there are effectively only 8 unique cases:

| Case | Above | Below | Result | Shape |
|------|-------|-------|--------|-------|
| 0    | 4     | 0     | Nothing | Empty |
| 1    | 3     | 1     | Triangle | 3 edges intersected |
| 2    | 3     | 1     | Triangle | 3 edges intersected |
| 3    | 2     | 2     | **Quadrilateral** | 4 edges intersected |
| 4    | 3     | 1     | Triangle | 3 edges intersected |
| 5    | 2     | 2     | **Quadrilateral** | 4 edges intersected |
| 6    | 2     | 2     | **Quadrilateral** | 4 edges intersected |
| 7    | 1     | 3     | Triangle | 3 edges intersected |
| 8-15 | (mirror of 0-7) | | |

### Critical Insight: The Quadrilateral Cases

When 2 vertices are above and 2 are below, the intersection is a **quadrilateral** (4 vertices). This must be split into 2 triangles for rendering.

**This is likely where the pinwheel bug originates** - if the quadrilateral vertices are not properly ordered, the resulting triangles will have inconsistent winding.

### Reference: Lookup Table Structure

From the [arxiv paper on 4D spacetime visualization](https://arxiv.org/html/2403.19036v1):

> "A 16x4 lookup table determines which edges intersect for each case. The second dimension is 4 because there are at most 4 edges intersected. When the result corresponds to a triangle, the fourth entry is set in such a way that the quadrilateral still represents a triangle, thus avoiding a conditional."

---

## 3. The Simplex/Hyperplane Intersection Theorem

### Mathematical Foundation

From [David Eppstein's analysis](https://ics.uci.edu/~eppstein/junkyard/simplex-section.html):

> "If there are n points above the plane and m below, then the intersection is the **Cartesian product** of an n-point simplex and an m-point simplex."

Concrete examples:
- **(n,m) = (1,3)** or **(3,1)**: Triangle (0-simplex x 2-simplex)
- **(n,m) = (2,2)**: **Triangular prism structure** (1-simplex x 1-simplex = quadrilateral)

This is why the 2-2 split case produces a quadrilateral: it's the product of two line segments.

---

## 4. Vertex Ordering / Winding Order

### The Critical Problem

When slicing produces a quadrilateral, the 4 intersection points must be ordered correctly to form a proper convex quad. If they're in the wrong order, you get:
- A self-intersecting "bowtie" shape
- Triangles with inconsistent winding (one clockwise, one counter-clockwise)
- Visual artifacts like "pinwheel" patterns

### The Solution: Angular Sorting

From the [four renderer](https://github.com/mwalczyk/four):

> "For quad results, sorts four vertices by their **signed angle** with the polygon's normal using insertion sort."

The algorithm:
1. Compute the centroid of the 4 intersection points
2. Compute the polygon normal (cross product of two edge vectors)
3. For each vertex, compute its signed angle around the normal relative to the centroid
4. Sort vertices by this angle
5. Output triangles: (v0, v1, v2) and (v0, v2, v3)

### Alternative: Consistent Edge Ordering in Lookup Table

Engine4D uses a different approach: the lookup table itself specifies edges in a consistent order that guarantees proper winding when processed sequentially.

---

## 5. Tesseract Decomposition

### How Many Tetrahedra?

A tesseract (4D hypercube) has 8 cubic cells. Each cube can be decomposed into 6 tetrahedra.

**Total: 8 x 6 = 48 tetrahedra** for a complete tesseract.

However, this naive decomposition has redundancy. More efficient decompositions exist.

### Expected Result at w=0

When slicing a tesseract at w=0:
- The hyperplane passes through the "middle" of the tesseract
- The intersection should be a **cube** (the 3D analog)
- This cube should have 6 faces, properly oriented

If you're seeing a pinwheel instead of a cube, the face triangles likely have inconsistent winding.

### Reference
From the [Interactive 4D Handbook](https://baileysnyder.com/interactive-4d/4d-cubes/):

> "When all sliders are set to 0, even though the tesseract has 8 potential cubes to slice, the final image appears to be a normal cube with only 6 faces."

---

## 6. Prism Cross-Sections and Triangulation

### Why Prisms Matter

The intersection of certain 4D primitives with a hyperplane produces triangular prisms. A triangular prism must be decomposed into tetrahedra for rendering.

### Standard Decomposition: 3 Tetrahedra

From [Euclid's Elements (Book XII, Proposition 7)](https://proofwiki.org/wiki/Prism_on_Triangular_Base_divided_into_Three_Equal_Tetrahedra):

A triangular prism with vertices ABC (bottom) and DEF (top) decomposes into:
1. Tetrahedron ABDC (base ABD, apex C)
2. Tetrahedron DEBC (base DEB, apex C)
3. Tetrahedron EBCD (base EBC, apex D)

These three tetrahedra have equal volume.

### Vertex Ordering Consistency

The key is that diagonal edges must be chosen **consistently** across adjacent prisms. If one prism uses diagonal BD and an adjacent prism uses diagonal AC, the resulting mesh will have gaps or overlaps.

---

## 7. Comparison of Implementation Approaches

### Miegakure / 4D Toys (Marc ten Bosch)
- Custom engine built from scratch
- Uses geometric algebra for rotations
- Published paper: "N-Dimensional Rigid Body Dynamics" (SIGGRAPH 2020)
- Procedural 3D texturing for 4D surfaces

### Engine4D / 4D Golf (CodeParade)
- Unity-based toolkit
- Uses lookup tables (regeneratable via editor)
- CoreND.cginc shader for 4D transformations
- Open source: https://github.com/HackerPoet/Engine4D

### hypervis (t-veor)
- Rust-based (wgpu-rs)
- Compute shader for tetrahedron slicing
- Uses rotors (geometric algebra) instead of quaternions
- Clear documentation of the slicing algorithm

### four (mwalczyk)
- Also uses compute shaders
- Well-documented GPU pipeline
- Explicit angular sorting for quad vertices
- Uses glMultiDrawArraysIndirect for efficiency

---

## 8. Recommendations for Rust4D

### Immediate Bug Investigation

1. **Check the quadrilateral cases** (result codes 3, 5, 6, and their mirrors 9, 10, 12)
   - Are the 4 vertices being emitted in the correct order?
   - Is the quad being split into triangles consistently?

2. **Verify consistent winding**
   - All triangles should have the same winding (all CW or all CCW)
   - Use the determinant of the vertex matrix to check orientation

3. **Test with a single tetrahedron first**
   - Slice a single tetrahedron at various positions
   - Verify the output is correct for all 16 cases

### Recommended Algorithm

```rust
// Pseudo-code for tetrahedron-hyperplane intersection

fn slice_tetrahedron(tet: [Vec4; 4], plane: Hyperplane) -> Vec<Triangle> {
    // 1. Classify vertices
    let signs: [bool; 4] = tet.map(|v| plane.signed_distance(v) >= 0.0);
    let code = signs[0] as u8
             | (signs[1] as u8) << 1
             | (signs[2] as u8) << 2
             | (signs[3] as u8) << 3;

    // 2. Lookup which edges to intersect
    let edges = EDGE_TABLE[code as usize];

    // 3. Compute intersection points
    let points: Vec<Vec3> = edges.iter()
        .filter(|&&e| e >= 0)
        .map(|&e| intersect_edge(tet, e, plane))
        .collect();

    // 4. Emit triangles
    match points.len() {
        0 => vec![],
        3 => vec![Triangle(points[0], points[1], points[2])],
        4 => {
            // CRITICAL: Sort by angle to ensure consistent winding
            let sorted = sort_quad_vertices(points);
            vec![
                Triangle(sorted[0], sorted[1], sorted[2]),
                Triangle(sorted[0], sorted[2], sorted[3]),
            ]
        }
        _ => unreachable!()
    }
}

fn sort_quad_vertices(mut points: Vec<Vec3>) -> Vec<Vec3> {
    let centroid = points.iter().sum() / 4.0;
    let normal = (points[1] - points[0]).cross(points[2] - points[0]).normalize();
    let ref_dir = (points[0] - centroid).normalize();

    points.sort_by(|a, b| {
        let angle_a = signed_angle(a - centroid, ref_dir, normal);
        let angle_b = signed_angle(b - centroid, ref_dir, normal);
        angle_a.partial_cmp(&angle_b).unwrap()
    });

    points
}
```

### Edge Table Reference

The 6 edges of a tetrahedron (vertices 0,1,2,3):
```
Edge 0: 0-1
Edge 1: 0-2
Edge 2: 0-3
Edge 3: 1-2
Edge 4: 1-3
Edge 5: 2-3
```

A typical lookup table for which edges intersect:
```rust
const EDGE_TABLE: [[i8; 4]; 16] = [
    [-1, -1, -1, -1],  // 0: 0000 - no intersection
    [ 0,  1,  2, -1],  // 1: 0001 - triangle
    [ 0,  3,  4, -1],  // 2: 0010 - triangle
    [ 1,  2,  3,  4],  // 3: 0011 - quad (CRITICAL)
    [ 1,  3,  5, -1],  // 4: 0100 - triangle
    [ 0,  2,  3,  5],  // 5: 0101 - quad (CRITICAL)
    [ 0,  1,  4,  5],  // 6: 0110 - quad (CRITICAL)
    [ 2,  4,  5, -1],  // 7: 0111 - triangle
    [ 2,  4,  5, -1],  // 8: 1000 - triangle (mirror of 7)
    [ 0,  1,  4,  5],  // 9: 1001 - quad (mirror of 6)
    [ 0,  2,  3,  5],  // 10: 1010 - quad (mirror of 5)
    [ 1,  3,  5, -1],  // 11: 1011 - triangle (mirror of 4)
    [ 1,  2,  3,  4],  // 12: 1100 - quad (mirror of 3)
    [ 0,  3,  4, -1],  // 13: 1101 - triangle (mirror of 2)
    [ 0,  1,  2, -1],  // 14: 1110 - triangle (mirror of 1)
    [-1, -1, -1, -1],  // 15: 1111 - no intersection
];
```

**Important**: The edge ordering in this table must ensure consistent winding. If borrowed from another implementation, verify it matches your edge numbering convention.

---

## 9. Sources and Further Reading

### Primary References

1. [Marching Simplices Paper](https://www.researchgate.net/publication/236273366_Marching_Simplices) - Multi-dimensional surface extraction
2. [Isosurfacing in Higher Dimensions](https://www.sci.utah.edu/~jmk/papers/isosurfacing-in-higher-dimensions.pdf) - Bhaniramka & Wenger (222 cases for 4D)
3. [Simplex/Hyperplane Intersection](https://ics.uci.edu/~eppstein/junkyard/simplex-section.html) - David Eppstein's mathematical analysis
4. [Tessellation of 4D Spacetime](https://arxiv.org/html/2403.19036v1) - GPU implementation details
5. [Four-Space Visualization](https://hollasch.github.io/ray4/Four-Space_Visualization_of_4D_Objects.html) - Steve Hollasch's thesis

### Game Engine References

1. [Engine4D](https://github.com/HackerPoet/Engine4D) - Unity toolkit (4D Golf)
2. [hypervis](https://github.com/t-veor/hypervis) - Rust 4D physics engine
3. [four](https://github.com/mwalczyk/four) - 4D renderer with detailed docs
4. [Marching Hypercubes Code](https://github.com/lucasmb86/MarchingHypercubes) - Reference implementation

### Tutorials

1. [Unity 4D Series](https://www.alanzucconi.com/2023/07/06/understanding-the-fourth-dimension/) - Alan Zucconi
2. [Interactive 4D Handbook](https://baileysnyder.com/interactive-4d/4d-cubes/) - Bailey Snyder
3. [Tesseract Visualization](https://ciechanow.ski/tesseract/) - Bartosz Ciechanowski

---

## 10. Conclusion

The pinwheel bug is almost certainly caused by **inconsistent vertex ordering in the quadrilateral cases**. The fix involves:

1. Ensuring the lookup table edges are specified in a winding-consistent order
2. Or, sorting quadrilateral vertices by angle before triangulation
3. Verifying that all triangle normals point consistently outward

The standard algorithm is well-established and used across all major 4D engines. The key insight is that tetrahedron slicing produces at most 4 intersection points, making it much simpler than directly slicing complex polytopes.
