use std::{
    ffi::{c_char, CStr, CString},
    ptr,
};

use super::{
    ffi::{OPPW4_PLUGIN_API_STRUCT_SIZE, OPPW4_PLUGIN_API_VERSION},
    structs::Oppw4PluginApi,
};

pub fn cstring_lossy(value: impl AsRef<str>) -> CString {
    let bytes = value
        .as_ref()
        .as_bytes()
        .iter()
        .copied()
        .filter(|byte| *byte != 0)
        .collect::<Vec<_>>();
    CString::new(bytes).unwrap_or_else(|_| CString::new("").expect("empty cstring"))
}

/// # Safety
///
/// When `value` is non-null, it must point to a valid NUL-terminated C string
/// for the returned reference lifetime.
pub unsafe fn optional_cstr<'a>(value: *const c_char) -> Option<&'a CStr> {
    (!value.is_null()).then(|| CStr::from_ptr(value))
}

pub const fn null_api() -> Oppw4PluginApi {
    Oppw4PluginApi {
        version: OPPW4_PLUGIN_API_VERSION,
        struct_size: OPPW4_PLUGIN_API_STRUCT_SIZE,
        host_context: ptr::null_mut(),
        game_root_utf8: ptr::null(),
        plugin_root_utf8: ptr::null(),
        mods_root_utf8: ptr::null(),
        config_root_utf8: ptr::null(),
        log: None,
        module_base: None,
        read_memory: None,
        write_memory: None,
        scan_memory: None,
        for_each_plugin_mod_zip: None,
        for_each_plugin_mod: None,
        register_file_provider: None,
        game_status: None,
        register_registry_module: None,
        active_character: None,
        debug_enabled: None,
        replace_linkdata_entry: None,
        patch_linkdata_row: None,
        require_capability: None,
        register_game_status_provider: None,
        register_active_character_provider: None,
        register_linkdata_provider: None,
        register_rdb_service: None,
        register_rdb_patch_provider: None,
        register_rdb_virtual_provider: None,
        subscribe_signal: None,
        emit_signal: None,
        register_config_schema: None,
        has_signal_listeners: None,
    }
}
