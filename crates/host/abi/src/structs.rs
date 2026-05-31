use std::ffi::{c_char, c_void};

use super::ffi::{
    HostActiveCharacterFn, HostDebugEnabledFn, HostEmitSignalFn, HostForEachPluginModFn,
    HostForEachPluginModZipFn, HostGameStatusFn, HostHasSignalListenersFn, HostLogFn,
    HostModuleBaseFn, HostPatchLinkDataRowFn, HostReadMemoryFn,
    HostRegisterActiveCharacterProviderFn, HostRegisterConfigSchemaFn, HostRegisterFileProviderFn,
    HostRegisterGameStatusProviderFn, HostRegisterLinkDataProviderFn,
    HostRegisterRdbPatchProviderFn, HostRegisterRdbServiceFn, HostRegisterRdbVirtualProviderFn,
    HostRegisterRegistryModuleFn, HostReplaceLinkDataEntryFn, HostRequireCapabilityFn,
    HostScanMemoryFn, HostSubscribeSignalFn, HostWriteMemoryFn, Oppw4ProviderCloseFn,
    Oppw4ProviderFileTimeFn, Oppw4ProviderOpenPathFn, Oppw4ProviderPatchReadFn,
    Oppw4ProviderReadFn, Oppw4ProviderSeekFn, Oppw4ProviderSizeFn, Oppw4RegistryModuleInstallFn,
    Oppw4RegistryModuleInvokeFn,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4PluginApi {
    pub version: u32,
    pub struct_size: u32,
    pub host_context: *mut c_void,
    pub game_root_utf8: *const c_char,
    pub plugin_root_utf8: *const c_char,
    pub mods_root_utf8: *const c_char,
    pub config_root_utf8: *const c_char,
    pub log: Option<HostLogFn>,
    pub module_base: Option<HostModuleBaseFn>,
    pub read_memory: Option<HostReadMemoryFn>,
    pub write_memory: Option<HostWriteMemoryFn>,
    pub scan_memory: Option<HostScanMemoryFn>,
    pub for_each_plugin_mod_zip: Option<HostForEachPluginModZipFn>,
    pub for_each_plugin_mod: Option<HostForEachPluginModFn>,
    pub register_file_provider: Option<HostRegisterFileProviderFn>,
    pub game_status: Option<HostGameStatusFn>,
    pub register_registry_module: Option<HostRegisterRegistryModuleFn>,
    pub active_character: Option<HostActiveCharacterFn>,
    pub debug_enabled: Option<HostDebugEnabledFn>,
    pub replace_linkdata_entry: Option<HostReplaceLinkDataEntryFn>,
    pub patch_linkdata_row: Option<HostPatchLinkDataRowFn>,
    pub require_capability: Option<HostRequireCapabilityFn>,
    pub register_game_status_provider: Option<HostRegisterGameStatusProviderFn>,
    pub register_active_character_provider: Option<HostRegisterActiveCharacterProviderFn>,
    pub register_linkdata_provider: Option<HostRegisterLinkDataProviderFn>,
    pub register_rdb_service: Option<HostRegisterRdbServiceFn>,
    pub register_rdb_patch_provider: Option<HostRegisterRdbPatchProviderFn>,
    pub register_rdb_virtual_provider: Option<HostRegisterRdbVirtualProviderFn>,
    pub subscribe_signal: Option<HostSubscribeSignalFn>,
    pub emit_signal: Option<HostEmitSignalFn>,
    pub register_config_schema: Option<HostRegisterConfigSchemaFn>,
    pub has_signal_listeners: Option<HostHasSignalListenersFn>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4LogEntry {
    pub plugin_id: *const c_char,
    pub message: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4PluginModEntry {
    pub id: *const c_char,
    pub name: *const c_char,
    pub source_path_utf8: *const c_char,
    pub entry_utf8: *const c_char,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginModInfo {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub entry: String,
    pub flags: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4FileProvider {
    pub plugin_id: *const c_char,
    pub provider_context: *mut c_void,
    pub open_path: Option<Oppw4ProviderOpenPathFn>,
    pub read: Option<Oppw4ProviderReadFn>,
    pub close: Option<Oppw4ProviderCloseFn>,
    pub size: Option<Oppw4ProviderSizeFn>,
    pub file_time: Option<Oppw4ProviderFileTimeFn>,
    pub seek: Option<Oppw4ProviderSeekFn>,
    pub patch_read: Option<Oppw4ProviderPatchReadFn>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4RegistryModule {
    pub plugin_id: *const c_char,
    pub module_name: *const c_char,
    pub module_context: *mut c_void,
    pub install: Option<Oppw4RegistryModuleInstallFn>,
    pub schema_json: *const c_char,
    pub invoke: Option<Oppw4RegistryModuleInvokeFn>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4ConfigSchema {
    pub plugin_id: *const c_char,
    pub schema_name: *const c_char,
    pub schema_utf8: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4LinkDataEntryPatch {
    pub plugin_id: *const c_char,
    pub file: u32,
    pub entry: u32,
    pub payload: *const u8,
    pub payload_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4LinkDataRowPatch {
    pub plugin_id: *const c_char,
    pub file: u32,
    pub entry: u32,
    pub operation: u32,
    pub section: u32,
    pub record_size: u32,
    pub row: u32,
    pub payload: *const u8,
    pub payload_len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Oppw4GameStatus {
    pub phase: u32,
    pub flags: u32,
    pub observed_file_opens: u32,
    pub seconds_since_host_start: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Oppw4ActiveCharacter {
    pub runtime_id: u16,
    pub alt_id: u16,
    pub flags: u32,
    pub local_player: usize,
    pub fx_owner: usize,
    pub source: usize,
    pub sequence: u64,
}
