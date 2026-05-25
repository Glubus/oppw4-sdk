use mlua::Lua;

use crate::{log, runtime::ffi};

use super::logs::write_mod_entries;
use super::module::{register_plugin_module, RegisteredModule};

#[derive(Clone, Copy)]
pub(super) enum ModRunReason {
    Initial,
    HotReload,
}

impl ModRunReason {
    fn success_label(self) -> &'static str {
        match self {
            Self::Initial => "applied",
            Self::HotReload => "hot-reloaded",
        }
    }

    fn failure_label(self) -> &'static str {
        match self {
            Self::Initial => "pending/failed",
            Self::HotReload => "hot-reload failed",
        }
    }
}

pub(super) fn run_mod(
    mod_entry: &lua_api::LuaMod,
    modules: Vec<RegisteredModule>,
    reason: ModRunReason,
) -> bool {
    let result = lua_api::run_lua_mod(mod_entry, |lua| {
        install_player_snapshot(lua)?;
        for module in modules {
            register_plugin_module(lua, &module)?;
        }
        Ok(())
    });
    match result {
        Ok(report) => {
            write_mod_entries(mod_entry, &report.logs);
            log::write_line(format!(
                "lua host: mod {} id={} uses={:?}",
                reason.success_label(),
                mod_entry.manifest.id,
                mod_entry.manifest.uses_plugins
            ));
            true
        }
        Err(error) => {
            log::write_line(format!(
                "lua host: mod {} id={} error={error:?}",
                reason.failure_label(),
                mod_entry.manifest.id
            ));
            false
        }
    }
}

fn install_player_snapshot(lua: &Lua) -> mlua::Result<()> {
    let rows = lua.create_table()?;
    if let Some(active) = ffi::active_character_snapshot().filter(|active| active.sequence != 0) {
        let row = lua.create_table()?;
        row.set("runtime_id", active.runtime_id)?;
        row.set("alt_id", active.alt_id)?;
        row.set("local_player", active.local_player as u64)?;
        row.set("fx_owner", active.fx_owner as u64)?;
        row.set("sequence", active.sequence)?;
        rows.set(1, row)?;
    }
    lua.globals().set("__oppw4_active_characters", rows)
}
