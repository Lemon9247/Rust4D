//! Input handling module
//!
//! Provides input mapping from raw events to semantic actions.

mod input_mapper;
mod key_names;

pub use input_mapper::{InputAction, InputMapper};
pub use key_names::{camera_action_name, keycode_to_name, mouse_button_to_name};
