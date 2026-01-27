//! 4D Physics simulation for Rust4D
//!
//! This crate provides physics simulation for 4D rigid bodies, including:
//! - Collision shapes (spheres, AABBs, planes)
//! - Collision detection
//! - Rigid body dynamics with gravity
//! - Player physics for FPS-style movement

pub mod collision;
pub mod shapes;

// Re-export commonly used types
pub use collision::{aabb_vs_aabb, aabb_vs_plane, sphere_vs_aabb, sphere_vs_plane, Contact};
pub use shapes::{Collider, Plane4D, Sphere4D, AABB4D};
