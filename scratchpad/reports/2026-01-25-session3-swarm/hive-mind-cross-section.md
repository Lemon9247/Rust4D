# Hive Mind: Cross-Section Pipeline Implementation

## Goal
Implement the 4D cross-section rendering pipeline that slices 5-cells and renders the 3D result.

## Completed Foundation
- Vec4, Rotor4: 4D math with geometric algebra
- Camera4D: 6-DoF camera
- Tesseract: 16 vertices, 24 5-cells (Kuhn triangulation)
- Lookup tables: EDGE_TABLE, TRI_TABLE for 32 cases
- RenderContext: wgpu window setup

## Agent Assignments

### Shader Agent
- [ ] Write `slice.wgsl`: Compute shader that slices 5-cells at w=slice_w
- [ ] Write `render.wgsl`: Vertex/fragment shaders with W-depth coloring

### Pipeline Agent
- [ ] Set up compute pipeline for slicing
- [ ] Set up render pipeline for display
- [ ] Create buffers for simplices, triangles, uniforms
- [ ] Implement indirect draw from compute output

## Coordination
- Shader Agent provides shader code
- Pipeline Agent integrates shaders into wgpu pipeline
- Both should use the existing lookup tables in `pipeline/lookup_tables.rs`

## Data Structures

### GPU Vertex (for simplices)
```rust
#[repr(C)]
struct Vertex4D {
    position: [f32; 4],  // x, y, z, w
    color: [f32; 4],     // rgba
}
```

### GPU Simplex
```rust
#[repr(C)]
struct Simplex4D {
    vertices: [Vertex4D; 5],
}
```

### GPU Output Triangle
```rust
#[repr(C)]
struct Triangle3D {
    v0: Vertex3D,
    v1: Vertex3D,
    v2: Vertex3D,
}

struct Vertex3D {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
    w_depth: f32,  // Original w-coordinate for coloring
}
```

### Uniforms
```rust
struct SliceParams {
    slice_w: f32,
    camera_matrix: [[f32; 4]; 4],
    view_matrix: [[f32; 4]; 4],
    proj_matrix: [[f32; 4]; 4],
}
```

## Questions
(Agents can add questions here for coordination)
