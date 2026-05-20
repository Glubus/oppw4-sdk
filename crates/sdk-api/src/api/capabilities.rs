use crate::{api::r#unsafe, cstring_lossy, error::PluginError, PluginResult};
use plugin_abi::Oppw4PluginApi;

#[derive(Clone, Copy)]
pub struct CapabilityService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> CapabilityService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn require(self, plugin_id: &str, capability: &str) -> PluginResult<()> {
        let require = self
            .abi
            .require_capability
            .ok_or(PluginError::MissingHostFunction("require_capability"))?;
        let plugin_id = cstring_lossy(plugin_id);
        let capability = cstring_lossy(capability);
        let code = r#unsafe::require_capability(
            self.abi.host_context,
            require,
            plugin_id.as_c_str(),
            capability.as_c_str(),
        );
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "require_capability",
                code,
            })
        }
    }
}
