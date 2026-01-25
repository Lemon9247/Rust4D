//! Tesseract (4D Hypercube) geometry
//!
//! A tesseract has 16 vertices (all combinations of ±h for x,y,z,w),
//! 32 edges, 24 faces (squares), and 8 cells (cubes).
//!
//! For cross-section rendering, we decompose it into 24 5-cells (4D simplices).

use rust4d_math::Vec4;

/// A tesseract (4D hypercube)
pub struct Tesseract {
    /// The 16 vertices of the tesseract
    pub vertices: [Vec4; 16],
    /// Indices into vertices forming 5-cells (4D simplices)
    /// Each 5-cell has 5 vertices
    pub simplices: Vec<[usize; 5]>,
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

        Self { vertices, simplices }
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
}
