use std::sync::{Mutex, OnceLock};

use plugin_sdk::HostApi;

static ENTRIES: OnceLock<Mutex<usize>> = OnceLock::new();

pub(crate) fn initialize(host: HostApi<'_>) -> Result<(), String> {
    let _ = host;
    let _ = ENTRIES.get_or_init(|| Mutex::new(0));
    Ok(())
}

pub(crate) fn edit_count() -> usize {
    ENTRIES
        .get_or_init(|| Mutex::new(0))
        .lock()
        .map(|entries| *entries)
        .unwrap_or(0)
}
