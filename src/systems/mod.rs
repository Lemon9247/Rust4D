//! Application systems
//!
//! Modular systems extracted from main.rs for better organization and testability.

mod render;
mod simulation;
mod window;

pub use render::{RenderError, RenderSystem};
pub use simulation::SimulationSystem;
pub use window::WindowSystem;
