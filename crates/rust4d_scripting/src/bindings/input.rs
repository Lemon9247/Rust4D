//! Input bindings for Lua — live `InputSnapshot` bridge
//!
//! Reads the per-frame [`InputSnapshot`] registered into `app_data` by
//! `ScriptSystem`. When no snapshot is registered (e.g. the binding is called
//! outside a lifecycle dispatch), queries return defaults (false / zero)
//! instead of erroring, so scripts remain robust in tests and headless mode.
//!
//! # Usage (Lua)
//!
//! ```lua
//! if input.is_key_pressed("W") then ... end
//! if input.is_action_just_pressed("jump") then ... end
//! local forward = input.get_axis("W", "S")   -- -1..1
//! local dx, dy = input.mouse_delta()
//! ```

use mlua::prelude::*;

use crate::context::InputSnapshot;

/// Known valid key names for validation / typo warnings.
const VALID_KEY_NAMES: &[&str] = &[
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "Shift",
    "LShift",
    "RShift",
    "Control",
    "LControl",
    "RControl",
    "Ctrl",
    "LCtrl",
    "RCtrl",
    "Alt",
    "LAlt",
    "RAlt",
    "Super",
    "LSuper",
    "RSuper",
    "Win",
    "LWin",
    "RWin",
    "Space",
    "Enter",
    "Return",
    "Escape",
    "Esc",
    "Tab",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Up",
    "Down",
    "Left",
    "Right",
    "Minus",
    "Plus",
    "Equals",
    "LeftBracket",
    "RightBracket",
    "LBracket",
    "RBracket",
    "Backslash",
    "Semicolon",
    "Quote",
    "Apostrophe",
    "Comma",
    "Period",
    "Slash",
    "Grave",
    "Backtick",
    "Tilde",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadAdd",
    "NumpadSubtract",
    "NumpadMultiply",
    "NumpadDivide",
    "NumpadEnter",
    "NumpadDecimal",
    "CapsLock",
    "NumLock",
    "ScrollLock",
    "PrintScreen",
    "Pause",
];

/// Validate a key name, warning once on unknown names (which may be typos).
fn validate_key_name(key: &str) -> LuaResult<()> {
    let key_upper = key.to_uppercase();
    for &valid in VALID_KEY_NAMES {
        if valid.eq_ignore_ascii_case(key) || valid.to_uppercase() == key_upper {
            return Ok(());
        }
    }
    log::warn!(
        "[input] Unknown key name '{}'. Known keys include A-Z, 0-9, F1-F12, Space, \
         Enter, Escape, Shift, Control, Alt, Arrow keys, etc.",
        key
    );
    Ok(())
}

const VALID_MOUSE_BUTTONS: &[&str] = &["left", "right", "middle", "mouse1", "mouse2", "mouse3"];

fn validate_mouse_button(button: &str) -> LuaResult<()> {
    let lower = button.to_lowercase();
    if !VALID_MOUSE_BUTTONS.contains(&lower.as_str()) {
        return Err(LuaError::RuntimeError(format!(
            "Invalid mouse button '{}'. Valid buttons: left, right, middle",
            button
        )));
    }
    Ok(())
}

/// Run `f` against the per-frame `InputSnapshot`, or return a default if none.
fn with_input<F, R>(lua: &Lua, default: R, f: F) -> LuaResult<R>
where
    F: FnOnce(&InputSnapshot) -> R,
{
    match lua.app_data_ref::<InputSnapshot>() {
        Some(snap) => Ok(f(&snap)),
        None => {
            log::trace!("[input] no InputSnapshot registered; returning default");
            Ok(default)
        }
    }
}

