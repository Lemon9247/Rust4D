//! Key / mouse button name normalization for the Lua input snapshot
//!
//! Maps winit `KeyCode` / `MouseButton` to the canonical string names expected
//! by the scripting crate's input bindings (see
//! `rust4d_scripting::bindings::input::VALID_KEY_NAMES`).

use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use rust4d_input::CameraAction;

/// Convert a `KeyCode` to its canonical binding name, or `None` if unsupported.
///
/// Names match the set documented in the input bindings (letters upper-cased,
/// common special keys, F-keys, numpad, etc.).
pub fn keycode_to_name(key: KeyCode) -> Option<&'static str> {
    Some(match key {
        // Letters
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        // Numbers
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        // Function keys
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::F5 => "F5",
        KeyCode::F6 => "F6",
        KeyCode::F7 => "F7",
        KeyCode::F8 => "F8",
        KeyCode::F9 => "F9",
        KeyCode::F10 => "F10",
        KeyCode::F11 => "F11",
        KeyCode::F12 => "F12",
        // Modifiers
        KeyCode::ShiftLeft => "LShift",
        KeyCode::ShiftRight => "RShift",
        KeyCode::ControlLeft => "LCtrl",
        KeyCode::ControlRight => "RCtrl",
        KeyCode::AltLeft => "LAlt",
        KeyCode::AltRight => "RAlt",
        KeyCode::SuperLeft => "LSuper",
        KeyCode::SuperRight => "RSuper",
        // Special keys
        KeyCode::Space => "Space",
        KeyCode::Enter => "Enter",
        KeyCode::Escape => "Escape",
        KeyCode::Tab => "Tab",
        KeyCode::Backspace => "Backspace",
        KeyCode::Delete => "Delete",
        KeyCode::Insert => "Insert",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::PageUp => "PageUp",
        KeyCode::PageDown => "PageDown",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        // Punctuation
        KeyCode::Minus => "Minus",
        KeyCode::Equal => "Equals",
        KeyCode::BracketLeft => "LBracket",
        KeyCode::BracketRight => "RBracket",
        KeyCode::Backslash => "Backslash",
        KeyCode::Semicolon => "Semicolon",
        KeyCode::Quote => "Quote",
        KeyCode::Comma => "Comma",
        KeyCode::Period => "Period",
        KeyCode::Slash => "Slash",
        KeyCode::Backquote => "Grave",
        // Numpad
        KeyCode::Numpad0 => "Numpad0",
        KeyCode::Numpad1 => "Numpad1",
        KeyCode::Numpad2 => "Numpad2",
        KeyCode::Numpad3 => "Numpad3",
        KeyCode::Numpad4 => "Numpad4",
        KeyCode::Numpad5 => "Numpad5",
        KeyCode::Numpad6 => "Numpad6",
        KeyCode::Numpad7 => "Numpad7",
        KeyCode::Numpad8 => "Numpad8",
        KeyCode::Numpad9 => "Numpad9",
        KeyCode::NumpadAdd => "NumpadAdd",
        KeyCode::NumpadSubtract => "NumpadSubtract",
        KeyCode::NumpadMultiply => "NumpadMultiply",
        KeyCode::NumpadDivide => "NumpadDivide",
        KeyCode::NumpadEnter => "NumpadEnter",
        KeyCode::NumpadDecimal => "NumpadDecimal",
        // Lock keys
        KeyCode::CapsLock => "CapsLock",
        KeyCode::NumLock => "NumLock",
        KeyCode::ScrollLock => "ScrollLock",
        KeyCode::PrintScreen => "PrintScreen",
        KeyCode::Pause => "Pause",
        _ => return None,
    })
}

/// Convert a `MouseButton` to its canonical name.
pub fn mouse_button_to_name(button: MouseButton) -> Option<&'static str> {
    Some(match button {
        MouseButton::Left => "left",
        MouseButton::Right => "right",
        MouseButton::Middle => "middle",
        MouseButton::Back => "mouse3",
        MouseButton::Forward => "mouse2",
        MouseButton::Other(_) => return None,
    })
}

/// Canonical action name for a `CameraAction` (for the `actions` map in the
/// `InputSnapshot`).
pub fn camera_action_name(action: CameraAction) -> &'static str {
    match action {
        CameraAction::MoveForward => "move_forward",
        CameraAction::MoveBackward => "move_backward",
        CameraAction::MoveLeft => "move_left",
        CameraAction::MoveRight => "move_right",
        CameraAction::MoveUp => "move_up",
        CameraAction::MoveDown => "move_down",
        CameraAction::MoveAna => "move_ana",
        CameraAction::MoveKata => "move_kata",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_letter_keys_uppercased() {
        assert_eq!(keycode_to_name(KeyCode::KeyW), Some("W"));
        assert_eq!(keycode_to_name(KeyCode::KeyA), Some("A"));
    }

    #[test]
    fn test_special_keys() {
        assert_eq!(keycode_to_name(KeyCode::Space), Some("Space"));
        assert_eq!(keycode_to_name(KeyCode::Escape), Some("Escape"));
        assert_eq!(keycode_to_name(KeyCode::F11), Some("F11"));
        assert_eq!(keycode_to_name(KeyCode::ArrowUp), Some("Up"));
    }

    #[test]
    fn test_mouse_buttons() {
        assert_eq!(mouse_button_to_name(MouseButton::Left), Some("left"));
        assert_eq!(mouse_button_to_name(MouseButton::Right), Some("right"));
    }

    #[test]
    fn test_camera_action_names() {
        assert_eq!(
            camera_action_name(CameraAction::MoveForward),
            "move_forward"
        );
        assert_eq!(camera_action_name(CameraAction::MoveAna), "move_ana");
    }

    #[test]
    fn test_unmapped_key_returns_none() {
        // Pick a key not in the table; most keys are mapped, so use a rare one.
        assert!(keycode_to_name(KeyCode::NumpadEqual).is_none());
    }
}
