use std::{
    ffi::{c_char, c_void},
    slice,
    sync::{Mutex, OnceLock},
};

use plugin_sdk::HostApi;
use serde::Deserialize;

use crate::PLUGIN_ID;

static DEBUG_PANEL: OnceLock<Mutex<Option<DebugPanel>>> = OnceLock::new();
static HOST_STATUS: OnceLock<Mutex<Option<HostStatus>>> = OnceLock::new();

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
    subscribe_debug_snapshot(host);
    subscribe_host_status(host);
}

fn subscribe_debug_snapshot(host: HostApi<'_>) {
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

fn subscribe_host_status(host: HostApi<'_>) {
    let result = unsafe {
        host.signals().subscribe_bytes(
            "sdk.host.status",
            std::ptr::null_mut(),
            host_status_callback,
        )
    };
    match result {
        Ok(()) => {
            let _ = host
                .log()
                .write(PLUGIN_ID, "sdk_overlay subscribed to sdk.host.status");
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("sdk_overlay host status subscribe failed: {error}"),
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

pub(crate) fn host_status() -> Option<HostStatus> {
    HOST_STATUS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|status| status.clone())
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct HostStatus {
    pub(crate) schema: String,
    pub(crate) message: String,
}

impl HostStatus {
    pub(crate) fn summary(&self) -> String {
        format!("schema={} message={}", self.schema, self.message)
    }
}

unsafe extern "system" fn host_status_callback(
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
    let Ok(status) = serde_json::from_str::<HostStatus>(text) else {
        return -4;
    };
    let Ok(mut snapshot) = HOST_STATUS.get_or_init(|| Mutex::new(None)).lock() else {
        return -3;
    };
    *snapshot = Some(status);
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

    #[test]
    fn parses_host_status_snapshot() {
        let status: HostStatus =
            serde_json::from_str(r#"{"schema":"sdk.host.status.v1","message":"mods loaded 4/10"}"#)
                .expect("status");

        assert_eq!(
            status.summary(),
            "schema=sdk.host.status.v1 message=mods loaded 4/10"
        );
    }
}