/// Register input bindings with the Lua VM.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let input_table = lua.create_table()?;

    input_table.set(
        "is_key_pressed",
        lua.create_function(|lua, key: String| {
            validate_key_name(&key)?;
            with_input(lua, false, |s| s.is_key_pressed(&key))
        })?,
    )?;

    input_table.set(
        "is_key_just_pressed",
        lua.create_function(|lua, key: String| {
            validate_key_name(&key)?;
            with_input(lua, false, |s| s.is_key_just_pressed(&key))
        })?,
    )?;

    input_table.set(
        "is_action_pressed",
        lua.create_function(|lua, action: String| {
            with_input(lua, false, |s| s.is_action_pressed(&action))
        })?,
    )?;

    input_table.set(
        "is_action_just_pressed",
        lua.create_function(|lua, action: String| {
            with_input(lua, false, |s| s.is_action_just_pressed(&action))
        })?,
    )?;

    input_table.set(
        "get_axis",
        lua.create_function(|lua, (positive, negative): (String, String)| {
            validate_key_name(&positive)?;
            validate_key_name(&negative)?;
            with_input(lua, 0.0, |s| s.key_axis(&positive, &negative))
        })?,
    )?;

    input_table.set(
        "mouse_delta",
        lua.create_function(|lua, ()| with_input(lua, (0.0f32, 0.0f32), |s| s.mouse_delta))?,
    )?;

    input_table.set(
        "mouse_position",
        lua.create_function(|lua, ()| with_input(lua, (0.0f32, 0.0f32), |s| s.mouse_position))?,
    )?;

    input_table.set(
        "is_mouse_button_pressed",
        lua.create_function(|lua, button: String| {
            validate_mouse_button(&button)?;
            with_input(lua, false, |s| s.is_mouse_pressed(&button))
        })?,
    )?;

    input_table.set(
        "is_mouse_button_just_pressed",
        lua.create_function(|lua, button: String| {
            validate_mouse_button(&button)?;
            with_input(lua, false, |s| s.is_mouse_just_pressed(&button))
        })?,
    )?;

    lua.globals().set("input", input_table)?;

    log::debug!("[input] Input bindings registered (live InputSnapshot bridge)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::InputSnapshot;

    fn create_lua_with_input() -> Lua {
        let lua = Lua::new();
        register(&lua).expect("Failed to register input bindings");
        lua
    }

    #[test]
    fn test_input_table_exists() {
        let lua = create_lua_with_input();
        let input: LuaTable = lua
            .globals()
            .get("input")
            .expect("input table should exist");
        for key in [
            "is_key_pressed",
            "is_key_just_pressed",
            "is_action_pressed",
            "is_action_just_pressed",
            "get_axis",
            "mouse_delta",
            "mouse_position",
            "is_mouse_button_pressed",
            "is_mouse_button_just_pressed",
        ] {
            assert!(input.contains_key(key).unwrap());
        }
    }

    #[test]
    fn test_defaults_without_snapshot() {
        let lua = create_lua_with_input();
        assert!(!lua
            .load("return input.is_key_pressed('W')")
            .eval::<bool>()
            .unwrap());
        let axis = lua
            .load("return input.get_axis('W','S')")
            .eval::<f32>()
            .unwrap();
        assert!(axis.abs() < 1e-6);
    }

    #[test]
    fn test_reads_from_snapshot() {
        let lua = create_lua_with_input();
        let mut snap = InputSnapshot::new();
        snap.press_key("w");
        snap.set_action("jump", 1.0);
        snap.mark_action_just_pressed("jump");
        snap.add_mouse_motion(3.0, 4.0);
        lua.set_app_data(snap);

        assert!(lua
            .load("return input.is_key_pressed('W')")
            .eval::<bool>()
            .unwrap());
        assert!(lua
            .load("return input.is_key_just_pressed('W')")
            .eval::<bool>()
            .unwrap());
        assert!(lua
            .load("return input.is_action_pressed('jump')")
            .eval::<bool>()
            .unwrap());
        assert!(lua
            .load("return input.is_action_just_pressed('jump')")
            .eval::<bool>()
            .unwrap());
        assert_eq!(
            lua.load("return input.get_axis('W','S')")
                .eval::<f32>()
                .unwrap(),
            1.0
        );
        let (dx, dy) = lua
            .load("return input.mouse_delta()")
            .eval::<(f32, f32)>()
            .unwrap();
        assert_eq!((dx, dy), (3.0, 4.0));
    }

    #[test]
    fn test_invalid_mouse_button_errors() {
        let lua = create_lua_with_input();
        assert!(lua
            .load("return input.is_mouse_button_pressed('nope')")
            .eval::<bool>()
            .is_err());
    }
}
