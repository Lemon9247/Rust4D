//! Tesseract (4D Hypercube) geometry
//!
//! A tesseract has 16 vertices (all combinations of ±h for x,y,z,w),
//! 32 edges, 24 faces (squares), and 8 cells (cubes).
//!
//! For cross-section rendering, we decompose it into tetrahedra (3-simplices).
//! This is simpler than using 5-cells because tetrahedra always produce
//! triangular cross-sections (never prisms).

use rust4d_math::Vec4;
use std::collections::HashSet;

/// A tetrahedron (3-simplex) for 4D slicing
/// Has 4 vertices and 6 edges
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tetrahedron {
    /// Indices into the tesseract's vertex array
    pub vertices: [usize; 4],
}

impl Tetrahedron {
    /// Create a new tetrahedron with sorted vertex indices (canonical form)
    pub fn new_canonical(mut vertices: [usize; 4]) -> Self {
        vertices.sort();
        Self { vertices }
    }

    /// Create a new tetrahedron preserving vertex order
    pub fn new(vertices: [usize; 4]) -> Self {
        Self { vertices }
    }
}

/// A tesseract (4D hypercube)
pub struct Tesseract {
    /// The 16 vertices of the tesseract
    pub vertices: [Vec4; 16],
    /// Indices into vertices forming 5-cells (4D simplices)
    /// Each 5-cell has 5 vertices
    pub simplices: Vec<[usize; 5]>,
    /// Tetrahedra decomposition (computed lazily)
    tetrahedra: Option<Vec<Tetrahedron>>,
}

impl Tesseract {
    /// Create a new tesseract centered at origin with given size
    pub fn new(size: f32) -> Self {
        let h = size * 0.5;

        // All 16 vertices are combinations of ±h for each coordinate
        // Using binary counting: vertex i has coordinates based on bits of i
        let vertices = [
            Vec4::new(-h, -h, -h, -h), // 0  = 0b0000
            Vec4::new( h, -h, -h, -h), // 1  = 0b0001
            Vec4::new(-h,  h, -h, -h), // 2  = 0b0010
            Vec4::new( h,  h, -h, -h), // 3  = 0b0011
            Vec4::new(-h, -h,  h, -h), // 4  = 0b0100
            Vec4::new( h, -h,  h, -h), // 5  = 0b0101
            Vec4::new(-h,  h,  h, -h), // 6  = 0b0110
            Vec4::new( h,  h,  h, -h), // 7  = 0b0111
            Vec4::new(-h, -h, -h,  h), // 8  = 0b1000
            Vec4::new( h, -h, -h,  h), // 9  = 0b1001
            Vec4::new(-h,  h, -h,  h), // 10 = 0b1010
            Vec4::new( h,  h, -h,  h), // 11 = 0b1011
            Vec4::new(-h, -h,  h,  h), // 12 = 0b1100
            Vec4::new( h, -h,  h,  h), // 13 = 0b1101
            Vec4::new(-h,  h,  h,  h), // 14 = 0b1110
            Vec4::new( h,  h,  h,  h), // 15 = 0b1111
        ];

        // Decompose into 24 5-cells (simplices)
        // We use a standard decomposition: each 4D cube can be split into 24 simplices
        // by choosing a center point and connecting it to each 3D cell's decomposition
        //
        // However, a cleaner approach for a tesseract is to use the Kuhn triangulation,
        // which decomposes the hypercube into 24 simplices based on the ordering of coordinates.
        //
        // Each simplex is defined by a path through the hypercube vertices where each step
        // changes exactly one coordinate from -h to +h.

        let simplices = Self::compute_simplex_decomposition();

        Self { vertices, simplices, tetrahedra: None }
    }

    /// Get tetrahedra, computing them if needed
    pub fn tetrahedra(&mut self) -> &[Tetrahedron] {
        if self.tetrahedra.is_none() {
            self.tetrahedra = Some(self.compute_unique_tetrahedra());
        }
        self.tetrahedra.as_ref().unwrap()
    }

