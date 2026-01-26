//! 4D Hyperplane geometry (floor/ground plane)
//!
//! A hyperplane in 4D is a 3D subspace. For a "floor", we create a plane
//! at constant Y that extends in X, Z, and W dimensions.
//!
//! To be sliceable by the W-plane, the hyperplane must have extent in W.
//! We model it as a grid of "pillars" - each pillar is a rectangular prism
//! extending in W, decomposed into tetrahedra.

use rust4d_math::Vec4;
use super::tesseract::Tetrahedron;

/// A checkerboard hyperplane at a fixed Y height
pub struct Hyperplane {
    /// All vertices of the hyperplane grid
    pub vertices: Vec<Vec4>,
    /// RGBA colors for each vertex
    pub colors: Vec<[f32; 4]>,
    /// Tetrahedra decomposition
    pub tetrahedra: Vec<Tetrahedron>,
}

impl Hyperplane {
    /// Create a new checkerboard hyperplane
    ///
    /// # Arguments
    /// * `y` - The Y height of the plane (e.g., -2.0 for below the tesseract)
    /// * `size` - Half-extent in X and Z (total size is 2*size)
    /// * `grid_size` - Number of cells along each axis
    /// * `cell_size` - Size of each checkerboard cell
    /// * `w_extent` - Half-extent in W dimension (for slicing visibility)
    /// * `thickness` - Small Y thickness for proper 4D volume
    pub fn new(
        y: f32,
        size: f32,
        grid_size: usize,
        cell_size: f32,
        w_extent: f32,
        thickness: f32,
    ) -> Self {
        let mut vertices = Vec::new();
        let mut colors = Vec::new();
        let mut tetrahedra = Vec::new();

        // Colors for checkerboard
        let color_a = [0.3, 0.3, 0.35, 1.0]; // Dark gray
        let color_b = [0.7, 0.7, 0.75, 1.0]; // Light gray

        let step = size * 2.0 / grid_size as f32;
        let start = -size;

        // Create grid of cells, each cell is a rectangular prism in 4D
        for i in 0..grid_size {
            for j in 0..grid_size {
                let x0 = start + i as f32 * step;
                let x1 = x0 + step;
                let z0 = start + j as f32 * step;
                let z1 = z0 + step;

                // Checkerboard pattern based on cell coordinates
                let cell_i = ((x0 + size) / cell_size).floor() as i32;
                let cell_j = ((z0 + size) / cell_size).floor() as i32;
                let color = if (cell_i + cell_j) % 2 == 0 { color_a } else { color_b };

                // Each cell is a 4D prism: 8 vertices
                // (x0,x1) × (y,y+thickness) × (z0,z1) × (-w_extent, +w_extent)
                let base_idx = vertices.len();

                let y0 = y;
                let y1 = y + thickness;
                let w0 = -w_extent;
                let w1 = w_extent;

                // 8 vertices of the rectangular prism
                // Using same binary indexing as tesseract:
                // bit 0 = x, bit 1 = y, bit 2 = z, bit 3 = w
                vertices.push(Vec4::new(x0, y0, z0, w0)); // 0 = 0b0000
                vertices.push(Vec4::new(x1, y0, z0, w0)); // 1 = 0b0001
                vertices.push(Vec4::new(x0, y1, z0, w0)); // 2 = 0b0010
                vertices.push(Vec4::new(x1, y1, z0, w0)); // 3 = 0b0011
                vertices.push(Vec4::new(x0, y0, z1, w0)); // 4 = 0b0100
                vertices.push(Vec4::new(x1, y0, z1, w0)); // 5 = 0b0101
                vertices.push(Vec4::new(x0, y1, z1, w0)); // 6 = 0b0110
                vertices.push(Vec4::new(x1, y1, z1, w0)); // 7 = 0b0111
                vertices.push(Vec4::new(x0, y0, z0, w1)); // 8 = 0b1000
                vertices.push(Vec4::new(x1, y0, z0, w1)); // 9 = 0b1001
                vertices.push(Vec4::new(x0, y1, z0, w1)); // 10 = 0b1010
                vertices.push(Vec4::new(x1, y1, z0, w1)); // 11 = 0b1011
                vertices.push(Vec4::new(x0, y0, z1, w1)); // 12 = 0b1100
                vertices.push(Vec4::new(x1, y0, z1, w1)); // 13 = 0b1101
                vertices.push(Vec4::new(x0, y1, z1, w1)); // 14 = 0b1110
                vertices.push(Vec4::new(x1, y1, z1, w1)); // 15 = 0b1111

                // All 16 vertices have the same color for this cell
                for _ in 0..16 {
                    colors.push(color);
                }

                // Decompose the tesseract-shaped cell into tetrahedra
                // Use the same Kuhn triangulation as the tesseract
                let cell_tetrahedra = Self::decompose_cell_to_tetrahedra(base_idx);
                tetrahedra.extend(cell_tetrahedra);
            }
        }

        Self {
            vertices,
            colors,
            tetrahedra,
        }
    }

