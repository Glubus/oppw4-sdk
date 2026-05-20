use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use plugin_sdk::{
    linkdata::{LinkDataEntryId, LinkDataFile},
    HostApi, OwnedHostApi,
};

use crate::{constants::PLUGIN_ID, hex};

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static ENTRIES: OnceLock<Mutex<BTreeSet<usize>>> = OnceLock::new();

pub(crate) fn initialize(host: HostApi<'_>, mods_root: PathBuf) -> Result<(), String> {
    let _ = HOST.set(host.owned());
    load_legacy_entry_patches(&mods_root)
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

fn load_legacy_entry_patches(mods_root: &Path) -> Result<(), String> {
    let patch_root = mods_root.join("LINKDATA_A");
    let Ok(entries) = fs::read_dir(&patch_root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(index) = patch_entry_index(&path) else {
            continue;
        };
        replace_entry(index, &hex::read_payload(&path)?)?;
    }
    Ok(())
}

fn patch_entry_index(path: &Path) -> Option<usize> {
    let stem = path.file_stem()?.to_string_lossy();
    let raw = stem
        .strip_prefix("entry_")
        .or_else(|| stem.strip_prefix("entry-"))
        .unwrap_or(&stem);
    raw.parse::<usize>().ok()
}
