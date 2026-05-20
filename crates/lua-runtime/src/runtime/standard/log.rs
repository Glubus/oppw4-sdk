use mlua::{Lua, Table};

use crate::runtime::require::register_std_module;

const LOG_BUFFER_GLOBAL: &str = "__oppw4_mod_log_buffer";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaLogEntry {
    pub level: String,
    pub message: String,
    pub mod_id: Option<String>,
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.globals().set(LOG_BUFFER_GLOBAL, lua.create_table()?)?;

    let module = lua.create_table()?;
    module.set("debug", lua.create_function(log_debug)?)?;
    module.set("info", lua.create_function(log_info)?)?;
    module.set("warn", lua.create_function(log_warn)?)?;
    module.set("error", lua.create_function(log_error)?)?;
    module.set("entries", lua.create_function(entries)?)?;
    register_std_module(lua, "log", module)
}

fn log_debug(lua: &Lua, message: String) -> mlua::Result<()> {
    push_entry(lua, "debug", &message)
}

fn log_info(lua: &Lua, message: String) -> mlua::Result<()> {
    push_entry(lua, "info", &message)
}

fn log_warn(lua: &Lua, message: String) -> mlua::Result<()> {
    push_entry(lua, "warn", &message)
}

fn log_error(lua: &Lua, message: String) -> mlua::Result<()> {
    push_entry(lua, "error", &message)
}

fn entries(lua: &Lua, (): ()) -> mlua::Result<Table> {
    lua.globals().get(LOG_BUFFER_GLOBAL)
}

pub(crate) fn collect_entries(lua: &Lua) -> mlua::Result<Vec<LuaLogEntry>> {
    let entries: Table = lua.globals().get(LOG_BUFFER_GLOBAL)?;
    entries
        .sequence_values::<Table>()
        .map(|entry| {
            let entry = entry?;
            Ok(LuaLogEntry {
                level: entry.get("level")?,
                message: entry.get("message")?,
                mod_id: entry.get("mod_id").ok(),
            })
        })
        .collect()
}

fn push_entry(lua: &Lua, level: &str, message: &str) -> mlua::Result<()> {
    let entries: Table = lua.globals().get(LOG_BUFFER_GLOBAL)?;
    let entry = lua.create_table()?;
    entry.set("level", level)?;
    entry.set("message", message)?;
    if let Some(mod_id) = lua.globals().get::<Option<String>>("__oppw4_mod_id")? {
        entry.set("mod_id", mod_id)?;
    }
    entries.set(entries.raw_len() + 1, entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_log_records_mod_scoped_entries() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        lua.globals()
            .set("__oppw4_mod_id", "fx_cycle_test")
            .expect("mod id");

        let value: String = lua
            .load(
                r#"
                local log = require("std.log")
                log.info("hello")
                local entries = log.entries()
                return entries[1].level .. ":" .. entries[1].mod_id .. ":" .. entries[1].message
                "#,
            )
            .eval()
            .expect("log entry");

        assert_eq!(value, "info:fx_cycle_test:hello");

        assert_eq!(
            collect_entries(&lua).expect("entries"),
            vec![LuaLogEntry {
                level: "info".to_string(),
                message: "hello".to_string(),
                mod_id: Some("fx_cycle_test".to_string()),
            }]
        );
    }
}
