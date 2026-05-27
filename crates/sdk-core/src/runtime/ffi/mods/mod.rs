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

fn legacy_mod_paths(mods_root: &Path) -> Vec<CString> {
    mods::list_legacy_paths(mods_root)
        .into_iter()
        .map(|path| cstring_lossy(&path.to_string_lossy()))
        .collect()
}

fn plugin_mods_for_context(context: &ApiContext) -> Vec<PreparedPluginMod> {
    let _ = context;
    Vec::new()
}

fn plugin_mod_entry(prepared: &PreparedPluginMod) -> Oppw4PluginModEntry {
    plugin_mod_entry_from_parts(&prepared.strings, prepared.is_zip)
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