    /// Decompose a single cell (mini-tesseract) into tetrahedra
    fn decompose_cell_to_tetrahedra(base_idx: usize) -> Vec<Tetrahedron> {
        // Use Kuhn triangulation - each permutation of dimensions gives a 5-cell
        // Then decompose 5-cells into tetrahedra
        let permutations = [
            [0, 1, 2, 3], [0, 1, 3, 2], [0, 2, 1, 3], [0, 2, 3, 1], [0, 3, 1, 2], [0, 3, 2, 1],
            [1, 0, 2, 3], [1, 0, 3, 2], [1, 2, 0, 3], [1, 2, 3, 0], [1, 3, 0, 2], [1, 3, 2, 0],
            [2, 0, 1, 3], [2, 0, 3, 1], [2, 1, 0, 3], [2, 1, 3, 0], [2, 3, 0, 1], [2, 3, 1, 0],
            [3, 0, 1, 2], [3, 0, 2, 1], [3, 1, 0, 2], [3, 1, 2, 0], [3, 2, 0, 1], [3, 2, 1, 0],
        ];

        let mut simplices = Vec::with_capacity(24);
        for perm in &permutations {
            let mut vertex_indices = [0usize; 5];
            let mut current = 0usize;
            vertex_indices[0] = current;
            for (i, &dim) in perm.iter().enumerate() {
                current |= 1 << dim;
                vertex_indices[i + 1] = current;
            }
            simplices.push(vertex_indices);
        }

        // Decompose 5-cells into tetrahedra
        let mut tetrahedra = Vec::new();
        let mut seen: std::collections::HashSet<[usize; 4]> = std::collections::HashSet::new();

        for simplex in &simplices {
            for omit in 0..5 {
                let mut tet_verts = [0usize; 4];
                let mut idx = 0;
                for i in 0..5 {
                    if i != omit {
                        tet_verts[idx] = base_idx + simplex[i];
                        idx += 1;
                    }
                }

                let mut canonical = tet_verts;
                canonical.sort();

                if seen.insert(canonical) {
                    tetrahedra.push(Tetrahedron::new(tet_verts));
                }
            }
        }

        tetrahedra
    }

    /// Get the number of tetrahedra
    pub fn tetrahedron_count(&self) -> usize {
        self.tetrahedra.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperplane_creation() {
        let plane = Hyperplane::new(-2.0, 4.0, 4, 1.0, 2.0, 0.01);

        // 4x4 grid = 16 cells, each with 16 vertices
        assert_eq!(plane.vertices.len(), 16 * 16);
        assert_eq!(plane.colors.len(), plane.vertices.len());
        assert!(plane.tetrahedra.len() > 0);
    }

    #[test]
    fn test_hyperplane_vertex_positions() {
        let plane = Hyperplane::new(-2.0, 4.0, 2, 1.0, 2.0, 0.01);

        // Check that all vertices are at y = -2.0 or slightly above
        for v in &plane.vertices {
            assert!(v.y >= -2.0 && v.y <= -1.99, "Vertex Y should be near -2.0, got {}", v.y);
        }
    }

    #[test]
    fn test_checkerboard_colors() {
        let plane = Hyperplane::new(-2.0, 4.0, 4, 2.0, 2.0, 0.01);

        // Check that we have at least 2 different colors
        let unique_colors: std::collections::HashSet<_> = plane.colors.iter()
            .map(|c| ((c[0] * 100.0) as i32, (c[1] * 100.0) as i32))
            .collect();
        assert!(unique_colors.len() >= 2, "Should have at least 2 colors for checkerboard");
    }
}