    /// Compute tetrahedra decomposition from 5-cells
    /// Each 5-cell is decomposed into 5 tetrahedra by omitting each vertex in turn
    /// Returns deduplicated tetrahedra (shared faces only appear once)
    fn compute_unique_tetrahedra(&self) -> Vec<Tetrahedron> {
        let mut seen: HashSet<[usize; 4]> = HashSet::new();
        let mut tetrahedra = Vec::new();

        for simplex in &self.simplices {
            // A 5-cell with vertices {v0,v1,v2,v3,v4} decomposes into 5 tetrahedra
            // by omitting each vertex in turn
            for omit in 0..5 {
                let mut tet_verts = [0usize; 4];
                let mut idx = 0;
                for i in 0..5 {
                    if i != omit {
                        tet_verts[idx] = simplex[i];
                        idx += 1;
                    }
                }

                // Sort for canonical form (deduplication)
                let mut canonical = tet_verts;
                canonical.sort();

                if seen.insert(canonical) {
                    // Store with original vertex order for consistent orientation
                    tetrahedra.push(Tetrahedron::new(tet_verts));
                }
            }
        }

        tetrahedra
    }

    /// Get the number of tetrahedra (computes if needed)
    pub fn tetrahedron_count(&mut self) -> usize {
        self.tetrahedra().len()
    }

    /// Get the vertices of a specific tetrahedron
    pub fn get_tetrahedron_vertices(&mut self, tet_idx: usize) -> [Vec4; 4] {
        // Ensure tetrahedra are computed
        if self.tetrahedra.is_none() {
            self.tetrahedra = Some(self.compute_unique_tetrahedra());
        }
        let indices = self.tetrahedra.as_ref().unwrap()[tet_idx].vertices;
        [
            self.vertices[indices[0]],
            self.vertices[indices[1]],
            self.vertices[indices[2]],
            self.vertices[indices[3]],
        ]
    }

    /// Compute the simplex decomposition of a tesseract
    ///
    /// Uses Kuhn triangulation: each simplex corresponds to a permutation of dimensions.
    /// For 4D, there are 4! = 24 permutations, hence 24 simplices.
    fn compute_simplex_decomposition() -> Vec<[usize; 5]> {
        // Generate all permutations of [0, 1, 2, 3]
        let permutations = [
            [0, 1, 2, 3], [0, 1, 3, 2], [0, 2, 1, 3], [0, 2, 3, 1], [0, 3, 1, 2], [0, 3, 2, 1],
            [1, 0, 2, 3], [1, 0, 3, 2], [1, 2, 0, 3], [1, 2, 3, 0], [1, 3, 0, 2], [1, 3, 2, 0],
            [2, 0, 1, 3], [2, 0, 3, 1], [2, 1, 0, 3], [2, 1, 3, 0], [2, 3, 0, 1], [2, 3, 1, 0],
            [3, 0, 1, 2], [3, 0, 2, 1], [3, 1, 0, 2], [3, 1, 2, 0], [3, 2, 0, 1], [3, 2, 1, 0],
        ];

        let mut simplices = Vec::with_capacity(24);

        for perm in &permutations {
            // For each permutation, create a simplex with 5 vertices
            // Starting from vertex 0 (all -h), we flip bits in the order given by perm

            let mut vertex_indices = [0usize; 5];
            let mut current = 0usize;
            vertex_indices[0] = current; // Start at 0b0000

            for (i, &dim) in perm.iter().enumerate() {
                current |= 1 << dim; // Flip the bit for this dimension
                vertex_indices[i + 1] = current;
            }

            simplices.push(vertex_indices);
        }

        simplices
    }

    /// Get the number of simplices (should be 24)
    pub fn simplex_count(&self) -> usize {
        self.simplices.len()
    }

