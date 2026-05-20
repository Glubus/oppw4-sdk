use std::{
    collections::BTreeSet,
    sync::{Mutex, OnceLock},
};

use plugin_sdk::{
    linkdata::{LinkDataEntryId, LinkDataFile},
    HostApi, OwnedHostApi,
};

use crate::constants::PLUGIN_ID;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static ENTRIES: OnceLock<Mutex<BTreeSet<usize>>> = OnceLock::new();

pub(crate) fn initialize(host: HostApi<'_>) -> Result<(), String> {
    let _ = HOST.set(host.owned());
    Ok(())
}

pub(crate) fn replace_entry(entry: usize, payload: &[u8]) -> Result<(), String> {
    let host = HOST
        .get()
        .ok_or_else(|| "moveset host state is not initialized".to_string())?;
    host.linkdata()
        .replace_entry(
            PLUGIN_ID,
            LinkDataFile::A,
            LinkDataEntryId::new(entry as u32),
            payload,
        )
        .map_err(|error| error.to_string())?;
    if let Ok(mut entries) = ENTRIES.get_or_init(|| Mutex::new(BTreeSet::new())).lock() {
        entries.insert(entry);
    }
    Ok(())
}

pub(crate) fn edit_count() -> usize {
    ENTRIES
        .get_or_init(|| Mutex::new(BTreeSet::new()))
        .lock()
        .map(|entries| entries.len())
        .unwrap_or(0)
}
