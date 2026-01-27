# Implementation Plan: Tetrahedra-Based 4D Slicing

**Created**: 2026-01-26
**Approach**: Option C - Decompose 5-cells into tetrahedra (like Engine4D)
**Estimated Sessions**: 2-3

## Rationale

Engine4D uses tetrahedra (4 vertices) instead of 5-cells (5 vertices). This eliminates prism cases entirely:

| Primitive | Max Intersection | Output Shape | Triangles | Complexity |
|-----------|------------------|--------------|-----------|------------|
| 5-cell | 6 points | Prism | 8 | High (sorting) |
| Tetrahedron | 3 points | Triangle | 1 | Low |

## Architecture Change Overview

```
CURRENT:
  Tesseract → 24 5-cells → 32 cases → Tetrahedra/Prisms → Complex triangulation

NEW:
  Tesseract → 24 5-cells → 120 tetrahedra → 16 cases → Single triangles
```

## Session 1: Geometry and Lookup Tables

### Task 1.1: Add Tetrahedra Decomposition

**File**: `crates/rust4d_render/src/geometry/tesseract.rs`

Each 5-cell can be decomposed into 5 tetrahedra. The 5-cell has vertices {v0, v1, v2, v3, v4}. We can decompose it by selecting one vertex as an "apex" and creating tetrahedra from the apex plus each triangular face of the opposite tetrahedron.

```rust
/// A tetrahedron (3-simplex) - 4 vertices
#[derive(Clone, Copy, Debug)]
pub struct Tetrahedron {
    pub vertices: [usize; 4],
}

impl Tesseract {
    /// Decompose all 5-cells into tetrahedra
    /// Each 5-cell becomes 5 tetrahedra, for 24 * 5 = 120 total
    /// (Can be optimized later to remove duplicates)
    pub fn compute_tetrahedra(&self) -> Vec<Tetrahedron> {
        let mut tetrahedra = Vec::with_capacity(24 * 5);

        for simplex in &self.simplices {
            // 5-cell vertices: v0, v1, v2, v3, v4
            // Decompose into 5 tetrahedra by omitting each vertex
            for omit in 0..5 {
                let mut tet = [0usize; 4];
                let mut idx = 0;
                for i in 0..5 {
                    if i != omit {
                        tet[idx] = simplex[i];
                        idx += 1;
                    }
                }
                tetrahedra.push(Tetrahedron { vertices: tet });
            }
        }

        tetrahedra
    }

    /// Compute tetrahedra with deduplication
    /// Shared tetrahedra between 5-cells are only included once
    pub fn compute_unique_tetrahedra(&self) -> Vec<Tetrahedron> {
        use std::collections::HashSet;

        let mut seen: HashSet<[usize; 4]> = HashSet::new();
        let mut tetrahedra = Vec::new();

        for simplex in &self.simplices {
            for omit in 0..5 {
                let mut tet = [0usize; 4];
                let mut idx = 0;
                for i in 0..5 {
                    if i != omit {
                        tet[idx] = simplex[i];
                        idx += 1;
                    }
                }
                // Sort for canonical form
                tet.sort();

                if seen.insert(tet) {
                    tetrahedra.push(Tetrahedron { vertices: tet });
                }
            }
        }

        tetrahedra
    }
}
```

### Task 1.2: New Lookup Tables for Tetrahedra

**File**: `crates/rust4d_render/src/pipeline/lookup_tables.rs`

Tetrahedra have 4 vertices and 6 edges:

