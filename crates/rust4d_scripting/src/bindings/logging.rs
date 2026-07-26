//! Logging bindings for Lua
//!
//! Provides a global `log` table routing Lua messages to the engine's `log`
//! facade so scripted games share the same log stream as native code.
//!
//! # Usage (Lua)
//!
//! ```lua
//! log.info("on_init fired")
//! log.warn("something unusual")
//! log.error("something broke")
//! log.debug("verbose detail")
//! log.trace("very verbose detail")
//! ```

use mlua::prelude::*;

/// Register the global `log` table with the Lua VM.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let log_table = lua.create_table()?;

    log_table.set(
        "info",
        lua.create_function(|_, msg: String| {
            log::info!("[lua] {}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "warn",
        lua.create_function(|_, msg: String| {
            log::warn!("[lua] {}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "error",
        lua.create_function(|_, msg: String| {
            log::error!("[lua] {}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "debug",
        lua.create_function(|_, msg: String| {
            log::debug!("[lua] {}", msg);
            Ok(())
        })?,
    )?;
    log_table.set(
        "trace",
        lua.create_function(|_, msg: String| {
            log::trace!("[lua] {}", msg);
            Ok(())
        })?,
    )?;

    lua.globals().set("log", log_table)?;

    log::debug!("[log] Logging bindings registered");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_lua_with_log() -> Lua {
        let lua = Lua::new();
        register(&lua).expect("Failed to register log bindings");
        lua
    }

    #[test]
    fn test_log_table_exists() {
        let lua = create_lua_with_log();
        let log: LuaTable = lua.globals().get("log").expect("log table should exist");
        assert!(log.contains_key("info").unwrap());
        assert!(log.contains_key("warn").unwrap());
        assert!(log.contains_key("error").unwrap());
        assert!(log.contains_key("debug").unwrap());
        assert!(log.contains_key("trace").unwrap());
    }

    #[test]
    fn test_log_info_does_not_panic() {
        let lua = create_lua_with_log();
        lua.load(r#"log.info("hello from lua")"#)
            .exec()
            .expect("log.info should be callable");
    }

    #[test]
    fn test_log_error_does_not_panic() {
        let lua = create_lua_with_log();
        lua.load(r#"log.error("error from lua")"#)
            .exec()
            .expect("log.error should be callable");
    }
}
