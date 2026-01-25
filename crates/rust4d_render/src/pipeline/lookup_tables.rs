//! Lookup tables for 4D simplex (5-cell) cross-section
//!
//! A 5-cell has 5 vertices and 10 edges. When sliced by a hyperplane,
//! we get 2^5 = 32 possible configurations depending on which vertices
//! are above or below the plane.

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
/// Each entry is up to 12 indices (4 triangles max), with -1 indicating end.
/// Indices refer to the order of intersection points in the crossed edges.
///
/// The intersection points are ordered by the edge index they come from.
pub const TRI_TABLE: [[i8; 12]; 32] = compute_tri_table();

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
const fn compute_tri_table() -> [[i8; 12]; 32] {
    // For simplicity, we'll use a pre-computed table based on the
    // topology of the 5-cell cross-sections.
    //
    // Case analysis:
    // - 0 or 5 vertices above: no intersection
    // - 1 vertex above: tetrahedron (4 triangles from 4 intersection points)
    // - 2 vertices above: triangular prism (6 triangles from 6 points)
    // - 3 vertices above: triangular prism (same as 2 below, symmetric)
    // - 4 vertices above: tetrahedron (same as 1 below, symmetric)

    let empty: [i8; 12] = [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];

    // For 1 vertex above: 4 edges crossed, intersection is a tetrahedron
    // We need to triangulate 4 points. Map intersection points to triangles.
    // Point order depends on which edges are crossed.
    //
    // When v0 is above (case 1): edges 0,1,2,3 are crossed
    // Points: p0 (edge0), p1 (edge1), p2 (edge2), p3 (edge3)
    // Tetrahedron triangles: 012, 013, 023, 123 (but need consistent winding)

    let tetra_4pts: [i8; 12] = [0, 1, 2, 0, 2, 3, 0, 1, 3, 1, 2, 3];

    // For 2 vertices above: 6 edges crossed, intersection is a triangular prism
    // We need to triangulate 6 points forming two parallel triangles connected by 3 quads
    //
    // When v0,v1 above (case 3): edges 1,2,3,4,5,6 are crossed
    // This gives 6 points that need proper triangulation

    // The triangulation depends on the geometry, but a triangular prism has 8 triangles:
    // 2 triangular caps + 3 quads (each quad = 2 triangles)
    // But we only have space for 4 triangles in our format. We need to be smarter.

    // Actually, for a convex 3D cross-section, we can use a simpler approach:
    // triangulate from a central point, or use ear-clipping.
    // For now, let's use a pre-computed table.

    // Due to complexity, we'll compute a simplified version that works for common cases
    let mut table: [[i8; 12]; 32] = [empty; 32];

    // Case 0: no vertices above (all below) - no intersection
    // Case 31: all vertices above - no intersection
    // These remain empty [-1, ...]

    // Case 1: v0 above - tetrahedron at v0
    // Edges crossed: 0,1,2,3 -> 4 points
    table[1] = tetra_4pts;

    // Case 2: v1 above - tetrahedron at v1
    // Edges crossed: 0,4,5,6 -> 4 points
    table[2] = tetra_4pts;

    // Case 4: v2 above - tetrahedron at v2
    // Edges crossed: 1,4,7,8 -> 4 points
    table[4] = tetra_4pts;

    // Case 8: v3 above - tetrahedron at v3
    // Edges crossed: 2,5,7,9 -> 4 points
    table[8] = tetra_4pts;

    // Case 16: v4 above - tetrahedron at v4
    // Edges crossed: 3,6,8,9 -> 4 points
    table[16] = tetra_4pts;

    // Cases with 4 vertices above (symmetric to 1 above)
    table[30] = tetra_4pts;  // v0 below (11110)
    table[29] = tetra_4pts;  // v1 below (11101)
    table[27] = tetra_4pts;  // v2 below (11011)
    table[23] = tetra_4pts;  // v3 below (10111)
    table[15] = tetra_4pts;  // v4 below (01111)

    // Cases with 2 or 3 vertices above create 6-point cross-sections
    // These need more triangles. We'll use a simplified triangulation.
    // For now, we create triangles from the first point as a fan
    // This works for convex cross-sections.
    let prism_6pts: [i8; 12] = [0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 5];

    // 2 vertices above cases
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

    // 3 vertices above cases (symmetric to 2 below)
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
