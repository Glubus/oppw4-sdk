use plugin_abi::Oppw4PluginApi;

use crate::PluginResult;

const CAP_HOOKS_INSTALL: &str = "hooks.install";

#[derive(Clone, Copy)]
pub struct HookService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> HookService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn require_install(self, plugin_id: &str) -> PluginResult<()> {
        super::CapabilityService::new(self.abi).require(plugin_id, CAP_HOOKS_INSTALL)
    }
}