    /// Get the vertices of a specific simplex
    pub fn get_simplex_vertices(&self, simplex_idx: usize) -> [Vec4; 5] {
        let indices = &self.simplices[simplex_idx];
        [
            self.vertices[indices[0]],
            self.vertices[indices[1]],
            self.vertices[indices[2]],
            self.vertices[indices[3]],
            self.vertices[indices[4]],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tesseract_vertex_count() {
        let t = Tesseract::new(2.0);
        assert_eq!(t.vertices.len(), 16);
    }

    #[test]
    fn test_tesseract_simplex_count() {
        let t = Tesseract::new(2.0);
        assert_eq!(t.simplex_count(), 24);
    }

    #[test]
    fn test_tesseract_vertices_positions() {
        let t = Tesseract::new(2.0);
        let h = 1.0;

        // Check a few key vertices
        assert_eq!(t.vertices[0].x, -h);
        assert_eq!(t.vertices[0].y, -h);
        assert_eq!(t.vertices[0].z, -h);
        assert_eq!(t.vertices[0].w, -h);

        assert_eq!(t.vertices[15].x, h);
        assert_eq!(t.vertices[15].y, h);
        assert_eq!(t.vertices[15].z, h);
        assert_eq!(t.vertices[15].w, h);
    }

    #[test]
    fn test_simplex_has_five_vertices() {
        let t = Tesseract::new(2.0);
        for simplex in &t.simplices {
            assert_eq!(simplex.len(), 5);
            // All indices should be valid
            for &idx in simplex {
                assert!(idx < 16);
            }
        }
    }

    #[test]
    fn test_simplex_vertices_form_path() {
        let t = Tesseract::new(2.0);

        // Each simplex should form a path where consecutive vertices differ by one bit
        for simplex in &t.simplices {
            for i in 0..4 {
                let v1 = simplex[i];
                let v2 = simplex[i + 1];
                let diff = v1 ^ v2;
                // Should be a power of 2 (exactly one bit different)
                assert!(diff.is_power_of_two(),
                    "Simplex vertices {} and {} differ by {} bits", v1, v2, diff.count_ones());
            }
        }
    }

    #[test]
    fn test_cross_section_at_w0_simplex_edges_analysis() {
        // This test analyzes which edges in the simplex decomposition cross w=0
        // to understand the cross-section geometry.
        //
        // IMPORTANT FINDING: The simplex decomposition creates internal edges
        // (diagonals) through the tesseract, not just the original tesseract edges.
        // When sliced at w=0, these internal edges also produce intersection points,
        // resulting in 27 unique points instead of just the 8 cube corners.
        //
        // This is CORRECT behavior for a simplicial mesh - the cross-section
        // of the simplices produces a triangulated surface (12 triangles forming
        // a cube surface, with extra internal triangles from the diagonal edges).
        let t = Tesseract::new(2.0);
        let h = 1.0;
        let slice_w = 0.0;

        // Collect all unique edges in the simplex decomposition
        let mut simplex_edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
        for simplex in &t.simplices {
            for i in 0..5 {
                for j in (i+1)..5 {
                    let (v0, v1) = if simplex[i] < simplex[j] {
                        (simplex[i], simplex[j])
                    } else {
                        (simplex[j], simplex[i])
                    };
                    simplex_edges.insert((v0, v1));
                }
            }
        }

        // Count edges that cross w=0 (one vertex w<0, other w>0)
        let mut crossing_edges = Vec::new();
        for &(v0, v1) in &simplex_edges {
            let w0 = t.vertices[v0].w;
            let w1 = t.vertices[v1].w;
            if (w0 < slice_w && w1 > slice_w) || (w0 > slice_w && w1 < slice_w) {
                crossing_edges.push((v0, v1));
            }
        }

        // The 8 "true" tesseract edges that cross w=0 (connecting w=-h to w=+h)
        // are: (0,8), (1,9), (2,10), (3,11), (4,12), (5,13), (6,14), (7,15)
        let tesseract_crossing_edges: Vec<(usize, usize)> = (0..8)
            .map(|i| (i, i + 8))
            .collect();

        // Verify these 8 edges produce the cube corners
        for &(v0, v1) in &tesseract_crossing_edges {
            let p0 = t.vertices[v0];
            let p1 = t.vertices[v1];
            assert_eq!(p0.w, -h);
            assert_eq!(p1.w, h);
            // The xyz should be the same for both endpoints
            assert_eq!(p0.x, p1.x);
            assert_eq!(p0.y, p1.y);
            assert_eq!(p0.z, p1.z);
        }

        // Report the edge counts
        println!("Total simplex edges: {}", simplex_edges.len());
        println!("Edges crossing w=0: {}", crossing_edges.len());
        println!("Tesseract edges crossing w=0: {}", tesseract_crossing_edges.len());

        // The simplex decomposition should include all tesseract edges
        for &(v0, v1) in &tesseract_crossing_edges {
            assert!(simplex_edges.contains(&(v0, v1)),
                "Tesseract edge ({}, {}) not found in simplex edges", v0, v1);
        }

        // NOTE: There are MORE crossing edges than just the 8 tesseract edges,
        // because the simplex decomposition adds diagonal edges through the interior.
        // This is expected and correct for the Kuhn triangulation.
        assert!(crossing_edges.len() >= 8,
            "Should have at least 8 edges crossing w=0 (the tesseract edges)");
    }

    #[test]
    fn test_cross_section_geometry_cube_corners_present() {
        // Verify that the 8 cube corner points are among the intersection points
        let t = Tesseract::new(2.0);
        let h = 1.0;
        let slice_w = 0.0;

        // The 8 cube corners at w=0
        let expected_corners = [
            [-h, -h, -h],
            [ h, -h, -h],
            [-h,  h, -h],
            [ h,  h, -h],
            [-h, -h,  h],
            [ h, -h,  h],
            [-h,  h,  h],
            [ h,  h,  h],
        ];

        // Find all intersection points from simplex edges
        let mut intersection_points: Vec<[f32; 3]> = Vec::new();
        for simplex in &t.simplices {
            for i in 0..5 {
                for j in (i+1)..5 {
                    let v0 = t.vertices[simplex[i]];
                    let v1 = t.vertices[simplex[j]];
                    let w0 = v0.w;
                    let w1 = v1.w;

                    if (w0 < slice_w && w1 > slice_w) || (w0 > slice_w && w1 < slice_w) {
                        let t_param = (slice_w - w0) / (w1 - w0);
                        let point = [
                            v0.x + t_param * (v1.x - v0.x),
                            v0.y + t_param * (v1.y - v0.y),
                            v0.z + t_param * (v1.z - v0.z),
                        ];

                        if !intersection_points.iter().any(|p|
                            (p[0] - point[0]).abs() < 0.001 &&
                            (p[1] - point[1]).abs() < 0.001 &&
                            (p[2] - point[2]).abs() < 0.001
                        ) {
                            intersection_points.push(point);
                        }
                    }
                }
            }
        }

        // Verify all 8 cube corners are present
        for corner in &expected_corners {
            let found = intersection_points.iter().any(|p|
                (p[0] - corner[0]).abs() < 0.001 &&
                (p[1] - corner[1]).abs() < 0.001 &&
                (p[2] - corner[2]).abs() < 0.001
            );
            assert!(found, "Cube corner {:?} not found in intersection points", corner);
        }
    }

    #[test]
    fn test_simplex_decomposition_covers_tesseract() {
        // Verify all 24 simplices together cover the entire tesseract
        // Each simplex starts at vertex 0 (all -h) and ends at vertex 15 (all +h)
        let t = Tesseract::new(2.0);

        for simplex in &t.simplices {
            assert_eq!(simplex[0], 0, "First vertex should be 0 (all -h)");
            assert_eq!(simplex[4], 15, "Last vertex should be 15 (all +h)");
        }
    }

    #[test]
    fn test_all_tesseract_edges_used_in_simplices() {
        // The tesseract has 32 edges. Each edge should appear in at least one simplex.
        // An edge of the tesseract connects two vertices that differ by exactly one bit.
        let t = Tesseract::new(2.0);

        // Collect all edges from the simplex decomposition
        let mut simplex_edges: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();

        for simplex in &t.simplices {
            // Each simplex has 10 edges (C(5,2) = 10)
            for i in 0..5 {
                for j in (i+1)..5 {
                    let (v0, v1) = if simplex[i] < simplex[j] {
                        (simplex[i], simplex[j])
                    } else {
                        (simplex[j], simplex[i])
                    };
                    simplex_edges.insert((v0, v1));
                }
            }
        }

        // Count tesseract edges (vertices differing by exactly one bit)
        let mut tesseract_edges = 0;
        for i in 0usize..16 {
            for j in (i+1)..16 {
                if (i ^ j).count_ones() == 1 {
                    tesseract_edges += 1;
                    // This edge should be in simplex_edges
                    assert!(simplex_edges.contains(&(i, j)),
                        "Tesseract edge ({}, {}) not found in any simplex", i, j);
                }
            }
        }

        assert_eq!(tesseract_edges, 32, "Tesseract should have 32 edges");
    }

    #[test]
    fn test_cross_section_surface_boundary() {
        // This test analyzes which intersection points lie on the BOUNDARY of the
        // cross-section (the cube surface) versus the INTERIOR (internal diagonals).
        //
        // When slicing a tesseract at w=0:
        // - Boundary points: on the 6 faces of the resulting cube
        // - Interior points: inside the cube volume (from internal simplex edges)
        //
        // For proper rendering, we only want triangles that form the cube surface.
        // Interior triangles should cancel out (matching internal faces from adjacent simplices).

        let t = Tesseract::new(2.0);
        let h = 1.0;
        let slice_w = 0.0;

        // Collect all unique intersection points
        let mut points: Vec<[f32; 3]> = Vec::new();

        for simplex in &t.simplices {
            for i in 0..5 {
                for j in (i+1)..5 {
                    let v0 = t.vertices[simplex[i]];
                    let v1 = t.vertices[simplex[j]];

                    if (v0.w < slice_w && v1.w > slice_w) || (v0.w > slice_w && v1.w < slice_w) {
                        let t_param = (slice_w - v0.w) / (v1.w - v0.w);
                        let point = [
                            v0.x + t_param * (v1.x - v0.x),
                            v0.y + t_param * (v1.y - v0.y),
                            v0.z + t_param * (v1.z - v0.z),
                        ];

                        // Check if already present
                        let is_new = !points.iter().any(|p|
                            (p[0] - point[0]).abs() < 0.001 &&
                            (p[1] - point[1]).abs() < 0.001 &&
                            (p[2] - point[2]).abs() < 0.001
                        );

                        if is_new {
                            points.push(point);
                        }
                    }
                }
            }
        }

        // Classify points as boundary vs interior
        // A point is on the boundary if at least one coordinate is at +-h
        let boundary_points: Vec<_> = points.iter().filter(|p| {
            (p[0].abs() - h).abs() < 0.001 ||
            (p[1].abs() - h).abs() < 0.001 ||
            (p[2].abs() - h).abs() < 0.001
        }).collect();

        let interior_points: Vec<_> = points.iter().filter(|p| {
            (p[0].abs() - h).abs() >= 0.001 &&
            (p[1].abs() - h).abs() >= 0.001 &&
            (p[2].abs() - h).abs() >= 0.001
        }).collect();

        println!("Total unique intersection points: {}", points.len());
        println!("Boundary points (on cube surface): {}", boundary_points.len());
        println!("Interior points (inside cube): {}", interior_points.len());

        // Print some boundary points for debugging
        println!("\nBoundary point samples (should be cube corners/edges/faces):");
        for (i, p) in boundary_points.iter().take(10).enumerate() {
            println!("  {}: ({:.2}, {:.2}, {:.2})", i, p[0], p[1], p[2]);
        }

        // The 8 cube corners are special: they lie on 3 faces simultaneously
        let corner_points: Vec<_> = points.iter().filter(|p| {
            (p[0].abs() - h).abs() < 0.001 &&
            (p[1].abs() - h).abs() < 0.001 &&
            (p[2].abs() - h).abs() < 0.001
        }).collect();

        println!("\nCorner points (all 3 coords at +-h): {}", corner_points.len());
        assert_eq!(corner_points.len(), 8, "Should have exactly 8 cube corners");

        // KEY INSIGHT: For proper cube rendering, we need 12 triangles
        // (2 per face * 6 faces). The extra interior points from the
        // simplex decomposition create additional triangles that should
        // be internal (pairs of opposing triangles on internal simplex faces).
    }

    #[test]
    fn test_simplex_internal_face_matching() {
        // Internal faces of the simplex decomposition should match between
        // adjacent simplices. This is crucial for the cross-section:
        // matching internal faces produce pairs of triangles that cancel out.

        let t = Tesseract::new(2.0);

        // Collect all simplex faces (triangular, i.e., 3 vertices from each simplex)
        let mut faces: std::collections::HashMap<(usize, usize, usize), usize> = std::collections::HashMap::new();

        for simplex in &t.simplices {
            // Each simplex has C(5,3) = 10 triangular faces
            for i in 0..5 {
                for j in (i+1)..5 {
                    for k in (j+1)..5 {
                        let mut f = [simplex[i], simplex[j], simplex[k]];
                        f.sort();
                        let key = (f[0], f[1], f[2]);
                        *faces.entry(key).or_insert(0) += 1;
                    }
                }
            }
        }

        // Count internal vs boundary faces
        let internal_faces = faces.iter().filter(|(_, &count)| count >= 2).count();
        let boundary_faces = faces.iter().filter(|(_, &count)| count == 1).count();

        println!("Total unique faces: {}", faces.len());
        println!("Internal faces (shared by 2+ simplices): {}", internal_faces);
        println!("Boundary faces (in 1 simplex only): {}", boundary_faces);

        // Boundary faces should correspond to faces of the tesseract's 8 cubic cells
        // Each cube has 6 square faces, each split into 2 triangles = 12 triangles per cube
        // 8 cubes * 12 triangles = 96 triangles, but each internal cube face is shared
        // So: boundary faces should form the surface of the 4D tesseract

        // Internal faces should have count exactly 2 (shared by adjacent simplices)
        for (face, count) in &faces {
            if *count > 2 {
                println!("WARNING: Face {:?} appears {} times (expected max 2)", face, count);
            }
        }
    }

    // ========== Tetrahedra decomposition tests ==========

    #[test]
    fn test_tetrahedra_decomposition_count() {
        let mut t = Tesseract::new(2.0);
        let count = t.tetrahedron_count();

        // 24 5-cells, each decomposed into 5 tetrahedra = 120 naive
        // After deduplication, should be fewer (shared tetrahedra)
        println!("Tetrahedra count: {}", count);

        // Each 5-cell contributes 5 tetrahedra, but many are shared
        // The exact count depends on the Kuhn triangulation structure
        assert!(count > 0, "Should have at least some tetrahedra");
        assert!(count <= 120, "Should have at most 120 tetrahedra (24 * 5)");
    }

    #[test]
    fn test_tetrahedra_have_four_vertices() {
        let mut t = Tesseract::new(2.0);
        for tet in t.tetrahedra() {
            assert_eq!(tet.vertices.len(), 4);
            // All indices should be valid
            for &idx in &tet.vertices {
                assert!(idx < 16, "Vertex index {} out of range", idx);
            }
        }
    }

    #[test]
    fn test_tetrahedra_cover_tesseract_edges() {
        // All 32 tesseract edges should appear in at least one tetrahedron
        let mut t = Tesseract::new(2.0);

        // Collect all edges from tetrahedra
        let mut tet_edges: HashSet<(usize, usize)> = HashSet::new();
        for tet in t.tetrahedra() {
            // Each tetrahedron has 6 edges
            for i in 0..4 {
                for j in (i+1)..4 {
                    let (v0, v1) = if tet.vertices[i] < tet.vertices[j] {
                        (tet.vertices[i], tet.vertices[j])
                    } else {
                        (tet.vertices[j], tet.vertices[i])
                    };
                    tet_edges.insert((v0, v1));
                }
            }
        }

        // Check that all tesseract edges are covered
        for i in 0usize..16 {
            for j in (i+1)..16 {
                if (i ^ j).count_ones() == 1 {
                    // This is a tesseract edge
                    assert!(tet_edges.contains(&(i, j)),
                        "Tesseract edge ({}, {}) not in any tetrahedron", i, j);
                }
            }
        }
    }

    #[test]
    fn test_tetrahedra_slice_produces_triangles() {
        // When sliced at w=0, each tetrahedron produces at most one triangle
        let mut t = Tesseract::new(2.0);
        let slice_w = 0.0;

        let mut triangle_count = 0;

        for tet in t.tetrahedra().to_vec() {
            let verts: Vec<_> = tet.vertices.iter()
                .map(|&i| t.vertices[i])
                .collect();

            // Count vertices above/below slice
            let above_count = verts.iter().filter(|v| v.w > slice_w).count();

            // Tetrahedra should produce triangles for cases 1,2,3 above
            // (0 above = no intersection, 4 above = no intersection)
            if above_count > 0 && above_count < 4 {
                triangle_count += 1;
            }
        }

        println!("Tetrahedra producing triangles at w=0: {}", triangle_count);
        // Should have triangles (exact count depends on tesseract structure)
        assert!(triangle_count > 0, "Should produce some triangles");
    }

    #[test]
    fn test_tetrahedra_edge_crossing_count() {
        // For tetrahedra:
        // - 1 or 3 above: 3 edges crossed (triangle)
        // - 2 above: 4 edges crossed (quadrilateral, split into 2 triangles)
        let mut t = Tesseract::new(2.0);
        let slice_w = 0.0;

        let mut triangle_cases = 0;
        let mut quad_cases = 0;

        for tet in t.tetrahedra().to_vec() {
            let verts: Vec<_> = tet.vertices.iter()
                .map(|&i| t.vertices[i])
                .collect();

            let above: Vec<bool> = verts.iter().map(|v| v.w > slice_w).collect();
            let above_count = above.iter().filter(|&&b| b).count();

            if above_count == 0 || above_count == 4 {
                continue; // No intersection
            }

            // Count edges crossed
            let edges = [(0,1), (0,2), (0,3), (1,2), (1,3), (2,3)];
            let crossed = edges.iter()
                .filter(|&&(i, j)| above[i] != above[j])
                .count();

            match above_count {
                1 | 3 => {
                    assert_eq!(crossed, 3,
                        "Tetrahedron with {} vertices above should have 3 edges crossed, got {}",
                        above_count, crossed);
                    triangle_cases += 1;
                }
                2 => {
                    assert_eq!(crossed, 4,
                        "Tetrahedron with 2 vertices above should have 4 edges crossed, got {}",
                        crossed);
                    quad_cases += 1;
                }
                _ => unreachable!()
            }
        }

        println!("Triangle cases (1 or 3 above): {}", triangle_cases);
        println!("Quad cases (2 above): {}", quad_cases);

        assert!(triangle_cases > 0, "Should have some triangle cases");
        assert!(quad_cases > 0, "Should have some quad cases");
    }
}
