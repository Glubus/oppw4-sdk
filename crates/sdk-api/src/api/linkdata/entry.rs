use plugin_abi::Oppw4LinkDataEntryPatch;

use crate::{
    api::{linkdata::LinkDataService, r#unsafe},
    cstring_lossy,
    error::PluginError,
    linkdata::{LinkDataEntryId, LinkDataFile},
    PluginResult,
};

impl<'api> LinkDataService<'api> {
    pub fn replace_entry(
        self,
        plugin_id: &str,
        file: LinkDataFile,
        entry: LinkDataEntryId,
        payload: &[u8],
    ) -> PluginResult<()> {
        let replace = self
            .abi
            .replace_linkdata_entry
            .ok_or(PluginError::MissingHostFunction("replace_linkdata_entry"))?;
        let plugin_id = cstring_lossy(plugin_id);
        let patch = Oppw4LinkDataEntryPatch {
            plugin_id: plugin_id.as_ptr(),
            file: file.as_raw(),
            entry: entry.get(),
            payload: payload.as_ptr(),
            payload_len: payload.len(),
        };
        let code = r#unsafe::replace_linkdata_entry(self.abi.host_context, replace, &patch);
        host_code_result("replace_linkdata_entry", code)
    }
}

pub(super) fn host_code_result(operation: &'static str, code: i32) -> PluginResult<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(PluginError::HostCallFailed { operation, code })
    }
}
