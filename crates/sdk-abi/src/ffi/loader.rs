use std::ffi::{c_char, c_void};

pub const OPPW4_LOADER_SDK_ABI_VERSION: u32 = 1;

pub type Oppw4LoaderLogFn =
    unsafe extern "system" fn(context: *mut c_void, message_utf8: *const c_char);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oppw4LoaderSdkInit {
    pub version: u32,
    pub host_context: *mut c_void,
    pub game_root_utf8: *const c_char,
    pub plugin_root_utf8: *const c_char,
    pub session_stamp_utf8: *const c_char,
    pub log: Option<Oppw4LoaderLogFn>,
}

pub type Oppw4SdkCoreInitializeFn =
    unsafe extern "system" fn(init: *const Oppw4LoaderSdkInit) -> i32;
