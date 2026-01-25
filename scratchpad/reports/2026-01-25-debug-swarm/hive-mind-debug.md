# Hive Mind: Debug Tesseract Rendering

## Problem
The tesseract cross-section looks wrong. At w=0 (slice through center), we should see a **cube**, but instead we're seeing a pinwheel of triangles.

## Possible Causes

1. **Simplex Decomposition**: The Kuhn triangulation might not correctly decompose the tesseract into 24 5-cells
2. **Slicing Algorithm**: The compute shader might compute incorrect intersection points
3. **Lookup Tables**: The EDGE_TABLE or TRI_TABLE might have wrong values
4. **Normal Computation**: Faces might be back-facing and culled incorrectly

## Agent Assignments

### Geometry Agent
- Verify the Kuhn triangulation produces valid 5-cells
- Check that the 24 simplices cover the entire tesseract
- Verify vertex indices are correct

### Shader Agent
- Review slice.wgsl compute shader logic
- Check edge intersection calculations
- Verify triangle table indexing

## Debug Strategy
1. First verify the math: what SHOULD we see at w=0?
2. Check the simplex decomposition
3. Check the compute shader output

---

## Agent Findings

### Geometry Agent (Complete)

**Verdict: Geometry is CORRECT**

- Tesseract vertices correctly positioned at all +/-h combinations
- Kuhn triangulation produces 24 valid 5-cells
- All 24 simplices start at vertex 0, end at vertex 15
- Path property verified: consecutive vertices differ by exactly 1 bit
- All 32 tesseract edges are included in the simplex decomposition

**Cross-section analysis at w=0:**
- 27 unique intersection points (not just 8 cube corners)
- 8 corners + 12 edge midpoints + 6 face centers + 1 cube center
- This is EXPECTED due to internal simplex diagonal edges

**Root cause identified:**
The TRI_TABLE only generates 4 triangles for 6-point cases, but 8 are needed for a complete triangular prism surface.

See: `geometry-agent-report.md`

### Shader Agent (Complete)

**Fixes Applied:**
1. Division-by-zero protection in edge interpolation
2. Runtime winding normalization for consistent outward-facing normals

See: `shader-agent-report.md`

## Combined Diagnosis

The pinwheel effect is caused by:
1. **Incomplete surface triangulation** (4 triangles instead of 8 for prism cases)
2. **Inconsistent triangle winding** (now fixed by shader agent)
3. **Visible internal triangles** (need rendering pipeline adjustment)

## Remaining Work

1. Expand TRI_TABLE from 12 to 24 entries to support 8 triangles
2. Compute correct triangulation for all 32 cases
3. Consider disabling back-face culling or enabling two-sided lighting
