//! Application systems
//!
//! Modular systems extracted from main.rs for better organization and testability.

mod geometry;
mod render;
mod script;
mod simulation;
mod window;

pub use geometry::build_geometry;
pub use render::{RenderError, RenderSystem};
pub use script::{ScriptSystem, ScriptUpdateResult};
pub use simulation::SimulationSystem;
pub use window::WindowSystem;
