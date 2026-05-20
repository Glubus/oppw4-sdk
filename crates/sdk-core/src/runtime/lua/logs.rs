use crate::log;
use crate::runtime::logs as host_logs;

pub(super) fn write_mod_entries(mod_entry: &lua_api::LuaMod, entries: &[lua_api::LuaLogEntry]) {
    for entry in entries {
        let message = format_entry(mod_entry, entry);
        host_logs::write_mod(mod_id(mod_entry, entry), &message);
        if plugin_sdk::log::mirror_mod_log_to_host(&entry.level) {
            log::write_line(message);
        }
    }
}

fn format_entry(mod_entry: &lua_api::LuaMod, entry: &lua_api::LuaLogEntry) -> String {
    let mod_id = mod_id(mod_entry, entry);
    format!(
        "lua mod log id={mod_id} level={} message={}",
        entry.level, entry.message
    )
}

fn mod_id<'a>(mod_entry: &'a lua_api::LuaMod, entry: &'a lua_api::LuaLogEntry) -> &'a str {
    entry.mod_id.as_deref().unwrap_or(&mod_entry.manifest.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_log_entry_with_mod_id_fallback() {
        let mod_entry = lua_api::LuaMod {
            manifest: lua_api::LuaModManifest {
                id: "fallback_mod".to_string(),
                name: "Fallback Mod".to_string(),
                uses_plugins: Vec::new(),
                entry_lua: "mod.lua".to_string(),
            },
            source: lua_api::ModSource::Directory("mods/fallback_mod".into()),
        };
        let entry = lua_api::LuaLogEntry {
            level: "info".to_string(),
            message: "loaded".to_string(),
            mod_id: None,
        };

        assert_eq!(
            format_entry(&mod_entry, &entry),
            "lua mod log id=fallback_mod level=info message=loaded"
        );
    }

    #[test]
    fn formats_log_entry_with_explicit_mod_id() {
        let mod_entry = lua_api::LuaMod {
            manifest: lua_api::LuaModManifest {
                id: "fallback_mod".to_string(),
                name: "Fallback Mod".to_string(),
                uses_plugins: Vec::new(),
                entry_lua: "mod.lua".to_string(),
            },
            source: lua_api::ModSource::Directory("mods/fallback_mod".into()),
        };
        let entry = lua_api::LuaLogEntry {
            level: "warn".to_string(),
            message: "override".to_string(),
            mod_id: Some("real_mod".to_string()),
        };

        assert_eq!(
            format_entry(&mod_entry, &entry),
            "lua mod log id=real_mod level=warn message=override"
        );
    }
}
