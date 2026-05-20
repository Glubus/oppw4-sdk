use std::ffi::{c_char, c_void};

pub type Oppw4ProviderOpenPathFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    path_utf8: *const c_char,
    out_handle: *mut u64,
) -> i32;
pub type Oppw4ProviderReadFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    buffer: *mut u8,
    bytes_to_read: u32,
    requested_offset: i64,
    out_bytes_read: *mut u32,
) -> i32;
pub type Oppw4ProviderCloseFn =
    unsafe extern "system" fn(provider_context: *mut c_void, handle: u64) -> i32;
pub type Oppw4ProviderSizeFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    out_size: *mut u64,
) -> i32;
pub type Oppw4ProviderFileTimeFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    out_filetime: *mut u64,
) -> i32;
pub type Oppw4ProviderSeekFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    handle: u64,
    distance: i64,
    move_method: u32,
    out_position: *mut u64,
) -> i32;
pub type Oppw4ProviderPatchReadFn = unsafe extern "system" fn(
    provider_context: *mut c_void,
    path_utf8: *const c_char,
    os_handle: usize,
    read_offset: u64,
    buffer: *mut u8,
    len: usize,
) -> i32;
