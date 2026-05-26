use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use mlua::{HookTriggers, Lua, VmState};

use crate::{mod_files, LuaMod, ModSource};

pub use crate::std_plugins::LuaLogEntry;

#[derive(Debug)]
pub enum LuaRunError {
    ReadScript(std::io::Error),
    Lua(mlua::Error),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LuaRunReport {
    pub logs: Vec<LuaLogEntry>,
    pub mutations: Vec<LuaMutation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LuaMutation {
    pub kind: String,
    pub mod_id: String,
    pub character: Option<String>,
    pub entry: Option<u16>,
    pub payload_file: Option<String>,
    pub payload: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct LuaBatchRunReport {
    pub mod_id: String,
    pub result: Result<LuaRunReport, LuaRunError>,
}

const LUA_INSTRUCTION_POLL_INTERVAL: u32 = 10_000;
const LUA_LOAD_INSTRUCTION_BUDGET: usize = 5_000_000;

pub fn run_lua_mod<F>(mod_entry: &LuaMod, register_modules: F) -> Result<LuaRunReport, LuaRunError>
where
    F: FnOnce(&Lua) -> mlua::Result<()>,
{
    let lua = super::sandbox::new_lua().map_err(LuaRunError::Lua)?;
    super::install_runtime(&lua).map_err(LuaRunError::Lua)?;
    install_mod_globals(&lua, mod_entry).map_err(LuaRunError::Lua)?;
    set_current_mod_file_context(mod_entry);
    register_modules(&lua).map_err(LuaRunError::Lua)?;
    super::sandbox::hide_unsafe_globals(&lua).map_err(LuaRunError::Lua)?;
    let source = mod_entry
        .read_entry_script()
        .map_err(LuaRunError::ReadScript)?;
    install_instruction_budget(&lua, LUA_LOAD_INSTRUCTION_BUDGET, &mod_entry.manifest.id);
    let exec_result = lua
        .load(&source)
        .set_name(format!(
            "{}:{}",
            mod_entry.manifest.id, mod_entry.manifest.entry_lua
        ))
        .exec();
    lua.remove_hook();
    let logs = crate::std_plugins::collect_log_entries(&lua).map_err(LuaRunError::Lua)?;
    let mutations = collect_pending_mutations(&lua).map_err(LuaRunError::Lua)?;
    mod_files::set_current_mod_file_context(None);
    exec_result
        .map(|()| LuaRunReport { logs, mutations })
        .map_err(LuaRunError::Lua)
}

pub fn run_lua_mods<F, B>(
    mod_entries: &[LuaMod],
    register_modules: F,
    mut before_mod: B,
) -> Result<Vec<LuaBatchRunReport>, LuaRunError>
where
    F: FnOnce(&Lua) -> mlua::Result<()>,
    B: FnMut(&Lua, &LuaMod) -> mlua::Result<()>,
{
    let lua = super::sandbox::new_lua().map_err(LuaRunError::Lua)?;
    super::install_runtime(&lua).map_err(LuaRunError::Lua)?;
    register_modules(&lua).map_err(LuaRunError::Lua)?;
    super::sandbox::hide_unsafe_globals(&lua).map_err(LuaRunError::Lua)?;

    let mut reports = Vec::with_capacity(mod_entries.len());
    for mod_entry in mod_entries {
        let result = run_lua_mod_in_existing_vm(&lua, mod_entry, &mut before_mod);
        reports.push(LuaBatchRunReport {
            mod_id: mod_entry.manifest.id.clone(),
            result,
        });
    }
    Ok(reports)
}

fn run_lua_mod_in_existing_vm<B>(
    lua: &Lua,
    mod_entry: &LuaMod,
    before_mod: &mut B,
) -> Result<LuaRunReport, LuaRunError>
where
    B: FnMut(&Lua, &LuaMod) -> mlua::Result<()>,
{
    install_mod_globals(lua, mod_entry).map_err(LuaRunError::Lua)?;
    set_current_mod_file_context(mod_entry);
    before_mod(lua, mod_entry).map_err(LuaRunError::Lua)?;
    let source = mod_entry
        .read_entry_script()
        .map_err(LuaRunError::ReadScript)?;
    install_instruction_budget(lua, LUA_LOAD_INSTRUCTION_BUDGET, &mod_entry.manifest.id);
    let exec_result = lua
        .load(&source)
        .set_name(format!(
            "{}:{}",
            mod_entry.manifest.id, mod_entry.manifest.entry_lua
        ))
        .exec();
    lua.remove_hook();
    let logs = crate::std_plugins::collect_log_entries(lua).map_err(LuaRunError::Lua)?;
    crate::std_plugins::clear_log_entries(lua).map_err(LuaRunError::Lua)?;
    let mutations = collect_pending_mutations(lua).map_err(LuaRunError::Lua)?;
    clear_pending_mutations(lua).map_err(LuaRunError::Lua)?;
    mod_files::set_current_mod_file_context(None);
    exec_result
        .map(|()| LuaRunReport { logs, mutations })
        .map_err(LuaRunError::Lua)
}

fn collect_pending_mutations(lua: &Lua) -> mlua::Result<Vec<LuaMutation>> {
    let Some(queue) = lua
        .globals()
        .get::<Option<mlua::Table>>("__oppw4_pending_mutations")?
    else {
        return Ok(Vec::new());
    };
    let mut mutations = Vec::new();
    for mutation in queue.sequence_values::<mlua::Table>() {
        let mutation = mutation?;
        let payload = mutation
            .get::<Option<mlua::String>>("payload")?
            .map(|payload| payload.as_bytes().to_vec());
        mutations.push(LuaMutation {
            kind: mutation.get::<String>("type")?,
            mod_id: mutation.get::<String>("mod_id")?,
            character: mutation.get::<Option<String>>("character")?,
            entry: mutation.get::<Option<u16>>("entry")?,
            payload_file: mutation.get::<Option<String>>("payload_file")?,
            payload,
        });
    }
    Ok(mutations)
}

fn clear_pending_mutations(lua: &Lua) -> mlua::Result<()> {
    lua.globals()
        .set("__oppw4_pending_mutations", lua.create_table()?)
}

fn install_instruction_budget(lua: &Lua, max_instructions: usize, mod_id: &str) {
    let counter = Arc::new(AtomicUsize::new(0));
    let mod_id = mod_id.to_string();
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(LUA_INSTRUCTION_POLL_INTERVAL),
        move |_lua, _debug| {
            let used = counter.fetch_add(
                LUA_INSTRUCTION_POLL_INTERVAL as usize,
                Ordering::Relaxed,
            ) + LUA_INSTRUCTION_POLL_INTERVAL as usize;
            if used > max_instructions {
                return Err(mlua::Error::RuntimeError(format!(
                    "lua instruction budget exceeded mod={mod_id} used={used} budget={max_instructions}"
                )));
            }
            Ok(VmState::Continue)
        },
    );
}

fn set_current_mod_file_context(mod_entry: &LuaMod) {
    mod_files::set_current_mod_file_context(Some(mod_files::CurrentModFileContext {
        root: match &mod_entry.source {
            ModSource::Directory(root) => root.clone(),
            ModSource::Zip { path, .. } => path.clone(),
        },
        zip_root: match &mod_entry.source {
            ModSource::Directory(_) => String::new(),
            ModSource::Zip { root, .. } => root.clone(),
        },
        is_zip: matches!(mod_entry.source, ModSource::Zip { .. }),
        files: Default::default(),
    }));
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

    #[test]
    fn run_lua_mod_stops_infinite_loop_with_instruction_budget() {
        let root = temp_root("lua-runner-budget");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(root.join("mod.lua"), "while true do end").expect("script");
        let mod_entry = LuaMod {
            manifest: crate::LuaModManifest {
                id: "budget_test".to_string(),
                name: "Budget Test".to_string(),
                uses_plugins: Vec::new(),
                entry_lua: "mod.lua".to_string(),
            },
            source: ModSource::Directory(root.clone()),
        };

        let error = run_lua_mod(&mod_entry, |_| Ok(())).expect_err("budget should stop loop");
        let message = format!("{error:?}");

        assert!(message.contains("instruction budget exceeded"));
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
