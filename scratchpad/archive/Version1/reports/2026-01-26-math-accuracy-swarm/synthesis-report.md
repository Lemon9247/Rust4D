# Synthesis Report: Mathematical Accuracy Comparison

**Date**: 2026-01-26
**Swarm**: Math Accuracy Comparison (3 agents)

## Executive Summary

The swarm analysis has identified **three root causes** of the stray triangle bug, in order of severity:

| Priority | Issue | Impact | Fix Effort |
|----------|-------|--------|------------|
| **CRITICAL** | Debug code skips prism orientation | 67% of cases broken | 1 line change |
| **HIGH** | Centroid-based orientation is position-dependent | Edge cases near origin | Medium refactor |
| **LONG-TERM** | 5-cells are more complex than tetrahedra | Architectural | Major refactor |

## Root Cause Analysis

### 1. The `!is_prism` Debug Condition (CRITICAL)

**Location**: `slice.wgsl` line 429

```wgsl
if (!is_prism && dot(normal, tri_center) < 0.0) {
```

This condition **completely skips orientation correction for prism cases**.

**Impact**:
- 20 out of 30 non-empty slice cases are prisms (2 or 3 vertices above)
- 67% of all triangle-producing cases have no orientation fix
- Prism triangles rely solely on TRI_TABLE winding, which may not match after point sorting

**Fix**: Remove `!is_prism` - single line change.

### 2. Centroid Method vs Geometric Method (HIGH)

**Rust4D uses**: `dot(normal, tri_center) < 0` to determine if normal points inward

**Engine4D uses**: Scalar triple product of signed edge directions, precomputed in LUT

**Problems with centroid method**:
1. Triangles with centroid at/near origin give numerically unstable results
2. Adjacent simplices may compute different orientations for shared geometry
3. Position-dependent (not configuration-dependent)

**Engine4D's approach is mathematically superior** because it:
- Depends only on which vertices are above/below
- Is computed once during LUT generation
- Is consistent for all triangles in the same configuration

### 3. 5-Cells vs Tetrahedra (ARCHITECTURAL)

| Aspect | Rust4D (5-cells) | Engine4D (tetrahedra) |
|--------|------------------|----------------------|
| Vertices per primitive | 5 | 4 |
| Cases | 32 | 16 |
| Max intersection points | 6 (prism) | 3 (triangle) |
| Triangulation complexity | 4 or 8 triangles | 1 triangle |
| Point sorting required | Yes (for prisms) | No |

Engine4D's simpler primitive eliminates an entire class of bugs.

## Mathematical Equivalences Confirmed

The swarm confirmed these implementations are **mathematically equivalent**:

1. **Intersection point formula**: Both use `t = (slice_w - w0)/(w1 - w0)` with linear interpolation
2. **Case indexing**: Both use `bit i = vertex i above`
3. **Normal computation**: Both use `cross(e1, e2)` where `e1 = p1-p0, e2 = p2-p0`

The bug is **not** in these core algorithms.

## Files Involved

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `slice.wgsl` | Compute shader | Fix orientation logic |
| `lookup_tables.rs` | LUT data | Possibly add orientation flags |
| `tesseract.rs` | Geometry | If decomposing to tetrahedra |

## Verification of TRI_TABLE Winding

The analysis confirmed the prism triangulation is **correctly wound**:
- Cap A (0,2,4): CCW from one direction
- Cap B (1,5,3): CCW from opposite direction (reversed indices)
- Side triangles: Properly wound for consistent facing

The table itself is correct. The bug is the **missing runtime orientation correction**.

## Conclusion

The "stray triangles" bug has a clear primary cause: **debug code that disables orientation correction for prism cases**. Removing `!is_prism` should fix most visual issues immediately.

For robustness, implementing Engine4D's geometric orientation method would prevent edge cases near the origin.

---

*Synthesized from reports by: Rust4D Analysis Agent, Engine4D Analysis Agent, Comparison Agent*
