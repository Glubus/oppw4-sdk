use std::ffi::{c_char, c_void};

use super::{
    HostActiveCharacterFn, HostGameStatusFn, HostModuleBaseFn, HostReadMemoryFn,
    HostRegisterFileProviderFn, HostScanMemoryFn, HostWriteMemoryFn,
};

pub const OPPW4_LOADER_SDK_ABI_VERSION: u32 = 1;

pub type Oppw4LoaderLogFn =
    unsafe extern "system" fn(context: *mut c_void, message_utf8: *const c_char);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4LoaderSdkInit {
    pub version: u32,
    pub host_context: *mut c_void,
    pub debug_enabled: u32,
    pub game_root_utf8: *const c_char,
    pub plugin_root_utf8: *const c_char,
    pub session_stamp_utf8: *const c_char,
    pub log: Option<Oppw4LoaderLogFn>,
    pub module_base: Option<HostModuleBaseFn>,
    pub read_memory: Option<HostReadMemoryFn>,
    pub write_memory: Option<HostWriteMemoryFn>,
    pub scan_memory: Option<HostScanMemoryFn>,
    pub register_file_provider: Option<HostRegisterFileProviderFn>,
    pub game_status: Option<HostGameStatusFn>,
    pub active_character: Option<HostActiveCharacterFn>,
}

impl Default for Oppw4LoaderSdkInit {
    fn default() -> Self {
        Self {
            version: OPPW4_LOADER_SDK_ABI_VERSION,
            host_context: std::ptr::null_mut(),
            debug_enabled: 0,
            game_root_utf8: std::ptr::null(),
            plugin_root_utf8: std::ptr::null(),
            session_stamp_utf8: std::ptr::null(),
            log: None,
            module_base: None,
            read_memory: None,
            write_memory: None,
            scan_memory: None,
            register_file_provider: None,
            game_status: None,
            active_character: None,
        }
    }
}

pub type Oppw4SdkCoreInitializeFn =
    unsafe extern "system" fn(init: *const Oppw4LoaderSdkInit) -> i32;
