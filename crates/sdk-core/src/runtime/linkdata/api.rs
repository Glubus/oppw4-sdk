use std::ffi::c_void;

use plugin_abi::{
    optional_cstr, Oppw4LinkDataEntryPatch, Oppw4LinkDataRowPatch, OPPW4_LINKDATA_ROW_OP_INSERT,
    OPPW4_LINKDATA_ROW_OP_REMOVE, OPPW4_LINKDATA_ROW_OP_REPLACE,
};

use super::{state::RowPatch, with_registry};
use crate::runtime::ffi::{context_from_raw, CAP_LINKDATA_PATCH};
use plugin_sdk::linkdata::{LinkDataEntryId, LinkDataFile};

pub(crate) unsafe extern "system" fn host_replace_linkdata_entry(
    host_context: *mut c_void,
    patch: *const Oppw4LinkDataEntryPatch,
) -> i32 {
    let request = match entry_request_from_abi(patch) {
        Ok(request) => request,
        Err(code) => return code,
    };
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) =
        context.require_capability_for_plugin_id(&request.plugin_id, CAP_LINKDATA_PATCH)
    {
        return code;
    }
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

pub(crate) unsafe extern "system" fn host_patch_linkdata_row(
    host_context: *mut c_void,
    patch: *const Oppw4LinkDataRowPatch,
) -> i32 {
    let request = match row_request_from_abi(patch) {
        Ok(request) => request,
        Err(code) => return code,
    };
    let context = match context_from_raw(host_context) {
        Ok(context) => context,
        Err(code) => return code,
    };
    if let Err(code) =
        context.require_capability_for_plugin_id(&request.plugin_id, CAP_LINKDATA_PATCH)
    {
        return code;
    }
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
    optional_cstr(plugin_id)
        .map(|plugin_id| plugin_id.to_string_lossy().into_owned())
        .ok_or(-1)
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
        OPPW4_LINKDATA_ROW_OP_REPLACE => Some(RowPatch::Replace {
            section: patch.section as usize,
            record_size: patch.record_size as usize,
            row: patch.row as usize,
            payload,
        }),
        OPPW4_LINKDATA_ROW_OP_INSERT => Some(RowPatch::Insert {
            section: patch.section as usize,
            record_size: patch.record_size as usize,
            row: patch.row as usize,
            payload,
        }),
        OPPW4_LINKDATA_ROW_OP_REMOVE => Some(RowPatch::Remove {
            section: patch.section as usize,
            record_size: patch.record_size as usize,
            row: patch.row as usize,
        }),
        _ => None,
    }
}
