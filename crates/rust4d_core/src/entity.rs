//! Entity and Material types
//!
//! An Entity represents an object in the 4D world with a transform, shape, and material.

use std::collections::HashSet;
use std::sync::Arc;
use bitflags::bitflags;
use rust4d_math::ConvexShape4D;
use rust4d_physics::BodyKey;
use serde::{Serialize, Deserialize};
use crate::Transform4D;
use crate::shapes::ShapeTemplate;

bitflags! {
    /// Flags indicating which parts of an entity have changed and need updating
    ///
    /// Used for dirty tracking to avoid rebuilding all geometry when only
    /// some entities have changed.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct DirtyFlags: u8 {
        /// No changes
        const NONE = 0;
        /// Transform (position, rotation, scale) has changed
        const TRANSFORM = 1 << 0;
        /// Mesh/shape has changed
        const MESH = 1 << 1;
        /// Material has changed
        const MATERIAL = 1 << 2;
        /// All flags set - entity needs full rebuild
        const ALL = Self::TRANSFORM.bits() | Self::MESH.bits() | Self::MATERIAL.bits();
    }
}

/// A simple material with just a base color
///
/// This is minimal for now - can be extended with PBR properties later.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
/// - An optional name (for lookup by name)
/// - Tags (for categorization and filtering)
/// - A transform (position, rotation, scale)
/// - A shape (the geometry)
/// - A material (visual properties)
/// - An optional physics body key (links to PhysicsWorld)
/// - Dirty flags (for change tracking)
pub struct Entity {
    /// Optional name for this entity (for lookup)
    pub name: Option<String>,
    /// Tags for categorization (e.g., "dynamic", "static", "enemy")
    pub tags: HashSet<String>,
    /// The entity's transform in world space
    pub transform: Transform4D,
    /// The entity's shape
    pub shape: ShapeRef,
    /// The entity's material
    pub material: Material,
    /// Optional physics body key (links to PhysicsWorld)
    pub physics_body: Option<BodyKey>,
    /// Dirty flags for change tracking (what needs rebuilding)
    dirty: DirtyFlags,
}

impl Entity {
    /// Create a new entity with the given shape
    pub fn new(shape: ShapeRef) -> Self {
        Self {
            name: None,
            tags: HashSet::new(),
            transform: Transform4D::identity(),
            shape,
            material: Material::default(),
            physics_body: None,
            dirty: DirtyFlags::ALL, // New entities are dirty
        }
    }

    /// Create a new entity with shape and material
    pub fn with_material(shape: ShapeRef, material: Material) -> Self {
        Self {
            name: None,
            tags: HashSet::new(),
            transform: Transform4D::identity(),
            shape,
            material,
            physics_body: None,
            dirty: DirtyFlags::ALL, // New entities are dirty
        }
    }

    /// Create a new entity with shape, transform, and material
    pub fn with_transform(shape: ShapeRef, transform: Transform4D, material: Material) -> Self {
        Self {
            name: None,
            tags: HashSet::new(),
            transform,
            shape,
            material,
            physics_body: None,
            dirty: DirtyFlags::ALL, // New entities are dirty
        }
    }

    /// Set the name of this entity (for lookup)
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a tag to this entity
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Add multiple tags to this entity
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for tag in tags {
            self.tags.insert(tag.into());
        }
        self
    }

    /// Check if this entity has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Attach a physics body to this entity
    pub fn with_physics_body(mut self, key: BodyKey) -> Self {
        self.physics_body = Some(key);
        self
    }

    /// Get the shape of this entity
    pub fn shape(&self) -> &dyn ConvexShape4D {
        self.shape.as_shape()
    }

    // --- Dirty tracking methods ---

    /// Check if this entity has any dirty flags set
    #[inline]
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Get the current dirty flags
    #[inline]
    pub fn dirty_flags(&self) -> DirtyFlags {
        self.dirty
    }

    /// Mark this entity as dirty with the given flags
    #[inline]
    pub fn mark_dirty(&mut self, flags: DirtyFlags) {
        self.dirty |= flags;
    }

    /// Clear all dirty flags
    #[inline]
    pub fn clear_dirty(&mut self) {
        self.dirty = DirtyFlags::NONE;
    }

    /// Set the position and mark the transform as dirty
    pub fn set_position(&mut self, position: rust4d_math::Vec4) {
        self.transform.position = position;
        self.mark_dirty(DirtyFlags::TRANSFORM);
    }

    /// Set the transform and mark it as dirty
    pub fn set_transform(&mut self, transform: Transform4D) {
        self.transform = transform;
        self.mark_dirty(DirtyFlags::TRANSFORM);
    }

    /// Set the material and mark it as dirty
    pub fn set_material(&mut self, material: Material) {
        self.material = material;
        self.mark_dirty(DirtyFlags::MATERIAL);
    }
}

/// A serializable entity template
///
/// EntityTemplate is used for scene serialization. Unlike Entity, it stores
/// a ShapeTemplate (enum) rather than a trait object, making it serializable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTemplate {
    /// Optional name for this entity (for lookup)
    pub name: Option<String>,
    /// Tags for categorization (e.g., "dynamic", "static")
    #[serde(default)]
    pub tags: Vec<String>,
    /// The entity's transform in world space
    pub transform: Transform4D,
    /// The entity's shape template (serializable)
    pub shape: ShapeTemplate,
    /// The entity's material
    pub material: Material,
}

