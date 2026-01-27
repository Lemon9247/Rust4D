# Comparison Synthesis: Engine4D vs Rust4D

**Date**: 2026-01-26

## Critical Findings

### 1. Fundamental Architecture Difference

| Aspect | Engine4D | Rust4D |
|--------|----------|--------|
| **Primitive** | Tetrahedra (4 vertices) | 5-cells (5 vertices) |
| **Max intersection points** | 3 (always a triangle) | 6 (triangle or prism) |
| **Cases to handle** | 16 (2^4) | 32 (2^5) |
| **Triangulation complexity** | Always 1 triangle | 4 or 8 triangles |

**Impact**: Engine4D's simpler primitive makes slicing dramatically easier. A tetrahedron cross-section is ALWAYS a triangle (or degenerate). Rust4D must handle both tetrahedra (4 points) and prisms (6 points) with complex triangulation patterns.

### 2. Orientation Handling (ROOT CAUSE OF BUG)

**Engine4D**: Precomputes correct winding during LUT generation using cross-product geometric test:
```csharp
Vector3 va = (v[ia[0]] - v[ia[1]]) * (verts[ia[0]] ? 1f : -1f);
Vector3 vb = (v[ib[0]] - v[ib[1]]) * (verts[ib[0]] ? 1f : -1f);
Vector3 vc = (v[ic[0]] - v[ic[1]]) * (verts[ic[0]] ? 1f : -1f);
bool needsFlip = Vector3.Dot(va, Vector3.Cross(vb, vc)) < 0.0f;
```

**Rust4D**: Computes orientation at runtime using simplex centroid:
```wgsl
let to_centroid = simplex_centroid - tri_center;
if (dot(normal, to_centroid) > 0.0) {
    // Flip winding
}
```

**Problem**: The simplex centroid is NOT a reliable indicator of "inside" for the cross-section surface. For adjacent simplices that share internal faces, using individual simplex centroids can cause triangles to be oriented inconsistently - some facing outward, some facing inward.

### 3. Processing Model

**Engine4D**: Per-vertex processing in vertex shader
- Each vertex shader invocation produces ONE cross-section point
- Triangulation is implicit via index buffer
- No atomic operations needed

**Rust4D**: Per-simplex processing in compute shader
- Each compute invocation produces 4-8 triangles
- Requires atomic operations for output
- Must handle complex triangulation explicitly

## Why Rust4D Has Stray Triangles

Based on the comparison, the stray triangles are most likely caused by:

1. **Inconsistent normal orientation** between adjacent simplices
   - Each simplex uses its own centroid for orientation
   - Adjacent simplices may flip/not-flip triangles differently
   - Some triangles face the wrong way

2. **TRI_TABLE winding already accounts for some flips**
   - Cap B has pre-reversed winding (1,5,3 instead of 1,3,5)
   - The runtime flip logic may double-flip these
   - Result: some triangles face inward

## Recommended Fixes

### Option A: Geometric Orientation Test (Short-term)

Instead of using simplex centroid, compute orientation using the cross-product of edge directions (like Engine4D):

```wgsl
// For each triangle, compute orientation based on edge directions
// relative to the above/below classification
let edge1 = p1 - p0;
let edge2 = p2 - p0;
let computed_normal = cross(edge1, edge2);

// Determine if this normal points toward the "above" region
// If so, it's correct. If not, flip.
```

### Option B: Use Tesseract Centroid (Medium-term)

Use the tesseract's center (which is the origin) for all orientation checks, not individual simplex centroids. This ensures consistent outward orientation for the entire cube surface.

```wgsl
// All cross-section triangles should face away from the tesseract center (origin)
let to_origin = -tri_center;  // Direction from triangle to origin
if (dot(normal, to_origin) > 0.0) {
    // Normal points toward origin (inside tesseract), flip it
    // ...
}
```

### Option C: Decompose 5-cells into Tetrahedra (Long-term)

Convert Rust4D to use tetrahedra like Engine4D:
- Each 5-cell can be decomposed into 5 tetrahedra
- 24 5-cells → 120 tetrahedra (or optimize to ~80)
- Matches Engine4D's proven architecture
- Dramatically simplifies slicing logic

## Implementation Priority

1. **IMMEDIATE**: Try Option B (use origin instead of simplex centroid) - simple change
2. **IF NEEDED**: Implement Option A (geometric orientation test)
3. **FUTURE**: Consider Option C for architectural simplification

## Files to Modify

- `/home/lemoneater/Projects/Personal/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl`
  - Change normal orientation logic

## Conclusion

The root cause of the stray triangles is the **runtime normal orientation logic using simplex centroids**. Engine4D avoids this entirely by precomputing orientation and using simpler tetrahedra primitives.

The quickest fix is to use the tesseract center (origin) for all orientation checks instead of individual simplex centroids.
