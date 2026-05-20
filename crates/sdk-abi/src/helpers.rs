use std::{
    ffi::{c_char, CStr, CString},
    ptr,
};

use super::{ffi::OPPW4_PLUGIN_API_VERSION, structs::Oppw4PluginApi};

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
        host_context: ptr::null_mut(),
        game_root_utf8: ptr::null(),
        plugin_root_utf8: ptr::null(),
        plugin_mods_root_utf8: ptr::null(),
        log: None,
        module_base: None,
        read_memory: None,
        write_memory: None,
        scan_memory: None,
        for_each_plugin_mod_zip: None,
        for_each_plugin_mod: None,
        register_file_provider: None,
        game_status: None,
        register_lua_module: None,
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
    }
}
