//! Application systems
//!
//! Modular systems extracted from main.rs for better organization and testability.

mod simulation;
mod window;

pub use simulation::SimulationSystem;
pub use window::WindowSystem;
