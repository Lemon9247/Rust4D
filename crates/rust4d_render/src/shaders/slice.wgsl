// 4D Cross-Section Compute Shader
//
// This shader slices 4D simplices (5-cells) with a W-hyperplane and produces
// 3D triangles for rendering.
//
// Algorithm:
// 1. For each 5-cell, transform vertices by camera matrix
// 2. Compute which vertices are above the slice plane (case index 0-31)
// 3. Use lookup tables to determine which edges are crossed
// 4. Interpolate intersection points along crossed edges
// 5. Generate triangles from intersection points

// ============================================================================
// Data Structures
// ============================================================================

/// A vertex in 4D space with color
struct Vertex4D {
    position: vec4<f32>,  // x, y, z, w
    color: vec4<f32>,     // r, g, b, a
}

/// A 4D simplex (5-cell) with 5 vertices
struct Simplex4D {
    v0: Vertex4D,
    v1: Vertex4D,
    v2: Vertex4D,
    v3: Vertex4D,
    v4: Vertex4D,
}

/// A 3D triangle vertex for output
struct Vertex3D {
    position: vec3<f32>,
    _pad0: f32,
    normal: vec3<f32>,
    _pad1: f32,
    color: vec4<f32>,
    w_depth: f32,         // Original W coordinate for coloring
    _pad2: vec3<f32>,
}

/// A 3D triangle (3 vertices)
struct Triangle3D {
    v0: Vertex3D,
    v1: Vertex3D,
    v2: Vertex3D,
}

/// Parameters for the slice operation
struct SliceParams {
    slice_w: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    camera_matrix: mat4x4<f32>,
}

// ============================================================================
// Buffers
// ============================================================================

@group(0) @binding(0) var<storage, read> simplices: array<Simplex4D>;
@group(0) @binding(1) var<storage, read_write> triangles: array<Triangle3D>;
@group(0) @binding(2) var<storage, read_write> triangle_count: atomic<u32>;
@group(0) @binding(3) var<uniform> params: SliceParams;

// Lookup tables
@group(1) @binding(0) var<storage, read> edge_table: array<u32, 32>;
@group(1) @binding(1) var<storage, read> tri_table: array<array<i32, 12>, 32>;

// ============================================================================
// Constants
// ============================================================================

// Edge definitions for a 5-cell (10 edges connecting 5 vertices)
// Each edge connects two vertices indexed 0-4
// Edge indices: 0: (0,1), 1: (0,2), 2: (0,3), 3: (0,4),
//               4: (1,2), 5: (1,3), 6: (1,4),
//               7: (2,3), 8: (2,4), 9: (3,4)
const EDGE_V0: array<u32, 10> = array<u32, 10>(0u, 0u, 0u, 0u, 1u, 1u, 1u, 2u, 2u, 3u);
const EDGE_V1: array<u32, 10> = array<u32, 10>(1u, 2u, 3u, 4u, 2u, 3u, 4u, 3u, 4u, 4u);

// ============================================================================
// Helper Functions
// ============================================================================

/// Get position of a vertex by index from a simplex
fn get_vertex_position(s: Simplex4D, idx: u32) -> vec4<f32> {
    switch(idx) {
        case 0u: { return s.v0.position; }
        case 1u: { return s.v1.position; }
        case 2u: { return s.v2.position; }
        case 3u: { return s.v3.position; }
        default: { return s.v4.position; }
    }
}

/// Get color of a vertex by index from a simplex
fn get_vertex_color(s: Simplex4D, idx: u32) -> vec4<f32> {
    switch(idx) {
        case 0u: { return s.v0.color; }
        case 1u: { return s.v1.color; }
        case 2u: { return s.v2.color; }
        case 3u: { return s.v3.color; }
        default: { return s.v4.color; }
    }
}

/// Transform a 4D position by the camera matrix
/// The camera matrix transforms 4D positions (x,y,z,w) -> (x',y',z',w')
/// We treat w as another spatial dimension, then project to 3D
fn transform_4d(pos: vec4<f32>, mat: mat4x4<f32>) -> vec4<f32> {
    return mat * pos;
}

