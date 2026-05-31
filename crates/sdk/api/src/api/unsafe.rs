use std::ffi::{c_char, c_void, CStr, CString};

use crate::{PluginError, PluginModInfo, PluginResult};
use plugin_abi::{
    optional_cstr, HostActiveCharacterFn, HostDebugEnabledFn, HostEmitSignalFn,
    HostForEachPluginModFn, HostForEachPluginModZipFn, HostGameStatusFn, HostHasSignalListenersFn,
    HostLogFn, HostModuleBaseFn, HostPatchLinkDataRowFn, HostReadMemoryFn,
    HostRegisterConfigSchemaFn, HostRegisterFileProviderFn, HostRegisterRegistryModuleFn,
    HostReplaceLinkDataEntryFn, HostRequireCapabilityFn, HostScanMemoryFn, HostSignalCallbackFn,
    HostSubscribeSignalFn, HostWriteMemoryFn, Oppw4ActiveCharacter, Oppw4ConfigSchema,
    Oppw4FileProvider, Oppw4GameStatus, Oppw4LinkDataEntryPatch, Oppw4LinkDataRowPatch,
    Oppw4LogEntry, Oppw4PluginModEntry, Oppw4RegistryModule,
};

pub(super) fn host_log(
    host_context: *mut c_void,
    log: HostLogFn,
    plugin_id: &CString,
    message: &CString,
) {
    let entry = Oppw4LogEntry {
        plugin_id: plugin_id.as_ptr(),
        message: message.as_ptr(),
    };
    unsafe { log(host_context, &entry) };
}

pub(super) fn module_base(host_context: *mut c_void, module_base: HostModuleBaseFn) -> usize {
    unsafe { module_base(host_context) }
}

pub(super) fn read_memory(
    host_context: *mut c_void,
    read: HostReadMemoryFn,
    address: usize,
    out: &mut [u8],
) -> i32 {
    unsafe { read(host_context, address, out.as_mut_ptr(), out.len()) }
}

pub(super) fn write_memory(
    host_context: *mut c_void,
    write: HostWriteMemoryFn,
    address: usize,
    bytes: &[u8],
) -> i32 {
    unsafe { write(host_context, address, bytes.as_ptr(), bytes.len()) }
}

pub(super) fn scan_memory(
    host_context: *mut c_void,
    scan: HostScanMemoryFn,
    pattern: &[u8],
    mask: &[u8],
) -> usize {
    unsafe { scan(host_context, pattern.as_ptr(), mask.as_ptr(), pattern.len()) }
}

pub(super) fn require_capability(
    host_context: *mut c_void,
    require: HostRequireCapabilityFn,
    plugin_id: &CStr,
    capability: &CStr,
) -> i32 {
    unsafe { require(host_context, plugin_id.as_ptr(), capability.as_ptr()) }
}

pub(super) fn register_file_provider(
    host_context: *mut c_void,
    register: HostRegisterFileProviderFn,
    provider: &Oppw4FileProvider,
) -> i32 {
    unsafe { register(host_context, provider) }
}

pub(super) fn register_registry_module(
    host_context: *mut c_void,
    register: HostRegisterRegistryModuleFn,
    module: &Oppw4RegistryModule,
) -> i32 {
    unsafe { register(host_context, module) }
}

pub(super) fn register_config_schema(
    host_context: *mut c_void,
    register: HostRegisterConfigSchemaFn,
    schema: &Oppw4ConfigSchema,
) -> i32 {
    unsafe { register(host_context, schema) }
}

pub(super) fn replace_linkdata_entry(
    host_context: *mut c_void,
    replace: HostReplaceLinkDataEntryFn,
    patch: &Oppw4LinkDataEntryPatch,
) -> i32 {
    unsafe { replace(host_context, patch) }
}

pub(super) fn patch_linkdata_row(
    host_context: *mut c_void,
    patch_row: HostPatchLinkDataRowFn,
    patch: &Oppw4LinkDataRowPatch,
) -> i32 {
    unsafe { patch_row(host_context, patch) }
}

