use std::ffi::c_void;

use plugin_abi::{HostRdbPatchReadFn, HostRegisterRdbPatchProviderFn, Oppw4PluginApi};

use crate::{error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct RdbService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> RdbService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    /// # Safety
    ///
    /// `service_context` and `register_patch_provider` must remain valid while
    /// the service plugin is loaded.
    pub unsafe fn register_service(
        self,
        service_context: *mut c_void,
        register_patch_provider: HostRegisterRdbPatchProviderFn,
    ) -> PluginResult<()> {
        let register = self
            .abi
            .register_rdb_service
            .ok_or(PluginError::MissingHostFunction("register_rdb_service"))?;
        let code = unsafe {
            register(
                self.abi.host_context,
                service_context,
                Some(register_patch_provider),
            )
        };
        host_code_result("register_rdb_service", code)
    }

    /// # Safety
    ///
    /// `provider_context` and `patch_read` must remain valid while the plugin
    /// is loaded. The callback must not retain the path or buffer pointers
    /// after returning.
    pub unsafe fn register_patch_provider(
        self,
        provider_context: *mut c_void,
        patch_read: HostRdbPatchReadFn,
    ) -> PluginResult<()> {
        let register =
            self.abi
                .register_rdb_patch_provider
                .ok_or(PluginError::MissingHostFunction(
                    "register_rdb_patch_provider",
                ))?;
        let code = unsafe { register(self.abi.host_context, provider_context, Some(patch_read)) };
        host_code_result("register_rdb_patch_provider", code)
    }
}

fn host_code_result(operation: &'static str, code: i32) -> PluginResult<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(PluginError::HostCallFailed { operation, code })
    }
}
