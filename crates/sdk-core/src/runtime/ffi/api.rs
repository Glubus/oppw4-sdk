use std::{ffi::CString, path::Path};

use plugin_abi::{Oppw4PluginApi, OPPW4_PLUGIN_API_VERSION};

use super::{
    context::ApiContext,
    linkdata::{
        host_patch_linkdata_row, host_register_linkdata_provider, host_replace_linkdata_entry,
    },
    log::host_log,
    lua::host_register_lua_module,
    memory::{host_module_base, host_read_memory, host_scan_memory, host_write_memory},
    mods::{host_for_each_plugin_mod, host_for_each_plugin_mod_zip},
    providers::host_register_file_provider,
    rdb::{host_register_rdb_patch_provider, host_register_rdb_service},
    status::{
        host_active_character, host_debug_enabled, host_game_status,
        host_register_active_character_provider, host_register_game_status_provider,
    },
};

use plugin_abi::optional_cstr;

pub(crate) fn build_api(
    game_root: &Path,
    game_root_utf8: &CString,
    plugin_root_utf8: &CString,
    mods_root_utf8: &CString,
    context: &ApiContext,
) -> Oppw4PluginApi {
    let _ = game_root;
    Oppw4PluginApi {
        version: OPPW4_PLUGIN_API_VERSION,
        host_context: (context as *const ApiContext).cast_mut().cast(),
        game_root_utf8: game_root_utf8.as_ptr(),
        plugin_root_utf8: plugin_root_utf8.as_ptr(),
        mods_root_utf8: mods_root_utf8.as_ptr(),
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
        require_capability: Some(host_require_capability),
        register_game_status_provider: Some(host_register_game_status_provider),
        register_active_character_provider: Some(host_register_active_character_provider),
        register_linkdata_provider: Some(host_register_linkdata_provider),
        register_rdb_service: Some(host_register_rdb_service),
        register_rdb_patch_provider: Some(host_register_rdb_patch_provider),
    }
}

unsafe extern "system" fn host_require_capability(
    host_context: *mut std::ffi::c_void,
    plugin_id: *const std::ffi::c_char,
    capability: *const std::ffi::c_char,
) -> i32 {
    let context = match super::context::context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    let Some(capability) = optional_cstr(capability) else {
        return -25;
    };
    match context
        .require_capability_for_cstr(optional_cstr(plugin_id), &capability.to_string_lossy())
    {
        Ok(()) => 0,
        Err(code) => code,
    }
}
