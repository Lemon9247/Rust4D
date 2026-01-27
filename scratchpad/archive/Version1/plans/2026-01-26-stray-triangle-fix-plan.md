# Fix Plan: Stray Triangle Bug

**Created**: 2026-01-26
**Based on**: Math Accuracy Comparison Swarm Analysis
**Estimated Sessions**: 1-2

## Problem Statement

The 4D cross-section rendering shows "stray triangles" - some triangles appear to face the wrong direction, causing visual artifacts. Previous debugging attempts have not resolved the issue.

## Root Causes Identified

1. **PRIMARY**: Debug code at `slice.wgsl:435` skips orientation correction for prism cases
2. **SECONDARY**: Centroid-based orientation is position-dependent and can fail near origin
3. **ARCHITECTURAL**: Using 5-cells creates prism cases that tetrahedra-based engines avoid

## Fix Strategy

Three-phase approach, each phase producing a testable improvement:

---

## Phase 1: Remove Debug Code (IMMEDIATE)

**File**: `crates/rust4d_render/src/shaders/slice.wgsl`
**Line**: 435

### Current Code
```wgsl
// DEBUG: Skip flipping for prisms to test if that's causing issues
if (!is_prism && dot(normal, tri_center) < 0.0) {
```

### Fixed Code
```wgsl
// Flip normal if pointing toward origin (inward) instead of outward
if (dot(normal, tri_center) < 0.0) {
```

### Changes Required
1. Remove `!is_prism &&` from the condition on line 435
2. Remove or update the DEBUG comment on line 434

### Expected Result
- All 30 non-empty slice cases will have orientation correction
- Prism triangles (20 cases) will now be properly oriented
- Should fix majority of visible stray triangles

### Test Plan
1. Run the application
2. Move camera around the tesseract
3. Verify no stray triangles at any slice position
4. Test with slice_w at -1, 0, +1 (edges and center)

---

## Phase 2: Robust Orientation (IF NEEDED)

If Phase 1 doesn't fully fix the issue, implement a more robust orientation check.

### Option A: Improved Centroid Test

Ensure the centroid test handles edge cases better:

```wgsl
let tri_center = (p0 + p1 + p2) / 3.0;
let center_dist_sq = dot(tri_center, tri_center);

// Only flip if centroid is far enough from origin for reliable test
if (center_dist_sq > 0.0001) {
    if (dot(normal, tri_center) < 0.0) {
        // Flip winding
        let temp = v1;
        v1 = v2;
        v2 = temp;
        normal = -normal;
    }
} else {
    // Fallback: use simplex centroid for triangles near origin
    // Or use a consistent arbitrary direction
}
```

### Option B: Geometric Orientation (Engine4D Style)

Compute orientation based on above/below classification, not position.

**Concept**: For each triangle, determine orientation from the signed edge directions of the edges that produced the intersection points.

This requires knowing which edges produced each intersection point, which is available during the slicing computation but would need to be preserved.

**Implementation sketch**:
```wgsl
// Store edge indices with intersection points
struct IntersectionPoint {
    position: vec3<f32>,
    edge_v0: u32,  // Index of vertex with lower w
    edge_v1: u32,  // Index of vertex with higher w
}

// For triangle orientation:
// Get edge directions in reference space
// Compute scalar triple product
// If negative, flip winding
```

### Estimated Effort
- Option A: 0.5 sessions
- Option B: 1-2 sessions (requires storing edge info)

---

## Phase 3: Architectural Improvement (FUTURE)

Decompose 5-cells into tetrahedra to match Engine4D's architecture.

### Rationale
- Tetrahedra always produce triangles (no prisms)
- Simpler triangulation (1 triangle instead of 4-8)
- Proven approach used by Engine4D
- Eliminates sorting logic complexity

### Implementation
Each 5-cell can be decomposed into 5 tetrahedra by selecting one vertex as apex:

```rust
fn decompose_5cell_to_tetrahedra(vertices: [u32; 5]) -> [[u32; 4]; 5] {
    // Using vertex 4 as the "apex" for all tetrahedra
    let apex = vertices[4];
    let base = [vertices[0], vertices[1], vertices[2], vertices[3]];

    // Each tetrahedron uses the apex + 3 of the 4 base vertices
    [
        [base[0], base[1], base[2], apex], // omit base[3]
        [base[0], base[1], base[3], apex], // omit base[2]
        [base[0], base[2], base[3], apex], // omit base[1]
        [base[1], base[2], base[3], apex], // omit base[0]
        [base[0], base[1], base[2], base[3]], // The base tetrahedron itself
    ]
}
```

### Impact
- 24 5-cells → 120 tetrahedra (naive)
- Can optimize by sharing tetrahedra between 5-cells: ~80 unique
- Update lookup tables for 16 cases instead of 32
- Simplify triangle output to always be 0 or 1 triangle

### Estimated Effort
- 2-4 sessions

---

## Implementation Order

```
[Phase 1] ──────────────────────────────────────────> TEST
    │
    └── If fixed: DONE
    │
    └── If not fixed:
            │
            v
        [Phase 2A or 2B] ───────────────────────────> TEST
            │
            └── If fixed: DONE
            │
            └── If still issues:
                    │
                    v
                [Phase 3] ──────────────────────────> TEST
```

## Files to Modify

| Phase | File | Change |
|-------|------|--------|
| 1 | `slice.wgsl` | Remove debug condition |
| 2A | `slice.wgsl` | Improve centroid test |
| 2B | `slice.wgsl` | Add edge tracking |
| 2B | `lookup_tables.rs` | Add orientation flags |
| 3 | `tesseract.rs` | Decompose to tetrahedra |
| 3 | `lookup_tables.rs` | New 16-case tables |
| 3 | `slice.wgsl` | Simplify to tetrahedra |

## Verification Checklist

- [ ] No stray triangles visible at any camera angle
- [ ] Cross-section appears as solid cube (no holes)
- [ ] Lighting is consistent (no sudden dark patches)
- [ ] Works at all slice_w values (-1.0 to +1.0)
- [ ] Performance is acceptable (no regression)
- [ ] All existing tests pass

## Notes

- Engine4D uses tetrahedra stored with 4 identical vertex instances per GPU primitive
- Engine4D precomputes winding in LUT during generation (not runtime)
- The `saturate()` in Engine4D's intersection handles edge cases differently than our threshold check

---

*Plan generated from Math Accuracy Comparison Swarm analysis*
