//! Shape traits and primitives for 4D geometry
//!
//! This module provides the core shape abstraction for 4D objects.
//! Shapes are pure geometric data - no colors, materials, or rendering info.

use crate::Vec4;

/// A tetrahedron (3-simplex) defined by vertex indices
///
/// Tetrahedra are the fundamental building blocks for 4D slicing.
/// Each tetrahedron represents a solid region of 4D space that can
/// be sliced by a 3D hyperplane to produce triangles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tetrahedron {
    /// Indices into the parent shape's vertex array
    pub indices: [usize; 4],
}

impl Tetrahedron {
    /// Create a new tetrahedron with the given vertex indices
    #[inline]
    pub fn new(indices: [usize; 4]) -> Self {
        Self { indices }
    }

    /// Create a new tetrahedron with sorted vertex indices (canonical form)
    ///
    /// Useful for deduplication and comparison.
    pub fn new_canonical(mut indices: [usize; 4]) -> Self {
        indices.sort();
        Self { indices }
    }

    /// Get the indices as a sorted array (canonical form)
    pub fn canonical(&self) -> [usize; 4] {
        let mut sorted = self.indices;
        sorted.sort();
        sorted
    }
}

/// Trait for convex 4D shapes that can be sliced
///
/// A ConvexShape4D provides the geometric data needed for 4D rendering:
/// - Vertices: The 4D points that define the shape
/// - Tetrahedra: A decomposition into 3-simplices for slicing
///
/// Shapes are pure geometry - they contain no rendering-specific data
/// like colors or materials. That information lives in the entity/material system.
pub trait ConvexShape4D: Send + Sync {
    /// Get the vertices of this shape
    fn vertices(&self) -> &[Vec4];

    /// Get the tetrahedra decomposition of this shape
    fn tetrahedra(&self) -> &[Tetrahedron];

    /// Get the number of vertices
    #[inline]
    fn vertex_count(&self) -> usize {
        self.vertices().len()
    }

    /// Get the number of tetrahedra
    #[inline]
    fn tetrahedron_count(&self) -> usize {
        self.tetrahedra().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tetrahedron_new() {
        let tet = Tetrahedron::new([0, 1, 2, 3]);
        assert_eq!(tet.indices, [0, 1, 2, 3]);
    }

    #[test]
    fn test_tetrahedron_canonical() {
        let tet = Tetrahedron::new([3, 1, 0, 2]);
        assert_eq!(tet.canonical(), [0, 1, 2, 3]);
    }

    #[test]
    fn test_tetrahedron_new_canonical() {
        let tet = Tetrahedron::new_canonical([3, 1, 0, 2]);
        assert_eq!(tet.indices, [0, 1, 2, 3]);
    }
}
