use std::{
    ffi::{c_char, c_void, CStr},
    path::Path,
};

use plugin_sdk::linkdata::LinkDataFile;

use crate::log;

use super::with_registry;

pub(super) fn register() {
    let registered = hooks::register_file_provider(hooks::FileProviderRegistration {
        plugin_id: None,
        provider_context: std::ptr::null_mut(),
        open_path: provider_open_path,
        read: provider_read,
        close: provider_close,
        size: provider_size,
        file_time: None,
        seek: provider_seek,
        patch_read: None,
    });
    log::write_line(format!("linkdata host provider result={registered}"));
}

unsafe extern "system" fn provider_open_path(
    _context: *mut c_void,
    path_utf8: *const c_char,
    out_handle: *mut u64,
) -> i32 {
    if path_utf8.is_null() || out_handle.is_null() {
        return -1;
    }
    let path = CStr::from_ptr(path_utf8).to_string_lossy();
    with_registry(|registry| registry.open_path(&path, out_handle)).unwrap_or(0)
}

unsafe extern "system" fn provider_read(
    _context: *mut c_void,
    handle: u64,
    buffer: *mut u8,
    bytes_to_read: u32,
    requested_offset: i64,
    out_bytes_read: *mut u32,
) -> i32 {
    with_registry(|registry| {
        registry.read(
            handle,
            buffer,
            bytes_to_read,
            requested_offset,
            out_bytes_read,
        )
    })
    .unwrap_or(0)
}

unsafe extern "system" fn provider_close(_context: *mut c_void, handle: u64) -> i32 {
    with_registry(|registry| registry.close(handle)).unwrap_or(0)
}

unsafe extern "system" fn provider_size(
    _context: *mut c_void,
    handle: u64,
    out_size: *mut u64,
) -> i32 {
    with_registry(|registry| registry.size(handle, out_size)).unwrap_or(0)
}

unsafe extern "system" fn provider_seek(
    _context: *mut c_void,
    handle: u64,
    distance: i64,
    move_method: u32,
    out_position: *mut u64,
) -> i32 {
    with_registry(|registry| registry.seek(handle, distance, move_method, out_position))
        .unwrap_or(0)
}

pub(super) fn linkdata_file_from_path(path: &str) -> Option<LinkDataFile> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.eq_ignore_ascii_case(LinkDataFile::A.file_name()))
        .map(|_| LinkDataFile::A)
}
