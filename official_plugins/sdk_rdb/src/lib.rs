use std::{
    collections::HashMap,
    ffi::{c_char, c_void},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
};

use plugin_sdk::{
    export_plugin, HostRdbPatchReadFn, Oppw4FileProvider, Plugin, PluginContext, PluginResult,
    VirtualFileProvider,
};

static PATCH_PROVIDERS: OnceLock<Mutex<Vec<RdbPatchProvider>>> = OnceLock::new();
static VIRTUAL_PROVIDERS: OnceLock<Mutex<Vec<RdbVirtualProvider>>> = OnceLock::new();
static OPEN_HANDLES: OnceLock<Mutex<HashMap<u64, OpenRdbHandle>>> = OnceLock::new();
static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy)]
struct RdbPatchProvider {
    context: usize,
    patch_read: HostRdbPatchReadFn,
}

#[derive(Clone, Copy)]
struct RdbVirtualProvider {
    context: usize,
    open_path: plugin_sdk::Oppw4ProviderOpenPathFn,
    read: plugin_sdk::Oppw4ProviderReadFn,
    close: plugin_sdk::Oppw4ProviderCloseFn,
    size: plugin_sdk::Oppw4ProviderSizeFn,
    file_time: Option<plugin_sdk::Oppw4ProviderFileTimeFn>,
    seek: plugin_sdk::Oppw4ProviderSeekFn,
    patch_read: Option<plugin_sdk::Oppw4ProviderPatchReadFn>,
}

#[derive(Clone, Copy)]
struct OpenRdbHandle {
    provider: RdbVirtualProvider,
    provider_handle: u64,
}

struct SdkRdb;

