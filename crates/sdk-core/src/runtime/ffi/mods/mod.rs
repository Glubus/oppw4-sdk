mod r#unsafe;

use std::{ffi::CString, path::Path};

use plugin_abi::{Oppw4PluginModEntry, OPPW4_PLUGIN_MOD_FLAG_ZIP};

use crate::runtime::mods;

use super::{context::ApiContext, strings::cstring_lossy};

pub(crate) use r#unsafe::{host_for_each_plugin_mod, host_for_each_plugin_mod_zip};

struct ModEntryCStrings {
    id: CString,
    name: CString,
    source_path: CString,
    entry_lua: CString,
}

struct PreparedPluginMod {
    strings: ModEntryCStrings,
    is_zip: bool,
}

fn invalid_visitor_error<T>(host_context: *mut std::ffi::c_void, visitor: Option<T>) -> i32 {
    if host_context.is_null() {
        -1
    } else if visitor.is_none() {
        -2
    } else {
        -1
    }
}

fn is_mod_for_plugin(mod_entry: &lua_api::LuaMod, context: &ApiContext) -> bool {
    mod_entry.uses_plugin(&context.plugin_id)
}

fn legacy_mod_paths(plugin_mods_root: &Path) -> Vec<CString> {
    mods::list_legacy_paths(plugin_mods_root)
        .into_iter()
        .map(|path| cstring_lossy(&path.to_string_lossy()))
        .collect()
}

fn plugin_mods_for_context(context: &ApiContext) -> Vec<PreparedPluginMod> {
    lua_api::discover_mods(&context.plugin_mods_root)
        .into_iter()
        .filter(|mod_entry| is_mod_for_plugin(mod_entry, context))
        .map(|mod_entry| PreparedPluginMod {
            strings: mod_entry_cstrings(&mod_entry),
            is_zip: mod_entry.is_zip(),
        })
        .collect()
}

fn plugin_mod_entry(prepared: &PreparedPluginMod) -> Oppw4PluginModEntry {
    plugin_mod_entry_from_parts(&prepared.strings, prepared.is_zip)
}

fn mod_entry_cstrings(mod_entry: &lua_api::LuaMod) -> ModEntryCStrings {
    ModEntryCStrings {
        id: cstring_lossy(&mod_entry.manifest.id),
        name: cstring_lossy(&mod_entry.manifest.name),
        source_path: cstring_lossy(&mod_entry.source_path().to_string_lossy()),
        entry_lua: cstring_lossy(&mod_entry.manifest.entry_lua),
    }
}

fn plugin_mod_entry_from_parts(strings: &ModEntryCStrings, is_zip: bool) -> Oppw4PluginModEntry {
    Oppw4PluginModEntry {
        id: strings.id.as_ptr(),
        name: strings.name.as_ptr(),
        source_path_utf8: strings.source_path.as_ptr(),
        entry_lua_utf8: strings.entry_lua.as_ptr(),
        flags: plugin_mod_flags(is_zip),
    }
}

fn plugin_mod_flags(is_zip: bool) -> u32 {
    if is_zip {
        OPPW4_PLUGIN_MOD_FLAG_ZIP
    } else {
        0
    }
}
