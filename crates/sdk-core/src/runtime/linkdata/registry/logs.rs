use plugin_sdk::linkdata::{LinkDataEntryId, LinkDataFile};

use crate::log;

pub(super) fn entry_patch_registered(
    plugin_id: &str,
    file: LinkDataFile,
    entry: LinkDataEntryId,
    patch_count: usize,
) {
    log::write_line(format!(
        "linkdata entry patch registered plugin={plugin_id} file={} entry={} patches={patch_count}",
        file.file_name(),
        entry.get()
    ));
}

pub(super) fn entry_patch_conflict(
    file: LinkDataFile,
    entry: LinkDataEntryId,
    owner: &str,
    rejected: &str,
) {
    log::write_line(format!(
        "linkdata entry patch conflict file={} entry={} owner={} rejected={rejected}",
        file.file_name(),
        entry.get(),
        owner
    ));
}

pub(super) fn row_patch_registered(
    plugin_id: &str,
    file: LinkDataFile,
    entry: LinkDataEntryId,
    patch_count: usize,
) {
    log::write_line(format!(
        "linkdata row patch registered plugin={plugin_id} file={} entry={} patches={patch_count}",
        file.file_name(),
        entry.get()
    ));
}

pub(super) fn row_patch_conflict(
    file: LinkDataFile,
    entry: LinkDataEntryId,
    owner: &str,
    rejected: &str,
) {
    log::write_line(format!(
        "linkdata row patch conflict file={} entry={} owner={} rejected={rejected}",
        file.file_name(),
        entry.get(),
        owner
    ));
}

pub(super) fn virtual_opened(file: LinkDataFile, handle: u64) {
    log::write_line(format!(
        "linkdata virtual open file={} handle={handle}",
        file.file_name()
    ));
}

pub(super) fn virtual_open_failed(file: LinkDataFile, error: &str) {
    log::write_line(format!(
        "linkdata virtual open failed file={} error={error}",
        file.file_name()
    ));
}
