mod edits;
mod io;
mod patch;

use std::path::Path;

use plugin_sdk::linkdata::{LinkDataArchive, LinkDataFile};

use super::virtual_file::VirtualFile;
use edits::LinkDataEdits;
pub(super) use patch::RowPatch;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum LinkDataFileKey {
    A,
}

impl From<LinkDataFile> for LinkDataFileKey {
    fn from(file: LinkDataFile) -> Self {
        match file {
            LinkDataFile::A => Self::A,
        }
    }
}

#[derive(Default)]
pub(super) struct LinkDataState {
    edits: LinkDataEdits,
    file: Option<VirtualFile>,
}

impl LinkDataState {
    pub(super) fn has_patches(&self) -> bool {
        self.edits.has_patches()
    }

    pub(super) fn patch_count(&self) -> usize {
        self.edits.patch_count()
    }

    pub(super) fn has_handle(&self, handle: u64) -> bool {
        self.file
            .as_ref()
            .is_some_and(|file| file.has_handle(handle))
    }

    pub(super) fn replace_entry(
        &mut self,
        plugin_id: &str,
        entry: plugin_sdk::linkdata::LinkDataEntryId,
        payload: Vec<u8>,
    ) -> Result<(), String> {
        self.edits.replace_entry(plugin_id, entry, payload)?;
        self.file = None;
        Ok(())
    }

    pub(super) fn patch_row(
        &mut self,
        plugin_id: &str,
        entry: plugin_sdk::linkdata::LinkDataEntryId,
        patch: RowPatch,
    ) -> Result<(), String> {
        self.edits.patch_row(plugin_id, entry, patch)?;
        self.file = None;
        Ok(())
    }

    pub(super) fn open(&mut self, base_path: &Path) -> Result<u64, String> {
        self.ensure_file(base_path)?;
        self.file
            .as_mut()
            .map(VirtualFile::open)
            .ok_or_else(|| "virtual file missing".to_string())
    }

    fn ensure_file(&mut self, base_path: &Path) -> Result<(), String> {
        if self.file.is_some() {
            return Ok(());
        }
        let archive = read_archive(base_path)?;
        let edits = self.edits.entry_payloads(&archive)?;
        let patched = archive
            .rebuild_with_entry_payloads(&edits)
            .map_err(|error| error.to_string())?;
        self.file = Some(VirtualFile::new(patched));
        Ok(())
    }
}

fn read_archive(base_path: &Path) -> Result<LinkDataArchive, String> {
    let base = std::fs::read(base_path).map_err(|error| {
        format!(
            "base read failed path={} error={error}",
            base_path.display()
        )
    })?;
    LinkDataArchive::parse(base).map_err(|error| error.to_string())
}
