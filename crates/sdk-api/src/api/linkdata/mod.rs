mod entry;
mod row;
mod target;

use std::ffi::c_void;

use plugin_abi::{HostPatchLinkDataRowFn, HostReplaceLinkDataEntryFn, Oppw4PluginApi};

use crate::{error::PluginError, PluginResult};

pub use target::LinkDataRowTarget;

#[derive(Clone, Copy)]
pub struct LinkDataService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> LinkDataService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    /// # Safety
    ///
    /// `provider_context`, `replace_entry`, and `patch_row` must remain valid
    /// while the plugin is loaded. The callbacks must follow the SDK LinkData
    /// ABI and must not retain patch pointers after returning.
    pub unsafe fn register_provider(
        self,
        provider_context: *mut c_void,
        replace_entry: HostReplaceLinkDataEntryFn,
        patch_row: HostPatchLinkDataRowFn,
    ) -> PluginResult<()> {
        let register =
            self.abi
                .register_linkdata_provider
                .ok_or(PluginError::MissingHostFunction(
                    "register_linkdata_provider",
                ))?;
        let code = unsafe {
            register(
                self.abi.host_context,
                provider_context,
                Some(replace_entry),
                Some(patch_row),
            )
        };
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "register_linkdata_provider",
                code,
            })
        }
    }
}
