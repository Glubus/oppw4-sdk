use std::ffi::{c_char, c_void};

use super::super::{
    Oppw4ActiveCharacter, Oppw4FileProvider, Oppw4GameStatus, Oppw4LinkDataEntryPatch,
    Oppw4LinkDataRowPatch, Oppw4LogEntry, Oppw4LuaModule, Oppw4PluginModEntry,
};

pub type HostLogFn =
    unsafe extern "system" fn(host_context: *mut c_void, entry: *const Oppw4LogEntry);
pub type HostModuleBaseFn = unsafe extern "system" fn(host_context: *mut c_void) -> usize;
pub type HostReadMemoryFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    address: usize,
    out: *mut u8,
    len: usize,
) -> i32;
pub type HostWriteMemoryFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    address: usize,
    bytes: *const u8,
    len: usize,
) -> i32;
pub type HostScanMemoryFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    pattern: *const u8,
    mask: *const u8,
    len: usize,
) -> usize;
pub type HostPluginModZipVisitorFn =
    unsafe extern "system" fn(user_context: *mut c_void, path_utf8: *const c_char) -> i32;
pub type HostForEachPluginModZipFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    visitor: Option<HostPluginModZipVisitorFn>,
    user_context: *mut c_void,
) -> i32;
pub type HostPluginModVisitorFn =
    unsafe extern "system" fn(user_context: *mut c_void, entry: *const Oppw4PluginModEntry) -> i32;
pub type HostForEachPluginModFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    visitor: Option<HostPluginModVisitorFn>,
    user_context: *mut c_void,
) -> i32;
pub type HostRegisterFileProviderFn =
    unsafe extern "system" fn(host_context: *mut c_void, provider: *const Oppw4FileProvider) -> i32;
pub type HostGameStatusFn =
    unsafe extern "system" fn(host_context: *mut c_void, out_status: *mut Oppw4GameStatus) -> i32;
pub type HostRegisterGameStatusProviderFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    provider_context: *mut c_void,
    callback: Option<HostGameStatusFn>,
) -> i32;
pub type HostRegisterLuaModuleFn =
    unsafe extern "system" fn(host_context: *mut c_void, module: *const Oppw4LuaModule) -> i32;
pub type HostActiveCharacterFn =
    unsafe extern "system" fn(host_context: *mut c_void, out: *mut Oppw4ActiveCharacter) -> i32;
pub type HostRegisterActiveCharacterProviderFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    provider_context: *mut c_void,
    callback: Option<HostActiveCharacterFn>,
) -> i32;
pub type HostDebugEnabledFn = unsafe extern "system" fn(host_context: *mut c_void) -> i32;
pub type HostReplaceLinkDataEntryFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    patch: *const Oppw4LinkDataEntryPatch,
) -> i32;
pub type HostPatchLinkDataRowFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    patch: *const Oppw4LinkDataRowPatch,
) -> i32;
pub type HostRegisterLinkDataProviderFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    provider_context: *mut c_void,
    replace_entry: Option<HostReplaceLinkDataEntryFn>,
    patch_row: Option<HostPatchLinkDataRowFn>,
) -> i32;
pub type HostRequireCapabilityFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    plugin_id: *const c_char,
    capability: *const c_char,
) -> i32;
pub type HostRdbPatchReadFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    path_utf8: *const c_char,
    os_handle: usize,
    read_offset: u64,
    buffer: *mut u8,
    len: usize,
) -> i32;
pub type HostRegisterRdbPatchProviderFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    provider_context: *mut c_void,
    patch_read: Option<HostRdbPatchReadFn>,
) -> i32;
pub type HostRegisterRdbServiceFn = unsafe extern "system" fn(
    host_context: *mut c_void,
    service_context: *mut c_void,
    register_patch_provider: Option<HostRegisterRdbPatchProviderFn>,
) -> i32;
