//! Lookup tables for 4D simplex (5-cell) cross-section
//!
//! A 5-cell has 5 vertices and 10 edges. When sliced by a hyperplane,
//! we get 2^5 = 32 possible configurations depending on which vertices
//! are above or below the plane.
//!
//! Cross-section types:
//! - 0 or 5 vertices above: no intersection
//! - 1 or 4 vertices above: tetrahedron (4 triangles, 4 intersection points)
//! - 2 or 3 vertices above: triangular prism (8 triangles, 6 intersection points)

/// Edge definitions for a 5-cell
/// Each edge connects two vertices (indexed 0-4)
pub const EDGES: [[usize; 2]; 10] = [
    [0, 1], // Edge 0
    [0, 2], // Edge 1
    [0, 3], // Edge 2
    [0, 4], // Edge 3
    [1, 2], // Edge 4
    [1, 3], // Edge 5
    [1, 4], // Edge 6
    [2, 3], // Edge 7
    [2, 4], // Edge 8
    [3, 4], // Edge 9
];

/// For each case (0-31), which edges are crossed by the slice plane.
/// Bit i is set if edge i is crossed (i.e., its endpoints are on opposite sides).
///
/// An edge is crossed when: (vertex_a above XOR vertex_b above)
pub const EDGE_TABLE: [u16; 32] = compute_edge_table();

/// Triangle table: for each case, how to form triangles from intersection points.
/// Each entry is up to 24 indices (8 triangles max), with -1 indicating end.
/// Indices refer to the order of intersection points in the crossed edges.
///
/// The intersection points are ordered by the edge index they come from.
///
/// For tetrahedron cases (4 points): 4 triangles covering the tetrahedron surface
/// For prism cases (6 points): 8 triangles (2 caps + 3 quads split into 6 triangles)
pub const TRI_TABLE: [[i8; 24]; 32] = compute_tri_table();

