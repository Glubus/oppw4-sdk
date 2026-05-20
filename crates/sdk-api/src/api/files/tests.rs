use std::{
    ffi::{c_char, c_void, CStr},
    sync::{Mutex, OnceLock},
};

use plugin_abi::{null_api, Oppw4FileProvider, Oppw4PluginApi};

use super::{FileService, VirtualFileProvider};

static CAPTURED: OnceLock<Mutex<CapturedProvider>> = OnceLock::new();

#[derive(Default)]
struct CapturedProvider {
    plugin_id: String,
    provider_context: usize,
    has_open_path: bool,
    has_read: bool,
    has_close: bool,
    has_size: bool,
    has_file_time: bool,
    has_seek: bool,
    has_patch_read: bool,
}

unsafe extern "system" fn register_file_provider(
    _host_context: *mut c_void,
    provider: *const Oppw4FileProvider,
) -> i32 {
    let provider = &*provider;
    let plugin_id = CStr::from_ptr(provider.plugin_id).to_string_lossy();
    *CAPTURED
        .get_or_init(|| Mutex::new(CapturedProvider::default()))
        .lock()
        .expect("captured provider lock") = CapturedProvider {
        plugin_id: plugin_id.into_owned(),
        provider_context: provider.provider_context as usize,
        has_open_path: provider.open_path.is_some(),
        has_read: provider.read.is_some(),
        has_close: provider.close.is_some(),
        has_size: provider.size.is_some(),
        has_file_time: provider.file_time.is_some(),
        has_seek: provider.seek.is_some(),
        has_patch_read: provider.patch_read.is_some(),
    };
    0
}

unsafe extern "system" fn open_path(
    _provider_context: *mut c_void,
    _path_utf8: *const c_char,
    _out_handle: *mut u64,
) -> i32 {
    0
}

unsafe extern "system" fn read(
    _provider_context: *mut c_void,
    _handle: u64,
    _buffer: *mut u8,
    _bytes_to_read: u32,
    _requested_offset: i64,
    _out_bytes_read: *mut u32,
) -> i32 {
    0
}

unsafe extern "system" fn close(_provider_context: *mut c_void, _handle: u64) -> i32 {
    0
}

unsafe extern "system" fn size(
    _provider_context: *mut c_void,
    _handle: u64,
    _out_size: *mut u64,
) -> i32 {
    0
}

unsafe extern "system" fn file_time(
    _provider_context: *mut c_void,
    _handle: u64,
    _out_filetime: *mut u64,
) -> i32 {
    0
}

unsafe extern "system" fn seek(
    _provider_context: *mut c_void,
    _handle: u64,
    _distance: i64,
    _move_method: u32,
    _out_position: *mut u64,
) -> i32 {
    0
}

unsafe extern "system" fn patch_read(
    _provider_context: *mut c_void,
    _path_utf8: *const c_char,
    _os_handle: usize,
    _read_offset: u64,
    _buffer: *mut u8,
    _len: usize,
) -> i32 {
    0
}

#[test]
fn virtual_provider_builder_registers_full_abi_provider() {
    let api = Oppw4PluginApi {
        register_file_provider: Some(register_file_provider),
        ..null_api()
    };
    let provider_context = 0x1234usize as *mut c_void;
    let provider = VirtualFileProvider::new("skin_patcher", open_path, read, close, size)
        .context(provider_context)
        .file_time(file_time)
        .seek(seek)
        .patch_read(patch_read);

    FileService::new(&api)
        .register_virtual_provider(provider)
        .expect("register provider");

    let captured = CAPTURED
        .get()
        .expect("captured provider")
        .lock()
        .expect("captured provider lock");
    assert_eq!(captured.plugin_id, "skin_patcher");
    assert_eq!(captured.provider_context, provider_context as usize);
    assert!(captured.has_open_path);
    assert!(captured.has_read);
    assert!(captured.has_close);
    assert!(captured.has_size);
    assert!(captured.has_file_time);
    assert!(captured.has_seek);
    assert!(captured.has_patch_read);
}