impl EntityTemplate {
    /// Create a new entity template
    pub fn new(shape: ShapeTemplate, transform: Transform4D, material: Material) -> Self {
        Self {
            name: None,
            tags: Vec::new(),
            transform,
            shape,
            material,
        }
    }

    /// Set the name of this template
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a tag to this template
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Convert this template to an Entity
    pub fn to_entity(&self) -> Entity {
        let shape = self.shape.create_shape();
        let mut entity = Entity::with_transform(
            ShapeRef::Owned(shape),
            self.transform,
            self.material,
        );
        if let Some(ref name) = self.name {
            entity = entity.with_name(name.clone());
        }
        for tag in &self.tags {
            entity = entity.with_tag(tag.clone());
        }
        entity
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

    // --- Dirty tracking tests ---

    #[test]
    fn test_dirty_flags_default() {
        let flags = DirtyFlags::default();
        assert_eq!(flags, DirtyFlags::NONE);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_dirty_flags_all() {
        let flags = DirtyFlags::ALL;
        assert!(flags.contains(DirtyFlags::TRANSFORM));
        assert!(flags.contains(DirtyFlags::MESH));
        assert!(flags.contains(DirtyFlags::MATERIAL));
    }

    #[test]
    fn test_dirty_flags_combine() {
        let flags = DirtyFlags::TRANSFORM | DirtyFlags::MATERIAL;
        assert!(flags.contains(DirtyFlags::TRANSFORM));
        assert!(!flags.contains(DirtyFlags::MESH));
        assert!(flags.contains(DirtyFlags::MATERIAL));
    }

    #[test]
    fn test_new_entity_is_dirty() {
        let tesseract = Tesseract4D::new(2.0);
        let entity = Entity::new(ShapeRef::shared(tesseract));

        assert!(entity.is_dirty());
        assert_eq!(entity.dirty_flags(), DirtyFlags::ALL);
    }

    #[test]
    fn test_entity_clear_dirty() {
        let tesseract = Tesseract4D::new(2.0);
        let mut entity = Entity::new(ShapeRef::shared(tesseract));

        assert!(entity.is_dirty());
        entity.clear_dirty();
        assert!(!entity.is_dirty());
        assert_eq!(entity.dirty_flags(), DirtyFlags::NONE);
    }

    #[test]
    fn test_entity_mark_dirty() {
        let tesseract = Tesseract4D::new(2.0);
        let mut entity = Entity::new(ShapeRef::shared(tesseract));
        entity.clear_dirty();

        assert!(!entity.is_dirty());

        entity.mark_dirty(DirtyFlags::TRANSFORM);
        assert!(entity.is_dirty());
        assert!(entity.dirty_flags().contains(DirtyFlags::TRANSFORM));
        assert!(!entity.dirty_flags().contains(DirtyFlags::MESH));
    }

    #[test]
    fn test_set_position_marks_dirty() {
        let tesseract = Tesseract4D::new(2.0);
        let mut entity = Entity::new(ShapeRef::shared(tesseract));
        entity.clear_dirty();

        entity.set_position(Vec4::new(1.0, 2.0, 3.0, 4.0));

        assert!(entity.is_dirty());
        assert!(entity.dirty_flags().contains(DirtyFlags::TRANSFORM));
        assert_eq!(entity.transform.position.x, 1.0);
    }

    #[test]
    fn test_set_transform_marks_dirty() {
        let tesseract = Tesseract4D::new(2.0);
        let mut entity = Entity::new(ShapeRef::shared(tesseract));
        entity.clear_dirty();

        let new_transform = Transform4D::from_position(Vec4::new(5.0, 6.0, 7.0, 8.0));
        entity.set_transform(new_transform);

        assert!(entity.is_dirty());
        assert!(entity.dirty_flags().contains(DirtyFlags::TRANSFORM));
        assert_eq!(entity.transform.position.x, 5.0);
    }

    #[test]
    fn test_set_material_marks_dirty() {
        let tesseract = Tesseract4D::new(2.0);
        let mut entity = Entity::new(ShapeRef::shared(tesseract));
        entity.clear_dirty();

        entity.set_material(Material::RED);

        assert!(entity.is_dirty());
        assert!(entity.dirty_flags().contains(DirtyFlags::MATERIAL));
        assert!(!entity.dirty_flags().contains(DirtyFlags::TRANSFORM));
        assert_eq!(entity.material.base_color, [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_mark_dirty_combines_flags() {
        let tesseract = Tesseract4D::new(2.0);
        let mut entity = Entity::new(ShapeRef::shared(tesseract));
        entity.clear_dirty();

        entity.mark_dirty(DirtyFlags::TRANSFORM);
        entity.mark_dirty(DirtyFlags::MATERIAL);

        // Both flags should be set
        let flags = entity.dirty_flags();
        assert!(flags.contains(DirtyFlags::TRANSFORM));
        assert!(flags.contains(DirtyFlags::MATERIAL));
        assert!(!flags.contains(DirtyFlags::MESH));
    }
}
