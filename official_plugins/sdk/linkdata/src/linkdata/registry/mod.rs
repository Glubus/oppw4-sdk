mod logs;
mod open;

use std::{collections::BTreeMap, path::PathBuf};

use plugin_sdk::linkdata::{LinkDataEntryId, LinkDataFile};

use super::{
    provider::linkdata_file_from_path,
    state::{LinkDataFileKey, LinkDataState, RowPatch},
};

pub(super) struct LinkDataRegistry {
    game_root: PathBuf,
    files: BTreeMap<LinkDataFileKey, LinkDataState>,
}

impl LinkDataRegistry {
    pub(super) fn new(game_root: PathBuf) -> Self {
        Self {
            game_root,
            files: BTreeMap::from([(LinkDataFileKey::A, LinkDataState::default())]),
        }
    }

    pub(super) fn replace_entry(
        &mut self,
        plugin_id: &str,
        file: LinkDataFile,
        entry: LinkDataEntryId,
        payload: Vec<u8>,
    ) -> i32 {
        let Some(state) = self.state_for_file_mut(file) else {
            return -2;
        };
        match state.replace_entry(plugin_id, entry, payload) {
            Ok(()) => {
                logs::entry_patch_registered(plugin_id, file, entry, state.patch_count());
                0
            }
            Err(owner) => {
                logs::entry_patch_conflict(file, entry, &owner, plugin_id);
                -3
            }
        }
    }

    pub(super) fn patch_row(
        &mut self,
        plugin_id: &str,
        file: LinkDataFile,
        entry: LinkDataEntryId,
        patch: RowPatch,
    ) -> i32 {
        let Some(state) = self.state_for_file_mut(file) else {
            return -2;
        };
        match state.patch_row(plugin_id, entry, patch) {
            Ok(()) => {
                logs::row_patch_registered(plugin_id, file, entry, state.patch_count());
                0
            }
            Err(owner) => {
                logs::row_patch_conflict(file, entry, &owner, plugin_id);
                -3
            }
        }
    }

    pub(super) unsafe fn open_path(&mut self, path: &str, out_handle: *mut u64) -> i32 {
        let Some(file) = linkdata_file_from_path(path) else {
            return 0;
        };
        let game_root = self.game_root.clone();
        let Some(state) = self.state_for_file_mut(file) else {
            return 0;
        };
        if !state.has_patches() {
            return 0;
        }
        open::open_virtual_file(state, &game_root, file, out_handle)
    }

    pub(super) unsafe fn read(
        &mut self,
        handle: u64,
        buffer: *mut u8,
        bytes_to_read: u32,
        requested_offset: i64,
        out_bytes_read: *mut u32,
    ) -> i32 {
        self.state_for_handle_mut(handle)
            .map(|state| {
                state.read(
                    handle,
                    buffer,
                    bytes_to_read,
                    requested_offset,
                    out_bytes_read,
                )
            })
            .unwrap_or(0)
    }

    pub(super) fn close(&mut self, handle: u64) -> i32 {
        self.state_for_handle_mut(handle)
            .map(|state| state.close(handle))
            .unwrap_or(0)
    }

    pub(super) unsafe fn size(&mut self, handle: u64, out_size: *mut u64) -> i32 {
        self.state_for_handle_mut(handle)
            .map(|state| state.size(out_size))
            .unwrap_or(0)
    }

    pub(super) unsafe fn seek(
        &mut self,
        handle: u64,
        distance: i64,
        move_method: u32,
        out_position: *mut u64,
    ) -> i32 {
        self.state_for_handle_mut(handle)
            .map(|state| state.seek(handle, distance, move_method, out_position))
            .unwrap_or(0)
    }

    fn state_for_handle_mut(&mut self, handle: u64) -> Option<&mut LinkDataState> {
        self.files
            .values_mut()
            .find(|state| state.has_handle(handle))
    }

    fn state_for_file_mut(&mut self, file: LinkDataFile) -> Option<&mut LinkDataState> {
        self.files.get_mut(&LinkDataFileKey::from(file))
    }
}