```rust
/// Edge definitions for a tetrahedron (4 vertices, 6 edges)
pub const TETRA_EDGES: [[usize; 2]; 6] = [
    [0, 1], // Edge 0
    [0, 2], // Edge 1
    [0, 3], // Edge 2
    [1, 2], // Edge 3
    [1, 3], // Edge 4
    [2, 3], // Edge 5
];

/// For each case (0-15), which edges are crossed
/// Bit i set if edge i crosses the slice plane
pub const TETRA_EDGE_TABLE: [u8; 16] = compute_tetra_edge_table();

const fn compute_tetra_edge_table() -> [u8; 16] {
    let mut table = [0u8; 16];
    let mut case_idx: usize = 0;

    while case_idx < 16 {
        let mut edge_mask = 0u8;
        let mut edge_idx = 0;

        while edge_idx < 6 {
            let v0 = TETRA_EDGES[edge_idx][0];
            let v1 = TETRA_EDGES[edge_idx][1];

            let v0_above = (case_idx >> v0) & 1;
            let v1_above = (case_idx >> v1) & 1;

            if v0_above != v1_above {
                edge_mask |= 1 << edge_idx;
            }

            edge_idx += 1;
        }

        table[case_idx] = edge_mask;
        case_idx += 1;
    }

    table
}

/// Triangle output for each case
/// Only cases with exactly 3 crossed edges produce a triangle
/// Indices reference edges in crossing order
pub const TETRA_TRI_TABLE: [[i8; 3]; 16] = [
    [-1, -1, -1], // Case 0: all below, no intersection
    [ 0,  1,  2], // Case 1: v0 above - edges 0,1,2 crossed
    [ 0,  3,  4], // Case 2: v1 above - edges 0,3,4 crossed
    [ 1,  3,  4], // Case 3: v0,v1 above - edges 1,3,4 crossed (v2,v3 below)
    // ... fill in all 16 cases
    [-1, -1, -1], // Case 15: all above, no intersection
];
```

**Key insight**: For tetrahedra, every non-empty case produces exactly 0 or 1 triangle (3 edges crossed). This eliminates all the prism complexity.

### Task 1.3: Orientation Flags in Table

Engine4D precomputes winding. We can do the same:

```rust
/// Whether each triangle case needs winding flip
/// Computed using scalar triple product of reference tetrahedron
pub const TETRA_FLIP_TABLE: [bool; 16] = compute_flip_table();

const fn compute_flip_table() -> [bool; 16] {
    // Reference tetrahedron vertices (same as Engine4D)
    // v0 = (-1, -1, -1)
    // v1 = (-1,  1,  1)
    // v2 = ( 1, -1,  1)
    // v3 = ( 1,  1, -1)

    // For each case with 3 crossed edges:
    // 1. Get edge vectors directed from below to above
    // 2. Compute scalar triple product
    // 3. If negative, flag for flip

    let mut table = [false; 16];
    // ... implementation
    table
}
```

## Session 2: Shader Rewrite

### Task 2.1: Simplify slice.wgsl

**File**: `crates/rust4d_render/src/shaders/slice.wgsl`

The new shader will be dramatically simpler:

```wgsl
// Constants for tetrahedra
const TETRA_EDGES: array<vec2<u32>, 6> = array<vec2<u32>, 6>(
    vec2(0u, 1u), vec2(0u, 2u), vec2(0u, 3u),
    vec2(1u, 2u), vec2(1u, 3u), vec2(2u, 3u)
);

// Edge table: 16 entries, 6 bits each
const TETRA_EDGE_TABLE: array<u32, 16> = array<u32, 16>(
    0x00u, 0x07u, 0x19u, 0x1Eu, // Cases 0-3
    0x26u, 0x21u, 0x3Fu, 0x38u, // Cases 4-7
    0x38u, 0x3Fu, 0x21u, 0x26u, // Cases 8-11
    0x1Eu, 0x19u, 0x07u, 0x00u  // Cases 12-15
);

// Flip table: bit i set if case i needs winding flip
const TETRA_FLIP_FLAGS: u32 = 0x????u; // Computed from reference tet

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let tet_idx = id.x;
    if (tet_idx >= params.tetrahedron_count) { return; }

    // Load tetrahedron vertices
    let tet = tetrahedra[tet_idx];
    var v: array<vec4<f32>, 4>;
    v[0] = transform_vertex(vertices[tet.v0]);
    v[1] = transform_vertex(vertices[tet.v1]);
    v[2] = transform_vertex(vertices[tet.v2]);
    v[3] = transform_vertex(vertices[tet.v3]);

    // Compute case index (4 bits)
    var case_idx: u32 = 0u;
    if (v[0].w > params.slice_w) { case_idx |= 1u; }
    if (v[1].w > params.slice_w) { case_idx |= 2u; }
    if (v[2].w > params.slice_w) { case_idx |= 4u; }
    if (v[3].w > params.slice_w) { case_idx |= 8u; }

    let edge_mask = TETRA_EDGE_TABLE[case_idx];
    if (edge_mask == 0u) { return; } // No intersection

    // Find 3 crossed edges and compute intersection points
    var points: array<vec3<f32>, 3>;
    var pt_idx = 0u;
    for (var e = 0u; e < 6u; e++) {
        if ((edge_mask & (1u << e)) != 0u) {
            let e0 = TETRA_EDGES[e].x;
            let e1 = TETRA_EDGES[e].y;
            let t = (params.slice_w - v[e0].w) / (v[e1].w - v[e0].w);
            points[pt_idx] = mix(v[e0].xyz, v[e1].xyz, t);
            pt_idx++;
        }
    }

    // Compute normal
    let e1 = points[1] - points[0];
    let e2 = points[2] - points[0];
    var normal = normalize(cross(e1, e2));

    // Apply precomputed flip if needed
    let needs_flip = ((TETRA_FLIP_FLAGS >> case_idx) & 1u) != 0u;

    // Also check runtime orientation (for safety)
    let tri_center = (points[0] + points[1] + points[2]) / 3.0;
    let should_flip = (dot(normal, tri_center) < 0.0) != needs_flip;

    var p0 = points[0];
    var p1 = points[1];
    var p2 = points[2];

    if (should_flip) {
        let tmp = p1;
        p1 = p2;
        p2 = tmp;
        normal = -normal;
    }

    // Output single triangle
    let output_idx = atomicAdd(&triangle_count, 1u);
    triangles[output_idx].v0 = Vertex3D(p0, normal, ...);
    triangles[output_idx].v1 = Vertex3D(p1, normal, ...);
    triangles[output_idx].v2 = Vertex3D(p2, normal, ...);
}
```

