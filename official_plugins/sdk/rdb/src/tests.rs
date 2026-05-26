use super::*;
use plugin_sdk::Plugin;

#[test]
fn declares_stable_plugin_id() {
    assert_eq!(SdkPlugin::ID, "sdk_rdb");
}

#[test]
fn forget_handle_removes_open_handle() {
    let provider = test_provider();
    let handle = remember_handle(OpenRdbHandle {
        provider,
        provider_handle: 777,
    });

    assert_eq!(
        open_handle(handle).map(|open| open.provider_handle),
        Some(777)
    );
    assert_eq!(
        forget_handle(handle).map(|open| open.provider_handle),
        Some(777)
    );
    assert!(open_handle(handle).is_none());
    assert!(forget_handle(handle).is_none());
}

fn test_provider() -> RdbVirtualProvider {
    RdbVirtualProvider {
        context: 0,
        open_path: rejected_open_path,
        read: empty_read,
        close: ok_close,
        size: zero_size,
        file_time: None,
        seek: zero_seek,
        patch_read: None,
    }
}

unsafe extern "system" fn rejected_open_path(
    _provider_context: *mut c_void,
    _path_utf8: *const c_char,
    _out_handle: *mut u64,
) -> i32 {
    0
}

unsafe extern "system" fn empty_read(
    _provider_context: *mut c_void,
    _handle: u64,
    _buffer: *mut u8,
    _bytes_to_read: u32,
    _requested_offset: i64,
    out_bytes_read: *mut u32,
) -> i32 {
    if !out_bytes_read.is_null() {
        unsafe {
            *out_bytes_read = 0;
        }
    }
    1
}

unsafe extern "system" fn ok_close(_provider_context: *mut c_void, _handle: u64) -> i32 {
    1
}

unsafe extern "system" fn zero_size(
    _provider_context: *mut c_void,
    _handle: u64,
    out_size: *mut u64,
) -> i32 {
    if !out_size.is_null() {
        unsafe {
            *out_size = 0;
        }
    }
    1
}

unsafe extern "system" fn zero_seek(
    _provider_context: *mut c_void,
    _handle: u64,
    _distance: i64,
    _move_method: u32,
    out_position: *mut u64,
) -> i32 {
    if !out_position.is_null() {
        unsafe {
            *out_position = 0;
        }
    }
    1
}
