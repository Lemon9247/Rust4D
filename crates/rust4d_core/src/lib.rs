//! Core types for the Rust4D engine
//!
//! This crate provides the foundational types for building 4D worlds:
//!
//! - [`Transform4D`] - Position, rotation, and scale in 4D space
//! - [`Material`] - Visual properties of an entity
//! - [`Entity`] - An object in the world with transform, shape, and material
//! - [`ShapeRef`] - Reference to a shape (shared or owned)
//! - [`World`] - Container for all entities
//! - [`EntityKey`] - Generational key to an entity in the world
//! - [`ShapeTemplate`] - Serializable shape template
//! - [`EntityTemplate`] - Serializable entity template
//! - [`Scene`] - Loadable/saveable scene containing entities

mod transform;
mod entity;
mod world;
mod shapes;
mod scene;

pub use transform::Transform4D;
pub use entity::{Material, Entity, ShapeRef, DirtyFlags, EntityTemplate};
pub use world::{World, EntityKey};
pub use shapes::ShapeTemplate;
pub use scene::{Scene, SceneLoadError, SceneSaveError};

// Re-export commonly used types from rust4d_math for convenience
pub use rust4d_math::{Vec4, Rotor4, RotationPlane, ConvexShape4D, Tetrahedron};
pub use rust4d_math::{Tesseract4D, Hyperplane4D};

// Re-export physics types for convenient access through rust4d_core
pub use rust4d_physics::{BodyKey, PhysicsConfig, PhysicsWorld, RigidBody4D, StaticCollider};