pub(super) fn subscribe_signal(
    host_context: *mut c_void,
    subscribe: HostSubscribeSignalFn,
    signal: &CStr,
    subscriber_context: *mut c_void,
    callback: HostSignalCallbackFn,
) -> i32 {
    unsafe {
        subscribe(
            host_context,
            signal.as_ptr(),
            subscriber_context,
            Some(callback),
        )
    }
}

pub(super) fn emit_signal(
    host_context: *mut c_void,
    emit: HostEmitSignalFn,
    signal: &CStr,
    payload: &[u8],
) -> i32 {
    unsafe {
        emit(
            host_context,
            signal.as_ptr(),
            payload.as_ptr(),
            payload.len(),
        )
    }
}

pub(super) fn has_signal_listeners(
    host_context: *mut c_void,
    has_listeners: HostHasSignalListenersFn,
    signal: &CStr,
) -> i32 {
    unsafe { has_listeners(host_context, signal.as_ptr()) }
}

pub(super) fn game_status(
    host_context: *mut c_void,
    status: HostGameStatusFn,
) -> PluginResult<Oppw4GameStatus> {
    let mut out = Oppw4GameStatus::default();
    let code = unsafe { status(host_context, &mut out) };
    if code == 0 {
        Ok(out)
    } else {
        Err(PluginError::HostCallFailed {
            operation: "game_status",
            code,
        })
    }
}

pub(super) fn active_character(
    host_context: *mut c_void,
    active_character: HostActiveCharacterFn,
) -> PluginResult<Oppw4ActiveCharacter> {
    let mut out = Oppw4ActiveCharacter::default();
    let code = unsafe { active_character(host_context, &mut out) };
    if code == 0 {
        Ok(out)
    } else {
        Err(PluginError::HostCallFailed {
            operation: "active_character",
            code,
        })
    }
}

pub(super) fn debug_enabled(host_context: *mut c_void, debug_enabled: HostDebugEnabledFn) -> bool {
    unsafe { debug_enabled(host_context) != 0 }
}

pub(super) fn legacy_mod_paths(
    host_context: *mut c_void,
    for_each: HostForEachPluginModZipFn,
) -> Vec<String> {
    let mut paths = Vec::new();
    unsafe {
        let _ = for_each(
            host_context,
            Some(collect_plugin_mod_zip),
            (&mut paths as *mut Vec<String>).cast(),
        );
    }
    paths
}

pub(super) fn plugin_mods(
    host_context: *mut c_void,
    for_each: HostForEachPluginModFn,
) -> Vec<PluginModInfo> {
    let mut entries = Vec::new();
    unsafe {
        let _ = for_each(
            host_context,
            Some(collect_plugin_mod),
            (&mut entries as *mut Vec<PluginModInfo>).cast(),
        );
    }
    entries
}

unsafe extern "system" fn collect_plugin_mod_zip(
    user_context: *mut c_void,
    path_utf8: *const c_char,
) -> i32 {
    let Some(paths) = user_context.cast::<Vec<String>>().as_mut() else {
        return -1;
    };
    let Some(path) = optional_cstr(path_utf8) else {
        return -2;
    };
    paths.push(path.to_string_lossy().into_owned());
    0
}

unsafe extern "system" fn collect_plugin_mod(
    user_context: *mut c_void,
    entry: *const Oppw4PluginModEntry,
) -> i32 {
    let Some(entries) = user_context.cast::<Vec<PluginModInfo>>().as_mut() else {
        return -1;
    };
    let Some(entry) = entry.as_ref() else {
        return -2;
    };
    let Some(id) = optional_cstr(entry.id) else {
        return -3;
    };
    let Some(name) = optional_cstr(entry.name) else {
        return -4;
    };
    let Some(source_path) = optional_cstr(entry.source_path_utf8) else {
        return -5;
    };
    let Some(entry_path) = optional_cstr(entry.entry_utf8) else {
        return -6;
    };
    entries.push(PluginModInfo {
        id: id.to_string_lossy().into_owned(),
        name: name.to_string_lossy().into_owned(),
        source_path: source_path.to_string_lossy().into_owned(),
        entry: entry_path.to_string_lossy().into_owned(),
        flags: entry.flags,
    });
    0
}
