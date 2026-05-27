use std::{
    ffi::{c_char, c_void},
    slice,
    sync::{Mutex, OnceLock},
};

use plugin_sdk::HostApi;
use serde::Deserialize;

use crate::PLUGIN_ID;

static DEBUG_PANEL: OnceLock<Mutex<Option<DebugPanel>>> = OnceLock::new();

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct DebugPanel {
    pub(crate) schema: String,
    #[serde(default)]
    pub(crate) watches: Vec<DebugWatch>,
    #[serde(default)]
    pub(crate) scans: Vec<DebugScan>,
}

impl DebugPanel {
    pub(crate) fn summary(&self) -> String {
        format!(
            "schema={} watches={} scans={}",
            self.schema,
            self.watches.len(),
            self.scans.len()
        )
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct DebugWatch {
    pub(crate) id: String,
    pub(crate) address: String,
    #[serde(rename = "type")]
    pub(crate) value_type: String,
    pub(crate) value: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct DebugScan {
    pub(crate) id: String,
    pub(crate) hits: String,
    pub(crate) hit_count: usize,
}

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

pub(crate) fn debug_snapshot() -> Option<DebugPanel> {
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
    let Ok(panel) = serde_json::from_str::<DebugPanel>(text) else {
        return -4;
    };
    let Ok(mut snapshot) = DEBUG_PANEL.get_or_init(|| Mutex::new(None)).lock() else {
        return -3;
    };
    *snapshot = Some(panel);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_debug_panel_snapshot() {
        let panel: DebugPanel = serde_json::from_str(
            r#"{
              "schema": "sdk.debug.snapshot.v1",
              "watches": [{"id":"difficulty","address":"0x1234","type":"U8","value":"addr=0x1234 type=U8 value=2 raw=02"}],
              "scans": [{"id":"souls","hits":"1@0x1000(+0x0)","hit_count":1}]
            }"#,
        )
        .expect("panel");

        assert_eq!(
            panel.summary(),
            "schema=sdk.debug.snapshot.v1 watches=1 scans=1"
        );
        assert_eq!(panel.watches[0].id, "difficulty");
        assert_eq!(panel.scans[0].hit_count, 1);
    }
}
