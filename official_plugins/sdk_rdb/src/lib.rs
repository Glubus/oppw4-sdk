use std::{
    ffi::{c_char, c_void},
    sync::{Mutex, OnceLock},
};

use plugin_sdk::{
    export_plugin, HostRdbPatchReadFn, Plugin, PluginContext, PluginResult, VirtualFileProvider,
};

static PATCH_PROVIDERS: OnceLock<Mutex<Vec<RdbPatchProvider>>> = OnceLock::new();

#[derive(Clone, Copy)]
struct RdbPatchProvider {
    context: usize,
    patch_read: HostRdbPatchReadFn,
}

struct SdkRdb;

impl Plugin for SdkRdb {
    const ID: &'static str = "sdk.rdb";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let host = context.host();
        unsafe {
            host.rdb()
                .register_service(std::ptr::null_mut(), register_patch_provider)?;
        }
        host.files().register_virtual_provider(
            VirtualFileProvider::new("sdk.rdb", noop_open, noop_read, noop_close, noop_size)
                .seek(noop_seek)
                .patch_read(dispatch_patch_read),
        )?;
        context.log("sdk.rdb initialized");
        Ok(())
    }
}

export_plugin!(SdkRdb);

unsafe extern "system" fn register_patch_provider(
    _service_context: *mut c_void,
    provider_context: *mut c_void,
    patch_read: Option<HostRdbPatchReadFn>,
) -> i32 {
    let Some(patch_read) = patch_read else {
        return -1;
    };
    let providers = PATCH_PROVIDERS.get_or_init(|| Mutex::new(Vec::new()));
    let Ok(mut guard) = providers.lock() else {
        return -2;
    };
    guard.push(RdbPatchProvider {
        context: provider_context as usize,
        patch_read,
    });
    0
}

unsafe extern "system" fn dispatch_patch_read(
    _provider_context: *mut c_void,
    path_utf8: *const c_char,
    os_handle: usize,
    read_offset: u64,
    buffer: *mut u8,
    len: usize,
) -> i32 {
    let Some(providers) = PATCH_PROVIDERS.get() else {
        return 0;
    };
    let Ok(guard) = providers.lock() else {
        return -2;
    };
    let mut patched = 0i32;
    for provider in guard.iter() {
        let code = unsafe {
            (provider.patch_read)(
                provider.context as *mut c_void,
                path_utf8,
                os_handle,
                read_offset,
                buffer,
                len,
            )
        };
        if code < 0 {
            return code;
        }
        patched = patched.saturating_add(code);
    }
    patched
}

unsafe extern "system" fn noop_open(
    _provider_context: *mut c_void,
    _path_utf8: *const c_char,
    _out_handle: *mut u64,
) -> i32 {
    0
}

unsafe extern "system" fn noop_read(
    _provider_context: *mut c_void,
    _handle: u64,
    _buffer: *mut u8,
    _bytes_to_read: u32,
    _requested_offset: i64,
    _out_bytes_read: *mut u32,
) -> i32 {
    0
}

unsafe extern "system" fn noop_close(_provider_context: *mut c_void, _handle: u64) -> i32 {
    0
}

unsafe extern "system" fn noop_size(
    _provider_context: *mut c_void,
    _handle: u64,
    _out_size: *mut u64,
) -> i32 {
    0
}

unsafe extern "system" fn noop_seek(
    _provider_context: *mut c_void,
    _handle: u64,
    _distance: i64,
    _move_method: u32,
    _out_position: *mut u64,
) -> i32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_stable_plugin_id() {
        assert_eq!(SdkRdb::ID, "sdk.rdb");
    }
}
