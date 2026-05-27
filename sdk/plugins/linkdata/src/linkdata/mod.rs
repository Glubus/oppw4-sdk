mod provider;
mod registry;
mod state;
mod virtual_file;

use std::sync::{Mutex, OnceLock};

use plugin_sdk::{OwnedHostApi, PluginResult};
use registry::LinkDataRegistry;

static REGISTRY: OnceLock<Mutex<LinkDataRegistry>> = OnceLock::new();

pub(crate) fn initialize(host: OwnedHostApi) -> PluginResult<()> {
    let game_root = host.paths().game_root().unwrap_or_default();
    let _ = REGISTRY.set(Mutex::new(LinkDataRegistry::new(game_root.into())));
    unsafe {
        host.linkdata().register_provider(
            std::ptr::null_mut(),
            api::replace_entry,
            api::patch_row,
        )?;
    }
    provider::register(host)
}

fn with_registry<T>(action: impl FnOnce(&mut LinkDataRegistry) -> T) -> Option<T> {
    let registry = REGISTRY.get()?;
    let mut guard = registry.lock().ok()?;
    Some(action(&mut guard))
}

mod api {
    use std::ffi::{c_void, CStr};

    use plugin_sdk::{
        linkdata::{LinkDataEntryId, LinkDataFile},
        Oppw4LinkDataEntryPatch, Oppw4LinkDataRowPatch,
    };

    use super::{state::RowPatch, with_registry};

    pub(super) unsafe extern "system" fn replace_entry(
        _context: *mut c_void,
        patch: *const Oppw4LinkDataEntryPatch,
    ) -> i32 {
        let request = match entry_request_from_abi(patch) {
            Ok(request) => request,
            Err(code) => return code,
        };
        with_registry(|registry| {
            registry.replace_entry(
                &request.plugin_id,
                request.file,
                request.entry,
                request.payload,
            )
        })
        .unwrap_or(-4)
    }

    pub(super) unsafe extern "system" fn patch_row(
        _context: *mut c_void,
        patch: *const Oppw4LinkDataRowPatch,
    ) -> i32 {
        let request = match row_request_from_abi(patch) {
            Ok(request) => request,
            Err(code) => return code,
        };
        with_registry(|registry| {
            registry.patch_row(
                &request.plugin_id,
                request.file,
                request.entry,
                request.patch,
            )
        })
        .unwrap_or(-4)
    }

    struct EntryRequest {
        plugin_id: String,
        file: LinkDataFile,
        entry: LinkDataEntryId,
        payload: Vec<u8>,
    }

    struct RowRequest {
        plugin_id: String,
        file: LinkDataFile,
        entry: LinkDataEntryId,
        patch: RowPatch,
    }

    unsafe fn entry_request_from_abi(
        patch: *const Oppw4LinkDataEntryPatch,
    ) -> Result<EntryRequest, i32> {
        let patch = patch.as_ref().ok_or(-1)?;
        Ok(EntryRequest {
            plugin_id: plugin_id_from_raw(patch.plugin_id)?,
            file: LinkDataFile::from_raw(patch.file).ok_or(-2)?,
            entry: LinkDataEntryId::new(patch.entry),
            payload: payload_from_raw(patch.payload, patch.payload_len)?,
        })
    }

    unsafe fn row_request_from_abi(patch: *const Oppw4LinkDataRowPatch) -> Result<RowRequest, i32> {
        let patch = patch.as_ref().ok_or(-1)?;
        Ok(RowRequest {
            plugin_id: plugin_id_from_raw(patch.plugin_id)?,
            file: LinkDataFile::from_raw(patch.file).ok_or(-2)?,
            entry: LinkDataEntryId::new(patch.entry),
            patch: row_patch_from_abi(patch).ok_or(-5)?,
        })
    }

    unsafe fn plugin_id_from_raw(plugin_id: *const std::ffi::c_char) -> Result<String, i32> {
        if plugin_id.is_null() {
            return Err(-1);
        }
        Ok(CStr::from_ptr(plugin_id).to_string_lossy().into_owned())
    }

    unsafe fn payload_from_raw(payload: *const u8, payload_len: usize) -> Result<Vec<u8>, i32> {
        if payload.is_null() && payload_len != 0 {
            return Err(-1);
        }
        Ok(std::slice::from_raw_parts(payload, payload_len).to_vec())
    }

    unsafe fn row_patch_from_abi(patch: &Oppw4LinkDataRowPatch) -> Option<RowPatch> {
        let payload = payload_from_raw(patch.payload, patch.payload_len).ok()?;
        match patch.operation {
            plugin_sdk::OPPW4_LINKDATA_ROW_OP_REPLACE => Some(RowPatch::Replace {
                section: patch.section as usize,
                record_size: patch.record_size as usize,
                row: patch.row as usize,
                payload,
            }),
            plugin_sdk::OPPW4_LINKDATA_ROW_OP_INSERT => Some(RowPatch::Insert {
                section: patch.section as usize,
                record_size: patch.record_size as usize,
                row: patch.row as usize,
                payload,
            }),
            plugin_sdk::OPPW4_LINKDATA_ROW_OP_REMOVE => Some(RowPatch::Remove {
                section: patch.section as usize,
                record_size: patch.record_size as usize,
                row: patch.row as usize,
            }),
            _ => None,
        }
    }
}