/// Compute the intersection point on an edge between two 4D points
/// Returns (3D position, interpolated W, interpolation factor)
fn edge_intersection(
    p0: vec4<f32>,
    p1: vec4<f32>,
    c0: vec4<f32>,
    c1: vec4<f32>,
    slice_w: f32
) -> Vertex3D {
    // Compute interpolation factor based on W coordinate
    let w0 = p0.w;
    let w1 = p1.w;
    let t = (slice_w - w0) / (w1 - w0);

    // Interpolate position
    let pos = mix(p0, p1, t);

    // Interpolate color
    let color = mix(c0, c1, t);

    // The W coordinate at intersection is slice_w by definition
    // But we store the interpolated W for depth coloring
    let w_depth = slice_w;

    var vertex: Vertex3D;
    vertex.position = pos.xyz;
    vertex.color = color;
    vertex.w_depth = w_depth;
    vertex.normal = vec3<f32>(0.0, 0.0, 0.0); // Will be computed later
    return vertex;
}

/// Compute normal from three 3D points
fn compute_normal(p0: vec3<f32>, p1: vec3<f32>, p2: vec3<f32>) -> vec3<f32> {
    let e1 = p1 - p0;
    let e2 = p2 - p0;
    return normalize(cross(e1, e2));
}

// ============================================================================
// Main Compute Shader
// ============================================================================

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let simplex_idx = global_id.x;
    let num_simplices = arrayLength(&simplices);

    if (simplex_idx >= num_simplices) {
        return;
    }

    let simplex = simplices[simplex_idx];
    let slice_w = params.slice_w;
    let camera_mat = params.camera_matrix;

    // Transform all vertices by camera matrix
    var transformed: array<vec4<f32>, 5>;
    var colors: array<vec4<f32>, 5>;

    transformed[0] = transform_4d(simplex.v0.position, camera_mat);
    transformed[1] = transform_4d(simplex.v1.position, camera_mat);
    transformed[2] = transform_4d(simplex.v2.position, camera_mat);
    transformed[3] = transform_4d(simplex.v3.position, camera_mat);
    transformed[4] = transform_4d(simplex.v4.position, camera_mat);

    colors[0] = simplex.v0.color;
    colors[1] = simplex.v1.color;
    colors[2] = simplex.v2.color;
    colors[3] = simplex.v3.color;
    colors[4] = simplex.v4.color;

    // Compute case index: which vertices are above the slice plane
    var case_idx: u32 = 0u;
    if (transformed[0].w > slice_w) { case_idx |= 1u; }
    if (transformed[1].w > slice_w) { case_idx |= 2u; }
    if (transformed[2].w > slice_w) { case_idx |= 4u; }
    if (transformed[3].w > slice_w) { case_idx |= 8u; }
    if (transformed[4].w > slice_w) { case_idx |= 16u; }

    // Skip if no intersection (all vertices on same side)
    if (case_idx == 0u || case_idx == 31u) {
        return;
    }

    // Get edge mask from lookup table
    let edge_mask = edge_table[case_idx];

    // Compute intersection points for all crossed edges
    var intersection_points: array<Vertex3D, 10>;
    var point_count: u32 = 0u;

    for (var edge_idx: u32 = 0u; edge_idx < 10u; edge_idx++) {
        if ((edge_mask & (1u << edge_idx)) != 0u) {
            let v0_idx = EDGE_V0[edge_idx];
            let v1_idx = EDGE_V1[edge_idx];

            intersection_points[point_count] = edge_intersection(
                transformed[v0_idx],
                transformed[v1_idx],
                colors[v0_idx],
                colors[v1_idx],
                slice_w
            );
            point_count++;
        }
    }

    // Generate triangles from lookup table
    let tri_indices = tri_table[case_idx];

    // Process triangles (up to 4 triangles, 12 indices)
    var tri_idx: u32 = 0u;
    while (tri_idx < 12u) {
        let i0 = tri_indices[tri_idx];

        // Check for end marker
        if (i0 < 0) {
            break;
        }

        let i1 = tri_indices[tri_idx + 1u];
        let i2 = tri_indices[tri_idx + 2u];

        // Get the three vertices
        var v0 = intersection_points[u32(i0)];
        var v1 = intersection_points[u32(i1)];
        var v2 = intersection_points[u32(i2)];

        // Compute and assign normal
        let normal = compute_normal(v0.position, v1.position, v2.position);
        v0.normal = normal;
        v1.normal = normal;
        v2.normal = normal;

        // Allocate output slot atomically
        let output_idx = atomicAdd(&triangle_count, 1u);

        // Write triangle to output
        triangles[output_idx].v0 = v0;
        triangles[output_idx].v1 = v1;
        triangles[output_idx].v2 = v2;

        tri_idx += 3u;
    }
}
