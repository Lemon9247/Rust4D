//! 4D Mathematics Library
//!
//! This crate provides 4D vector and rotation types for the Rust4D engine.

mod vec4;
mod rotor4;
pub mod mat4;

pub use vec4::Vec4;
pub use rotor4::{Rotor4, RotationPlane};
pub use mat4::Mat4;