/// Compute the edge table at compile time
const fn compute_edge_table() -> [u16; 32] {
    let mut table = [0u16; 32];
    let mut case_idx: usize = 0;

    while case_idx < 32 {
        let mut edge_mask = 0u16;
        let mut edge_idx = 0;

        while edge_idx < 10 {
            let v0 = EDGES[edge_idx][0];
            let v1 = EDGES[edge_idx][1];

            // Check if vertex is above based on case bits
            let v0_above = (case_idx >> v0) & 1;
            let v1_above = (case_idx >> v1) & 1;

            // Edge is crossed if vertices are on opposite sides
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

/// Compute the triangle table at compile time
/// This is more complex as we need to properly triangulate each case
const fn compute_tri_table() -> [[i8; 24]; 32] {
    // Case analysis:
    // - 0 or 5 vertices above: no intersection
    // - 1 or 4 vertices above: tetrahedron (4 triangles from 4 points)
    // - 2 or 3 vertices above: triangular prism (8 triangles from 6 points)

    let empty: [i8; 24] = [-1; 24];

    // Tetrahedron: 4 points forming 4 triangular faces
    // All combinations of 3 points from 4: (0,1,2), (0,1,3), (0,2,3), (1,2,3)
    let tetra_4pts: [i8; 24] = [
        0, 1, 2,  // face 0
        0, 2, 3,  // face 1
        0, 3, 1,  // face 2
        1, 3, 2,  // face 3
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1  // unused
    ];

    // Triangular prism: 6 points forming 8 triangles
    // Points 0,1,2 form one triangular cap (from first "above" vertex)
    // Points 3,4,5 form other triangular cap (from second "above" vertex)
    // The prism sides connect corresponding edges:
    //   - Points 0,3 share connection to same "below" vertex
    //   - Points 1,4 share connection to same "below" vertex
    //   - Points 2,5 share connection to same "below" vertex
    //
    // Triangulation:
    //   Cap A: 0-1-2
    //   Cap B: 5-4-3 (reversed winding for opposite face)
    //   Side 1 (quad 0-1-4-3): triangles 0-1-4, 0-4-3
    //   Side 2 (quad 1-2-5-4): triangles 1-2-5, 1-5-4
    //   Side 3 (quad 2-0-3-5): triangles 2-0-3, 2-3-5
    let prism_6pts: [i8; 24] = [
        0, 1, 2,  // cap A
        5, 4, 3,  // cap B (opposite winding)
        0, 1, 4,  // side 1a
        0, 4, 3,  // side 1b
        1, 2, 5,  // side 2a
        1, 5, 4,  // side 2b
        2, 0, 3,  // side 3a
        2, 3, 5,  // side 3b
    ];

    let mut table: [[i8; 24]; 32] = [empty; 32];

    // Case 0: no vertices above (all below) - no intersection
    // Case 31: all vertices above - no intersection
    // These remain empty [-1, ...]

    // === Tetrahedron cases (1 vertex above) ===
    // Case 1: v0 above - edges 0,1,2,3 crossed
    table[1] = tetra_4pts;
    // Case 2: v1 above - edges 0,4,5,6 crossed
    table[2] = tetra_4pts;
    // Case 4: v2 above - edges 1,4,7,8 crossed
    table[4] = tetra_4pts;
    // Case 8: v3 above - edges 2,5,7,9 crossed
    table[8] = tetra_4pts;
    // Case 16: v4 above - edges 3,6,8,9 crossed
    table[16] = tetra_4pts;

    // === Tetrahedron cases (4 vertices above = 1 below) ===
    table[30] = tetra_4pts;  // v0 below
    table[29] = tetra_4pts;  // v1 below
    table[27] = tetra_4pts;  // v2 below
    table[23] = tetra_4pts;  // v3 below
    table[15] = tetra_4pts;  // v4 below

    // === Prism cases (2 vertices above) ===
    // Use prism triangulation: 2 caps + 3 quads = 8 triangles
    table[3] = prism_6pts;   // v0,v1 above
    table[5] = prism_6pts;   // v0,v2 above
    table[6] = prism_6pts;   // v1,v2 above
    table[9] = prism_6pts;   // v0,v3 above
    table[10] = prism_6pts;  // v1,v3 above
    table[12] = prism_6pts;  // v2,v3 above
    table[17] = prism_6pts;  // v0,v4 above
    table[18] = prism_6pts;  // v1,v4 above
    table[20] = prism_6pts;  // v2,v4 above
    table[24] = prism_6pts;  // v3,v4 above

    // === Prism cases (3 vertices above = 2 below) ===
    table[7] = prism_6pts;   // v0,v1,v2 above (v3,v4 below)
    table[11] = prism_6pts;  // v0,v1,v3 above
    table[13] = prism_6pts;  // v0,v2,v3 above
    table[14] = prism_6pts;  // v1,v2,v3 above
    table[19] = prism_6pts;  // v0,v1,v4 above
    table[21] = prism_6pts;  // v0,v2,v4 above
    table[22] = prism_6pts;  // v1,v2,v4 above
    table[25] = prism_6pts;  // v0,v3,v4 above
    table[26] = prism_6pts;  // v1,v3,v4 above
    table[28] = prism_6pts;  // v2,v3,v4 above

    table
}

/// Get the number of edges crossed for a given case
pub const fn edge_count(case_idx: usize) -> usize {
    EDGE_TABLE[case_idx].count_ones() as usize
}

/// Get the list of crossed edge indices for a given case
pub fn crossed_edges(case_idx: usize) -> Vec<usize> {
    let mask = EDGE_TABLE[case_idx];
    (0..10).filter(|i| (mask >> i) & 1 == 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_table_case_0() {
        // All vertices below - no edges crossed
        assert_eq!(EDGE_TABLE[0], 0);
    }

    #[test]
    fn test_triangle_table_coverage() {
        // Verify that all non-empty cases have enough triangles
        // to properly cover the cross-section surface.

        for case_idx in 0..32 {
            let edge_count = EDGE_TABLE[case_idx].count_ones();
            let mut tri_count = 0;

            for i in (0..24).step_by(3) {
                if TRI_TABLE[case_idx][i] >= 0 {
                    tri_count += 1;
                } else {
                    break;
                }
            }

            match edge_count {
                0 => assert_eq!(tri_count, 0, "Case {} has 0 edges but {} triangles", case_idx, tri_count),
                4 => {
                    // 4 intersection points form a tetrahedron (4 triangular faces)
                    assert_eq!(tri_count, 4,
                        "Case {} has 4 points (tetrahedron) but only {} triangles (need 4)",
                        case_idx, tri_count);
                }
                6 => {
                    // 6 intersection points form a triangular prism (8 triangles)
                    // 2 triangular caps + 3 quads (each split into 2 triangles)
                    assert_eq!(tri_count, 8,
                        "Case {} has 6 points (prism) but only {} triangles (need 8)",
                        case_idx, tri_count);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_edge_table_case_31() {
        // All vertices above - no edges crossed
        assert_eq!(EDGE_TABLE[31], 0);
    }

    #[test]
    fn test_edge_table_case_1() {
        // v0 above - edges 0,1,2,3 crossed (connecting v0 to v1,v2,v3,v4)
        let expected = 0b0000001111; // edges 0,1,2,3
        assert_eq!(EDGE_TABLE[1], expected);
    }

    #[test]
    fn test_edge_table_case_2() {
        // v1 above - edges connecting v1 to v0,v2,v3,v4
        // Edge 0: v0-v1 (crossed)
        // Edge 4: v1-v2 (crossed)
        // Edge 5: v1-v3 (crossed)
        // Edge 6: v1-v4 (crossed)
        let expected = 0b0001110001; // edges 0,4,5,6
        assert_eq!(EDGE_TABLE[2], expected);
    }

    #[test]
    fn test_edge_table_case_3() {
        // v0,v1 above - 6 edges crossed
        // Not crossed: edge 0 (v0-v1 both above)
        // Crossed: 1,2,3 (from v0), 4,5,6 (from v1)
        let expected = 0b0001111110; // edges 1,2,3,4,5,6
        assert_eq!(EDGE_TABLE[3], expected);
    }

    #[test]
    fn test_symmetry() {
        // Case i and case (31-i) should have same number of edges crossed
        for i in 0..16 {
            assert_eq!(
                EDGE_TABLE[i].count_ones(),
                EDGE_TABLE[31 - i].count_ones(),
                "Cases {} and {} should have same edge count", i, 31 - i
            );
        }
    }

    #[test]
    fn test_edge_count_distribution() {
        // 0 edges: cases 0 and 31
        // 4 edges: single vertex cases (5 cases each for 1 and 4 vertices above)
        // 6 edges: 2 or 3 vertices above

        let count_0 = (0..32).filter(|&i| EDGE_TABLE[i].count_ones() == 0).count();
        let count_4 = (0..32).filter(|&i| EDGE_TABLE[i].count_ones() == 4).count();
        let count_6 = (0..32).filter(|&i| EDGE_TABLE[i].count_ones() == 6).count();

        assert_eq!(count_0, 2);  // cases 0 and 31
        assert_eq!(count_4, 10); // C(5,1) + C(5,4) = 5 + 5
        assert_eq!(count_6, 20); // C(5,2) + C(5,3) = 10 + 10
    }
}
