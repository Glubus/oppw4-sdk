use std::time::Duration;

pub(crate) const PLUGIN_ID: &str = "sdk_runtime";

pub(crate) fn snapshot_interval(interval_ms: u64) -> Option<Duration> {
    if interval_ms == 0 {
        None
    } else {
        Some(Duration::from_millis(interval_ms.max(250)))
    }
}
