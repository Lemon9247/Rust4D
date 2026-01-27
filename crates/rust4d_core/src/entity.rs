//! Entity and Material types
//!
//! An Entity represents an object in the 4D world with a transform, shape, and material.

use std::sync::Arc;
use rust4d_math::ConvexShape4D;
use rust4d_physics::BodyHandle;
use crate::Transform4D;

/// A simple material with just a base color
///
/// This is minimal for now - can be extended with PBR properties later.
#[derive(Clone, Copy, Debug)]
pub struct Material {
    /// Base color as RGBA (each component 0.0-1.0)
    pub base_color: [f32; 4],
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0], // White
        }
    }
}

impl Material {
    /// Create a new material with the given RGBA color
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            base_color: [r, g, b, a],
        }
    }

    /// Create a new opaque material with the given RGB color
    pub fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// White material
    pub const WHITE: Self = Self { base_color: [1.0, 1.0, 1.0, 1.0] };

    /// Gray material
    pub const GRAY: Self = Self { base_color: [0.5, 0.5, 0.5, 1.0] };

    /// Red material
    pub const RED: Self = Self { base_color: [1.0, 0.0, 0.0, 1.0] };

    /// Green material
    pub const GREEN: Self = Self { base_color: [0.0, 1.0, 0.0, 1.0] };

    /// Blue material
    pub const BLUE: Self = Self { base_color: [0.0, 0.0, 1.0, 1.0] };
}

/// Reference to a shape - either shared (Arc) or owned (Box)
///
/// Use `Shared` for memory-efficient storage when multiple entities use the same shape.
/// Use `Owned` when an entity needs its own unique copy for modification.
pub enum ShapeRef {
    /// A shared reference to a shape (multiple entities can share this)
    Shared(Arc<dyn ConvexShape4D>),
    /// An owned shape (unique to this entity)
    Owned(Box<dyn ConvexShape4D>),
}

impl ShapeRef {
    /// Create a shared shape reference
    pub fn shared<S: ConvexShape4D + 'static>(shape: S) -> Self {
        Self::Shared(Arc::new(shape))
    }

    /// Create an owned shape reference
    pub fn owned<S: ConvexShape4D + 'static>(shape: S) -> Self {
        Self::Owned(Box::new(shape))
    }

    /// Get a reference to the underlying shape
    pub fn as_shape(&self) -> &dyn ConvexShape4D {
        match self {
            ShapeRef::Shared(arc) => arc.as_ref(),
            ShapeRef::Owned(boxed) => boxed.as_ref(),
        }
    }
}

/// An entity in the 4D world
///
/// Each entity has:
/// - A transform (position, rotation, scale)
/// - A shape (the geometry)
/// - A material (visual properties)
/// - An optional physics body handle (links to PhysicsWorld)
pub struct Entity {
    /// The entity's transform in world space
    pub transform: Transform4D,
    /// The entity's shape
    pub shape: ShapeRef,
    /// The entity's material
    pub material: Material,
    /// Optional physics body handle (links to PhysicsWorld)
    pub physics_body: Option<BodyHandle>,
}

impl Entity {
    /// Create a new entity with the given shape
    pub fn new(shape: ShapeRef) -> Self {
        Self {
            transform: Transform4D::identity(),
            shape,
            material: Material::default(),
            physics_body: None,
        }
    }

    /// Create a new entity with shape and material
    pub fn with_material(shape: ShapeRef, material: Material) -> Self {
        Self {
            transform: Transform4D::identity(),
            shape,
            material,
            physics_body: None,
        }
    }

    /// Create a new entity with shape, transform, and material
    pub fn with_transform(shape: ShapeRef, transform: Transform4D, material: Material) -> Self {
        Self {
            transform,
            shape,
            material,
            physics_body: None,
        }
    }

    /// Attach a physics body to this entity
    pub fn with_physics_body(mut self, handle: BodyHandle) -> Self {
        self.physics_body = Some(handle);
        self
    }

    /// Get the shape of this entity
    pub fn shape(&self) -> &dyn ConvexShape4D {
        self.shape.as_shape()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust4d_math::{Vec4, Tesseract4D};

    #[test]
    fn test_material_default() {
        let m = Material::default();
        assert_eq!(m.base_color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_material_new() {
        let m = Material::new(0.5, 0.6, 0.7, 0.8);
        assert_eq!(m.base_color, [0.5, 0.6, 0.7, 0.8]);
    }

    #[test]
    fn test_material_from_rgb() {
        let m = Material::from_rgb(0.5, 0.6, 0.7);
        assert_eq!(m.base_color, [0.5, 0.6, 0.7, 1.0]);
    }

    #[test]
    fn test_shape_ref_shared() {
        let tesseract = Tesseract4D::new(2.0);
        let shape_ref = ShapeRef::shared(tesseract);

        match &shape_ref {
            ShapeRef::Shared(_) => {}
            _ => panic!("Expected Shared variant"),
        }

        assert_eq!(shape_ref.as_shape().vertex_count(), 16);
    }

    #[test]
    fn test_shape_ref_owned() {
        let tesseract = Tesseract4D::new(2.0);
        let shape_ref = ShapeRef::owned(tesseract);

        match &shape_ref {
            ShapeRef::Owned(_) => {}
            _ => panic!("Expected Owned variant"),
        }

        assert_eq!(shape_ref.as_shape().vertex_count(), 16);
    }

    #[test]
    fn test_entity_new() {
        let tesseract = Tesseract4D::new(2.0);
        let entity = Entity::new(ShapeRef::shared(tesseract));

        assert_eq!(entity.shape().vertex_count(), 16);
        assert_eq!(entity.material.base_color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_entity_with_material() {
        let tesseract = Tesseract4D::new(2.0);
        let entity = Entity::with_material(
            ShapeRef::shared(tesseract),
            Material::RED,
        );

        assert_eq!(entity.material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_entity_with_transform() {
        let tesseract = Tesseract4D::new(2.0);
        let transform = Transform4D::from_position(Vec4::new(1.0, 2.0, 3.0, 4.0));
        let entity = Entity::with_transform(
            ShapeRef::shared(tesseract),
            transform,
            Material::BLUE,
        );

        assert_eq!(entity.transform.position.x, 1.0);
        assert_eq!(entity.material.base_color, [0.0, 0.0, 1.0, 1.0]);
    }
}