impl Plugin for SdkRdb {
    const ID: &'static str = "sdk.rdb";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let host = context.host();
        unsafe {
            host.rdb().register_service_with_virtual_provider(
                std::ptr::null_mut(),
                register_patch_provider,
                register_virtual_provider,
            )?;
        }
        host.files().register_virtual_provider(
            VirtualFileProvider::new(
                "sdk.rdb",
                dispatch_open,
                dispatch_read,
                dispatch_close,
                dispatch_size,
            )
            .file_time(dispatch_file_time)
            .seek(dispatch_seek)
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

unsafe extern "system" fn register_virtual_provider(
    _service_context: *mut c_void,
    provider: *const Oppw4FileProvider,
) -> i32 {
    let Some(provider) = provider.as_ref() else {
        return -1;
    };
    let (Some(open_path), Some(read), Some(close), Some(size), Some(seek)) = (
        provider.open_path,
        provider.read,
        provider.close,
        provider.size,
        provider.seek,
    ) else {
        return -2;
    };
    let providers = VIRTUAL_PROVIDERS.get_or_init(|| Mutex::new(Vec::new()));
    let Ok(mut guard) = providers.lock() else {
        return -3;
    };
    guard.push(RdbVirtualProvider {
        context: provider.provider_context as usize,
        open_path,
        read,
        close,
        size,
        file_time: provider.file_time,
        seek,
        patch_read: provider.patch_read,
    });
    0
}

unsafe extern "system" fn dispatch_open(
    _provider_context: *mut c_void,
    path_utf8: *const c_char,
    out_handle: *mut u64,
) -> i32 {
    if out_handle.is_null() {
        return -1;
    }
    let Some(providers) = VIRTUAL_PROVIDERS.get() else {
        return 0;
    };
    let Ok(guard) = providers.lock() else {
        return -2;
    };
    for provider in guard.iter().copied() {
        let mut provider_handle = 0u64;
        let code = unsafe {
            (provider.open_path)(
                provider.context as *mut c_void,
                path_utf8,
                &mut provider_handle,
            )
        };
        if code < 0 {
            return code;
        }
        if code == 0 {
            continue;
        }
        let handle = remember_handle(OpenRdbHandle {
            provider,
            provider_handle,
        });
        unsafe {
            *out_handle = handle;
        }
        return code;
    }
    0
}

unsafe extern "system" fn dispatch_read(
    _provider_context: *mut c_void,
    handle: u64,
    buffer: *mut u8,
    bytes_to_read: u32,
    requested_offset: i64,
    out_bytes_read: *mut u32,
) -> i32 {
    let Some(open) = open_handle(handle) else {
        return 0;
    };
    unsafe {
        (open.provider.read)(
            open.provider.context as *mut c_void,
            open.provider_handle,
            buffer,
            bytes_to_read,
            requested_offset,
            out_bytes_read,
        )
    }
}

unsafe extern "system" fn dispatch_close(_provider_context: *mut c_void, handle: u64) -> i32 {
    let Some(open) = forget_handle(handle) else {
        return 0;
    };
    unsafe { (open.provider.close)(open.provider.context as *mut c_void, open.provider_handle) }
}

unsafe extern "system" fn dispatch_size(
    _provider_context: *mut c_void,
    handle: u64,
    out_size: *mut u64,
) -> i32 {
    let Some(open) = open_handle(handle) else {
        return 0;
    };
    unsafe {
        (open.provider.size)(
            open.provider.context as *mut c_void,
            open.provider_handle,
            out_size,
        )
    }
}

unsafe extern "system" fn dispatch_file_time(
    _provider_context: *mut c_void,
    handle: u64,
    out_filetime: *mut u64,
) -> i32 {
    let Some(open) = open_handle(handle) else {
        return 0;
    };
    let Some(file_time) = open.provider.file_time else {
        return 0;
    };
    unsafe {
        file_time(
            open.provider.context as *mut c_void,
            open.provider_handle,
            out_filetime,
        )
    }
}

unsafe extern "system" fn dispatch_seek(
    _provider_context: *mut c_void,
    handle: u64,
    distance: i64,
    move_method: u32,
    out_position: *mut u64,
) -> i32 {
    let Some(open) = open_handle(handle) else {
        return 0;
    };
    unsafe {
        (open.provider.seek)(
            open.provider.context as *mut c_void,
            open.provider_handle,
            distance,
            move_method,
            out_position,
        )
    }
}

unsafe extern "system" fn dispatch_patch_read(
    _provider_context: *mut c_void,
    path_utf8: *const c_char,
    os_handle: usize,
    read_offset: u64,
    buffer: *mut u8,
    len: usize,
) -> i32 {
    let mut patched =
        match dispatch_legacy_patch_read(path_utf8, os_handle, read_offset, buffer, len) {
            Ok(patched) => patched,
            Err(code) => return code,
        };
    let Some(providers) = VIRTUAL_PROVIDERS.get() else {
        return patched;
    };
    let Ok(guard) = providers.lock() else {
        return -2;
    };
    for provider in guard.iter() {
        let Some(patch_read) = provider.patch_read else {
            continue;
        };
        let code = unsafe {
            patch_read(
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

fn dispatch_legacy_patch_read(
    path_utf8: *const c_char,
    os_handle: usize,
    read_offset: u64,
    buffer: *mut u8,
    len: usize,
) -> Result<i32, i32> {
    let Some(providers) = PATCH_PROVIDERS.get() else {
        return Ok(0);
    };
    let Ok(guard) = providers.lock() else {
        return Err(-2);
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
            return Err(code);
        }
        patched = patched.saturating_add(code);
    }
    Ok(patched)
}

fn remember_handle(open: OpenRdbHandle) -> u64 {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    let handles = OPEN_HANDLES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = handles.lock() {
        guard.insert(handle, open);
    }
    handle
}

fn open_handle(handle: u64) -> Option<OpenRdbHandle> {
    OPEN_HANDLES.get()?.lock().ok()?.get(&handle).copied()
}

fn forget_handle(handle: u64) -> Option<OpenRdbHandle> {
    OPEN_HANDLES.get()?.lock().ok()?.remove(&handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_stable_plugin_id() {
        assert_eq!(SdkRdb::ID, "sdk.rdb");
    }
}
