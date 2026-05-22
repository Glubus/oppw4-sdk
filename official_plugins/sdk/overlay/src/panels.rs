use std::{
    ffi::{c_char, c_void},
    slice,
    sync::{Mutex, OnceLock},
};

use plugin_sdk::HostApi;

use crate::PLUGIN_ID;

static DEBUG_PANEL: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub(crate) fn subscribe(host: HostApi<'_>) {
    let result = unsafe {
        host.signals().subscribe_bytes(
            "sdk.debug.snapshot",
            std::ptr::null_mut(),
            debug_snapshot_callback,
        )
    };
    match result {
        Ok(()) => {
            let _ = host
                .log()
                .write(PLUGIN_ID, "sdk_overlay subscribed to sdk.debug.snapshot");
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("sdk_overlay debug snapshot subscribe failed: {error}"),
            );
        }
    }
}

pub(crate) fn debug_snapshot() -> Option<String> {
    DEBUG_PANEL
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|snapshot| snapshot.clone())
}

unsafe extern "system" fn debug_snapshot_callback(
    _subscriber_context: *mut c_void,
    _signal_utf8: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    if payload.is_null() && payload_len != 0 {
        return -1;
    }
    let bytes = unsafe { slice::from_raw_parts(payload, payload_len) };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return -2;
    };
    let Ok(mut snapshot) = DEBUG_PANEL.get_or_init(|| Mutex::new(None)).lock() else {
        return -3;
    };
    *snapshot = Some(text.to_string());
    0
}
