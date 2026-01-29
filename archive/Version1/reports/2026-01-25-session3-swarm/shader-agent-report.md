# Shader Agent Report: 4D Cross-Section Shaders

**Date:** 2026-01-25
**Agent:** Shader Agent
**Task:** Implement WGSL shaders for 4D cross-section rendering

## Summary

Successfully created two WGSL shader files for the 4D rendering pipeline:
1. `slice.wgsl` - Compute shader for slicing 4D simplices
2. `render.wgsl` - Vertex/fragment shaders for rendering with W-depth coloring

## Files Created

### `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/slice.wgsl`

A compute shader that implements the marching simplices algorithm for 4D cross-sections.

**Key Features:**
- Workgroup size: 64 threads
- Processes one 5-cell (4D simplex) per thread
- Uses lookup tables for edge/triangle determination
- Outputs variable number of triangles per simplex (0-4)
- Atomic counter for output allocation

**Data Structures:**
- `Vertex4D`: 4D position + color (32 bytes)
- `Simplex4D`: 5 vertices (160 bytes)
- `Vertex3D`: 3D position + normal + color + w_depth (64 bytes with padding)
- `Triangle3D`: 3 vertices (192 bytes)
- `SliceParams`: slice_w + camera matrix (80 bytes)

**Algorithm:**
1. Transform all 5 vertices by the 4D camera matrix
2. Compute case index (0-31) based on which vertices are above slice_w
3. Early exit for cases 0 and 31 (no intersection)
4. Use edge_table to find crossed edges
5. Interpolate intersection points along crossed edges
6. Use tri_table to generate triangles from intersection points
7. Compute face normals for lighting
8. Atomically allocate and write output triangles

**Bindings:**
- Group 0: Input simplices, output triangles, triangle counter, slice params
- Group 1: Lookup tables (edge_table, tri_table)

### `/home/lemoneater/Projects/Rust4D/crates/rust4d_render/src/shaders/render.wgsl`

A vertex/fragment shader pair for rendering the sliced geometry with W-depth visualization.

**Key Features:**
- View/projection transformation
- W-depth coloring (blue for -W, red for +W)
- Lambertian diffuse lighting
- Vertex color blending
- Multiple fragment shader variants for debugging

**Uniforms (RenderUniforms):**
- `view_matrix`: mat4x4
- `projection_matrix`: mat4x4
- `light_direction`: vec3 (normalized direction TO light)
- `ambient_strength`: f32 (typical: 0.3)
- `diffuse_strength`: f32 (typical: 0.7)
- `w_color_strength`: f32 (0-1, how much W affects final color)
- `w_range`: f32 (max |W| value for normalization)

**Fragment Shader Variants:**
1. `fs_main` - Full rendering with lighting and W-depth blending
2. `fs_wireframe` - Pure W-depth color, no lighting
3. `fs_normals` - Normal visualization for debugging
4. `fs_w_depth_only` - W-depth with lighting, ignoring vertex colors

**W-Depth Color Mapping:**
- W < 0 (behind slice): Cool blue (0.2, 0.4, 0.9)
- W = 0 (at slice): Neutral gray (0.8, 0.8, 0.8)
- W > 0 (in front of slice): Warm red (0.9, 0.3, 0.2)

## Design Decisions

### Padding in Structs
WGSL has strict alignment requirements. I added explicit padding fields to ensure:
- vec3 fields are followed by f32 padding (16-byte alignment)
- mat4x4 fields are properly aligned (16-byte alignment)

### Edge Lookup via Constants
Rather than passing edge definitions as a buffer, I embedded them as shader constants (`EDGE_V0`, `EDGE_V1`). This is faster for a small fixed table.

### Triangle Generation
The tri_table lookup returns indices into the intersection points array. The shader iterates through groups of 3 indices until it hits a -1 sentinel or exhausts the 12-slot array.

### Normal Computation
Normals are computed per-face (flat shading) using the cross product of two triangle edges. For smoother appearance, the CPU-side code could accumulate and average normals per vertex.

### Multiple Fragment Entry Points
WGSL allows multiple entry points in one module. I provided debug variants that can be selected at pipeline creation time.

## Integration Notes

### Rust Struct Alignment
The Rust-side structs must match the WGSL layout exactly. Example:

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex3D {
    position: [f32; 3],
    _pad0: f32,
    normal: [f32; 3],
    _pad1: f32,
    color: [f32; 4],
    w_depth: f32,
    _pad2: [f32; 3],
}
```

### Lookup Table Upload
The lookup tables need to be uploaded as storage buffers:
- `edge_table`: 32 x u32 (128 bytes)
- `tri_table`: 32 x 12 x i32 (1536 bytes)

Note: The Rust lookup tables use `u16` and `i8`, but WGSL requires `u32` and `i32` for storage buffers. Convert when uploading.

### Indirect Draw
Since the triangle count is determined by the compute shader, use indirect drawing:
1. After compute pass, copy triangle_count to an indirect buffer
2. Use `draw_indirect` with the indirect buffer

## Open Questions / Future Work

1. **Smooth Normals**: Currently flat-shaded. Could implement smooth shading by averaging normals at shared vertices.

2. **Transparency**: The current blend state assumes opaque. W-depth could drive transparency for a "ghostly" 4D effect.

3. **Instancing**: If rendering multiple tesseracts, could add instance ID and per-instance transforms.

4. **Culling**: Currently no backface culling. Might want to enable for performance, but need to ensure consistent winding.

5. **Anti-aliasing**: MSAA configuration is left to the pipeline setup.

## Testing Recommendations

1. **Visual Test**: Render a tesseract at slice_w = 0. Should see a regular cube.

2. **Sweep Test**: Animate slice_w from -1 to +1. Cross-section should morph through various 3D shapes.

3. **Color Test**: Verify blue appears for negative W, red for positive W.

4. **Edge Cases**: Test with slice_w at exact vertex W coordinates.
