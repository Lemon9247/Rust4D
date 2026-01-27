//! Serializable shape templates
//!
//! ShapeTemplate provides a serializable representation of shapes,
//! solving the trait object serialization problem. Each variant
//! corresponds to a shape type and stores its construction parameters.

use serde::{Serialize, Deserialize};
use rust4d_math::{Tesseract4D, Hyperplane4D, ConvexShape4D};

/// Serializable shape template
///
/// This enum allows shapes to be serialized to/from RON files.
/// Each variant stores the parameters needed to construct the shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShapeTemplate {
    /// A 4D hypercube (tesseract)
    Tesseract {
        /// Full side length of the tesseract
        size: f32,
    },
    /// A floor/ground plane in 4D
    Hyperplane {
        /// Y-level of the plane
        y: f32,
        /// Half-extent in X and Z (total size is 2*size)
        size: f32,
        /// Number of cells along each axis
        subdivisions: u32,
        /// Half-extent in W dimension (for slicing visibility)
        cell_size: f32,
        /// Small Y thickness for proper 4D volume
        thickness: f32,
    },
}

impl ShapeTemplate {
    /// Create the actual shape from this template
    pub fn create_shape(&self) -> Box<dyn ConvexShape4D> {
        match self {
            ShapeTemplate::Tesseract { size } => {
                Box::new(Tesseract4D::new(*size))
            }
            ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size, thickness } => {
                Box::new(Hyperplane4D::new(*y, *size, *subdivisions as usize, *cell_size, *thickness))
            }
        }
    }

    /// Create a tesseract template
    pub fn tesseract(size: f32) -> Self {
        ShapeTemplate::Tesseract { size }
    }

    /// Create a hyperplane template
    pub fn hyperplane(y: f32, size: f32, subdivisions: u32, cell_size: f32, thickness: f32) -> Self {
        ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size, thickness }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tesseract_template() {
        let template = ShapeTemplate::tesseract(2.0);
        let shape = template.create_shape();
        assert_eq!(shape.vertex_count(), 16);
    }

    #[test]
    fn test_hyperplane_template() {
        let template = ShapeTemplate::hyperplane(-2.0, 4.0, 2, 2.0, 0.01);
        let shape = template.create_shape();
        // 2x2 grid = 4 cells, each with 16 vertices
        assert_eq!(shape.vertex_count(), 4 * 16);
    }

    #[test]
    fn test_tesseract_serialization() {
        let template = ShapeTemplate::tesseract(2.5);
        let serialized = ron::to_string(&template).unwrap();
        let deserialized: ShapeTemplate = ron::from_str(&serialized).unwrap();

        match deserialized {
            ShapeTemplate::Tesseract { size } => assert_eq!(size, 2.5),
            _ => panic!("Expected Tesseract variant"),
        }
    }

    #[test]
    fn test_hyperplane_serialization() {
        let template = ShapeTemplate::hyperplane(-2.0, 4.0, 4, 2.0, 0.01);
        let serialized = ron::to_string(&template).unwrap();
        let deserialized: ShapeTemplate = ron::from_str(&serialized).unwrap();

        match deserialized {
            ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size, thickness } => {
                assert_eq!(y, -2.0);
                assert_eq!(size, 4.0);
                assert_eq!(subdivisions, 4);
                assert_eq!(cell_size, 2.0);
                assert_eq!(thickness, 0.01);
            }
            _ => panic!("Expected Hyperplane variant"),
        }
    }
}
