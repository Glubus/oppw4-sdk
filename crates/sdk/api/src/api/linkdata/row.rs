use plugin_abi::{
    Oppw4LinkDataRowPatch, OPPW4_LINKDATA_ROW_OP_INSERT, OPPW4_LINKDATA_ROW_OP_REMOVE,
    OPPW4_LINKDATA_ROW_OP_REPLACE,
};

use super::{entry::host_code_result, LinkDataRowTarget};
use crate::{
    api::{linkdata::LinkDataService, r#unsafe},
    cstring_lossy,
    error::PluginError,
    PluginResult,
};

impl<'api> LinkDataService<'api> {
    pub fn replace_row(
        self,
        plugin_id: &str,
        target: LinkDataRowTarget,
        payload: &[u8],
    ) -> PluginResult<()> {
        self.patch_row(
            plugin_id,
            target,
            RowOperation::Replace,
            payload,
            "replace_linkdata_row",
        )
    }

    pub fn insert_row(
        self,
        plugin_id: &str,
        target: LinkDataRowTarget,
        payload: &[u8],
    ) -> PluginResult<()> {
        self.patch_row(
            plugin_id,
            target,
            RowOperation::Insert,
            payload,
            "insert_linkdata_row",
        )
    }

    pub fn remove_row(self, plugin_id: &str, target: LinkDataRowTarget) -> PluginResult<()> {
        self.patch_row(
            plugin_id,
            target,
            RowOperation::Remove,
            &[],
            "remove_linkdata_row",
        )
    }

    fn patch_row(
        self,
        plugin_id: &str,
        target: LinkDataRowTarget,
        operation: RowOperation,
        payload: &[u8],
        operation_name: &'static str,
    ) -> PluginResult<()> {
        let patch_row = self
            .abi
            .patch_linkdata_row
            .ok_or(PluginError::MissingHostFunction("patch_linkdata_row"))?;
        let plugin_id = cstring_lossy(plugin_id);
        let patch = row_patch(plugin_id.as_ptr(), target, operation, payload);
        let code = r#unsafe::patch_linkdata_row(self.abi.host_context, patch_row, &patch);
        host_code_result(operation_name, code)
    }
}

#[derive(Clone, Copy)]
enum RowOperation {
    Replace,
    Insert,
    Remove,
}

impl RowOperation {
    const fn as_raw(self) -> u32 {
        match self {
            Self::Replace => OPPW4_LINKDATA_ROW_OP_REPLACE,
            Self::Insert => OPPW4_LINKDATA_ROW_OP_INSERT,
            Self::Remove => OPPW4_LINKDATA_ROW_OP_REMOVE,
        }
    }
}

fn row_patch(
    plugin_id: *const std::ffi::c_char,
    target: LinkDataRowTarget,
    operation: RowOperation,
    payload: &[u8],
) -> Oppw4LinkDataRowPatch {
    Oppw4LinkDataRowPatch {
        plugin_id,
        file: target.file.as_raw(),
        entry: target.entry.get(),
        operation: operation.as_raw(),
        section: target.section,
        record_size: target.record_size,
        row: target.row,
        payload: payload.as_ptr(),
        payload_len: payload.len(),
    }
}
