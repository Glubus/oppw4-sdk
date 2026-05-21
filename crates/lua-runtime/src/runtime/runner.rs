use mlua::Lua;

use crate::{LuaMod, ModSource};

pub use crate::std_plugins::LuaLogEntry;

#[derive(Debug)]
pub enum LuaRunError {
    ReadScript(std::io::Error),
    Lua(mlua::Error),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LuaRunReport {
    pub logs: Vec<LuaLogEntry>,
}

pub fn run_lua_mod<F>(mod_entry: &LuaMod, register_modules: F) -> Result<LuaRunReport, LuaRunError>
where
    F: FnOnce(&Lua) -> mlua::Result<()>,
{
    let lua = super::sandbox::new_lua().map_err(LuaRunError::Lua)?;
    super::install_runtime(&lua).map_err(LuaRunError::Lua)?;
    install_mod_globals(&lua, mod_entry).map_err(LuaRunError::Lua)?;
    register_modules(&lua).map_err(LuaRunError::Lua)?;
    super::sandbox::hide_unsafe_globals(&lua).map_err(LuaRunError::Lua)?;
    let source = mod_entry
        .read_entry_script()
        .map_err(LuaRunError::ReadScript)?;
    let exec_result = lua
        .load(&source)
        .set_name(format!(
            "{}:{}",
            mod_entry.manifest.id, mod_entry.manifest.entry_lua
        ))
        .exec();
    let logs = crate::std_plugins::collect_log_entries(&lua).map_err(LuaRunError::Lua)?;
    exec_result
        .map(|()| LuaRunReport { logs })
        .map_err(LuaRunError::Lua)
}

fn install_mod_globals(lua: &Lua, mod_entry: &LuaMod) -> mlua::Result<()> {
    let globals = lua.globals();
    globals.set("__oppw4_mod_id", mod_entry.manifest.id.as_str())?;
    globals.set("__oppw4_mod_name", mod_entry.manifest.name.as_str())?;
    globals.set(
        "__oppw4_mod_root",
        match &mod_entry.source {
            ModSource::Directory(root) => root.to_string_lossy().to_string(),
            ModSource::Zip { path, .. } => path.to_string_lossy().to_string(),
        },
    )?;
    globals.set(
        "__oppw4_mod_zip_root",
        match &mod_entry.source {
            ModSource::Directory(_) => String::new(),
            ModSource::Zip { root, .. } => root.clone(),
        },
    )?;
    globals.set(
        "__oppw4_mod_is_zip",
        matches!(mod_entry.source, ModSource::Zip { .. }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn run_lua_mod_hides_unsafe_globals_from_script() {
        let root = temp_root("lua-runner-sandbox");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(
            root.join("mod.lua"),
            r#"
            local log = require("std.log")
            log.info(tostring(os) .. ":" .. tostring(io) .. ":" .. tostring(debug) .. ":" .. tostring(package))
            "#,
        )
        .expect("script");
        let mod_entry = LuaMod {
            manifest: crate::LuaModManifest {
                id: "sandbox_test".to_string(),
                name: "Sandbox Test".to_string(),
                uses_plugins: Vec::new(),
                entry_lua: "mod.lua".to_string(),
            },
            source: ModSource::Directory(root.clone()),
        };

        let report = run_lua_mod(&mod_entry, |_| Ok(())).expect("run mod");

        assert_eq!(report.logs.len(), 1);
        assert_eq!(report.logs[0].message, "nil:nil:nil:nil");
        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-{label}-{nanos}"))
    }
}
