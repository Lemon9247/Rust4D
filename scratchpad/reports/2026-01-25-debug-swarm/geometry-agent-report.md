# Geometry Agent Debug Report

## Summary

I analyzed the tesseract geometry and Kuhn triangulation to understand the "pinwheel of triangles" issue when slicing at w=0. The **geometry is correct** - the tesseract vertices and simplex decomposition are mathematically valid. The issue lies in how the cross-section triangles are rendered.

## Key Findings

### 1. Tesseract Vertices (CORRECT)

The 16 vertices are correctly positioned at all combinations of +/-h for each coordinate:

```
Vertex 0  (0b0000): (-h, -h, -h, -h)
Vertex 1  (0b0001): (+h, -h, -h, -h)
Vertex 2  (0b0010): (-h, +h, -h, -h)
...
Vertex 15 (0b1111): (+h, +h, +h, +h)
```

The bit-encoding scheme is correct: bit i of the vertex index determines the sign of dimension i.

### 2. Kuhn Triangulation (CORRECT)

The 24 simplices are correctly generated from the 4! = 24 permutations of [0,1,2,3]:

```rust
// For permutation [2, 0, 3, 1], the simplex path is:
// v0 = 0b0000 (start)
// v1 = 0b0100 (flip bit 2)
// v2 = 0b0101 (flip bit 0)
// v3 = 0b1101 (flip bit 3)
// v4 = 0b1111 (flip bit 1 = end)
```

Each simplex:
- Starts at vertex 0 (all -h)
- Ends at vertex 15 (all +h)
- Consecutive vertices differ by exactly one bit
- Forms a valid path through the tesseract

### 3. Cross-Section Geometry at w=0

When slicing the tesseract at w=0, the simplex decomposition produces:

| Metric | Value |
|--------|-------|
| Total simplex edges | 65 |
| Edges crossing w=0 | 27 |
| Tesseract edges crossing w=0 | 8 |
| Unique intersection points | 27 |
| Points on cube surface | 26 |
| Interior points | 1 (center at origin) |
| Cube corners (at +/-h) | 8 |

The 27 intersection points consist of:
- **8 corners**: All coordinates at +/-h (e.g., (+1, +1, +1))
- **12 edge midpoints**: Two coordinates at +/-h, one at 0 (e.g., (+1, +1, 0))
- **6 face centers**: One coordinate at +/-h, two at 0 (e.g., (+1, 0, 0))
- **1 cube center**: (0, 0, 0)

### 4. Why This Produces Extra Triangles

The Kuhn triangulation decomposes the tesseract into 24 simplices with **internal diagonal edges**. These diagonals create additional intersection points beyond the 8 cube corners.

When the compute shader slices each simplex:
1. It finds 4-6 intersection points per simplex
2. It generates 4 triangles using the lookup table
3. Adjacent simplices share some intersection points

The result is a **triangulated cube surface with interior triangulation** - more triangles than the minimum 12 needed for a cube surface.

### 5. The Real Issue: Triangle Table Deficit

The lookup table `TRI_TABLE` only allocates space for 4 triangles per case:

```rust
// For 6-point cases (2 or 3 vertices above slice plane):
// - A triangular prism surface needs 8 triangles
// - Current table only generates 4 (fan triangulation)
let prism_6pts: [i8; 12] = [0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5];
```

This creates **incomplete surfaces** for the 6-point cases (cases 3, 5, 6, 7, 9, 10, 11, 12, 13, 14, 17, 18, 19, 20, 21, 22, 24, 25, 26, 28).

Test output confirms this:
```
WARNING: Case 3 has 6 points (prism) but only 4 triangles (need 8 for closed surface)
WARNING: Case 5 has 6 points (prism) but only 4 triangles (need 8 for closed surface)
... (20 warnings total)
```

### 6. Internal Face Sharing Analysis

The simplex decomposition creates 110 unique triangular faces:
- **24 boundary faces** (on tesseract surface)
- **86 internal faces** (shared between simplices)

However, some internal faces are shared by **more than 2 simplices**:

```
Face (0, 7, 15) appears 6 times
Face (0, 11, 15) appears 6 times
... (14 faces appear 4-6 times)
```

This happens because the main diagonal (0 to 15) is shared by all 24 simplices, and faces containing this diagonal are shared multiple times.

## Recommendations

### Priority 1: Fix TRI_TABLE Size

The current 12-entry triangle table cannot represent 8 triangles. Options:
1. **Increase table width** to 24 entries (8 triangles * 3 vertices)
2. **Split 6-point cases** into multiple passes
3. **Use runtime triangulation** for complex cases

### Priority 2: Handle Internal Cancellation

For proper rendering, internal triangles should cancel out. The shader's winding normalization (added by Shader Agent) helps, but **back-face culling must be disabled** or internal faces will show gaps.

### Priority 3: Verify Simplex Orientation

All simplices should have consistent orientation. Currently:
- All start at vertex 0, end at vertex 15
- Adjacent simplices share faces but may have opposite windings

## Tests Added

I added two new tests to verify the geometry:

### `test_cross_section_surface_boundary`
Verifies that the 8 cube corners are present and classifies intersection points as boundary vs interior.

### `test_simplex_internal_face_matching`
Analyzes face sharing between simplices and identifies the high-sharing internal faces.

## Files Modified

- `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/geometry/tesseract.rs` - Added diagnostic tests

## Conclusion

The tesseract geometry and Kuhn triangulation are **mathematically correct**. The pinwheel effect is caused by:

1. **Incomplete triangulation** for 6-point cases (TRI_TABLE only generates 4 of 8 needed triangles)
2. **Inconsistent winding** causing some triangles to face inward (partially fixed by Shader Agent)
3. **Internal triangles visible** when they shouldn't be (need proper occlusion or cancellation)

The Shader Agent's winding normalization fix addresses issue #2. Issues #1 and #3 require changes to the lookup tables and/or rendering pipeline.