### Task 2.2: Update Pipeline Types

**File**: `crates/rust4d_render/src/pipeline/types.rs`

```rust
/// Tetrahedron for GPU
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuTetrahedron {
    pub v0: u32,
    pub v1: u32,
    pub v2: u32,
    pub v3: u32,
}
```

### Task 2.3: Update Slice Pipeline

**File**: `crates/rust4d_render/src/pipeline/slice_pipeline.rs`

- Change buffer to hold tetrahedra instead of 5-cells
- Update dispatch size: `tetrahedra.len()` instead of `simplices.len()`

## Session 3: Integration and Testing

### Task 3.1: Update Tests

- Verify tetrahedra count (expect 120 naive, ~80 deduplicated)
- Verify all tesseract edges are covered
- Verify cross-section at w=0 produces correct cube

### Task 3.2: Visual Testing

- Run application
- Verify no stray triangles
- Verify cube surface is complete (no holes)
- Test at various slice_w values

### Task 3.3: Performance Comparison

- Compare frame times with old 5-cell approach
- More primitives but simpler shader - net effect TBD

## Files to Modify

| File | Changes |
|------|---------|
| `geometry/tesseract.rs` | Add tetrahedra decomposition |
| `pipeline/lookup_tables.rs` | New 16-case tables |
| `pipeline/types.rs` | Add GpuTetrahedron |
| `pipeline/slice_pipeline.rs` | Use tetrahedra buffer |
| `shaders/slice.wgsl` | Complete rewrite (simpler) |

## Rollback Plan

Keep the old 5-cell code (rename to `slice_5cell.wgsl`) in case we need to compare behavior or rollback.

## Success Criteria

1. No visible stray triangles at any camera angle
2. Cross-section is a proper cube at w=0
3. Lighting is consistent (all triangles face outward)
4. All tests pass
5. Performance is acceptable

## Mathematical Notes

### Why Tetrahedra Work Better

A tetrahedron sliced by a hyperplane produces at most 3 intersection points (always a triangle or empty). The possible cases:

- 0 above: empty
- 1 above: 3 edges from that vertex cross → triangle
- 2 above: 3 edges cross (connecting 2 above to 2 below) → triangle
- 3 above: 3 edges to the 1 below vertex cross → triangle
- 4 above: empty

**No case produces more than 3 points**, so no prism triangulation needed.

### Orientation in Tetrahedra

For a tetrahedron with vertices A, B, C, D, the "outward" normal for face ABC (opposite to D) can be determined by:

```
N = (B-A) × (C-A)
is_outward = dot(N, D-A) < 0
```

If `dot(N, D-A) > 0`, the normal points toward D (inward), so flip it.

This can be precomputed for each case in the lookup table.
