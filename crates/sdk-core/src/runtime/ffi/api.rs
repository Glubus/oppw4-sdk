use std::{ffi::CString, path::Path};

use plugin_abi::{Oppw4PluginApi, OPPW4_PLUGIN_API_VERSION};

use super::{
    context::ApiContext,
    linkdata::{host_patch_linkdata_row, host_replace_linkdata_entry},
    log::host_log,
    lua::host_register_lua_module,
    memory::{host_module_base, host_read_memory, host_scan_memory, host_write_memory},
    mods::{host_for_each_plugin_mod, host_for_each_plugin_mod_zip},
    providers::host_register_file_provider,
    status::{host_active_character, host_debug_enabled, host_game_status},
};

pub(crate) fn build_api(
    game_root: &Path,
    game_root_utf8: &CString,
    plugin_root_utf8: &CString,
    plugin_mods_root_utf8: &CString,
    context: &ApiContext,
) -> Oppw4PluginApi {
    let _ = game_root;
    Oppw4PluginApi {
        version: OPPW4_PLUGIN_API_VERSION,
        host_context: (context as *const ApiContext).cast_mut().cast(),
        game_root_utf8: game_root_utf8.as_ptr(),
        plugin_root_utf8: plugin_root_utf8.as_ptr(),
        plugin_mods_root_utf8: plugin_mods_root_utf8.as_ptr(),
        log: Some(host_log),
        module_base: Some(host_module_base),
        read_memory: Some(host_read_memory),
        write_memory: Some(host_write_memory),
        scan_memory: Some(host_scan_memory),
        for_each_plugin_mod_zip: Some(host_for_each_plugin_mod_zip),
        for_each_plugin_mod: Some(host_for_each_plugin_mod),
        register_file_provider: Some(host_register_file_provider),
        game_status: Some(host_game_status),
        register_lua_module: Some(host_register_lua_module),
        active_character: Some(host_active_character),
        debug_enabled: Some(host_debug_enabled),
        replace_linkdata_entry: Some(host_replace_linkdata_entry),
        patch_linkdata_row: Some(host_patch_linkdata_row),
    }
}
