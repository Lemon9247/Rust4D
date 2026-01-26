# Session Report: DrawIndirect Vertex Count Bug Fix

**Date**: 2026-01-26
**Session Duration**: ~30 minutes
**Focus**: Fixing the rendering pipeline bug that caused blank/incorrect output

## Summary

Fixed a critical bug in the 4D rendering pipeline where the tesseract cross-section was not rendering correctly. The root cause was a mismatch between the compute shader's atomic counter (which tracked triangle count) and the `DrawIndirect` buffer (which expects vertex count).

## The Problem

After implementing the tetrahedra-based slicing approach (decomposing 5-cells into tetrahedra), the rendering showed:
1. Initially: blank screen (no triangles visible)
2. After partial debugging: disconnected, wrongly-positioned triangles

## Root Cause Analysis

The `DrawIndirect` API expects:
```rust
struct DrawIndirectArgs {
    vertex_count: u32,    // <- This needs to be triangles * 3
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}
```

But the shader was doing:
```wgsl
let output_idx = atomicAdd(&triangle_count, 1u);  // Increments by 1 per triangle
```

The `render_pipeline.rs` copied this counter directly to the indirect buffer:
```rust
encoder.copy_buffer_to_buffer(
    counter_buffer,  // Contains triangle count
    0,
    &self.indirect_buffer,  // Expects vertex count
    0,
    ...
);
```

So for 96 triangles, the counter would be 96, but we need vertex_count = 288.

## The Fix

Modified both `slice_tetra.wgsl` and `slice.wgsl` to increment by 3:

```wgsl
// Allocate output slot atomically
// Increment by 3 because DrawIndirect needs vertex count, not triangle count
let vertex_idx = atomicAdd(&triangle_count, 3u);
let output_idx = vertex_idx / 3u;

triangles[output_idx].v0 = tv0;
triangles[output_idx].v1 = tv1;
triangles[output_idx].v2 = tv2;
```

## Debugging Approach

1. **Isolated the problem**: Created a single test tetrahedron with known geometry
   - 3 vertices at w=-1, 1 vertex at w=+1
   - Slicing at w=0 should produce exactly 1 triangle

2. **Verified with simple case**: The single tetrahedron rendered correctly after the fix

3. **Restored full geometry**: Switched back to full tesseract (84 tetrahedra) and confirmed proper cube rendering

## Files Modified

| File | Changes |
|------|---------|
| `slice_tetra.wgsl:307-310` | Changed `atomicAdd(&triangle_count, 1u)` to `atomicAdd(&triangle_count, 3u)` and added index calculation |
| `slice.wgsl:447-450` | Same fix for the 5-cell based shader |
| `src/main.rs:55-74` | Restored proper tesseract geometry generation (was temporarily using single test tetrahedron) |

## Test Results

All 45 tests pass:
- Tesseract geometry tests
- Tetrahedra decomposition tests
- Lookup table tests
- Pipeline type size tests

## Visual Verification

The tesseract cross-section at w=0 now renders as a cube with proper color gradients (RGB based on XYZ position of vertices).

## Lessons Learned

1. **Read the API docs carefully**: `DrawIndirect` expects vertex count, not primitive count. This is documented but easy to miss.

2. **TODO comments are technical debt**: The original code had a `TODO: Add a small compute shader to multiply by 3, or do it on CPU` comment that was never addressed.

3. **Simple test cases are invaluable**: Reducing from 84 tetrahedra to 1 made the problem much easier to isolate.

## What's Next

The rendering pipeline is now functional. Potential future improvements:
- Re-enable backface culling (currently disabled for debugging)
- Add wireframe mode for debugging geometry
- Optimize the tetrahedra decomposition (currently 84, could potentially reduce)

## Commit

```
01b7037 Fix DrawIndirect vertex count bug and add tetrahedra-based slicing
```
