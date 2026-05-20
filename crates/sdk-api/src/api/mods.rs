use plugin_abi::Oppw4PluginApi;

use crate::{api::r#unsafe, PluginModInfo};

#[derive(Clone, Copy)]
pub struct ModService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> ModService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn legacy_paths(self) -> Vec<String> {
        let Some(for_each) = self.abi.for_each_plugin_mod_zip else {
            return Vec::new();
        };
        r#unsafe::legacy_mod_paths(self.abi.host_context, for_each)
    }

    pub fn plugin_mods(self) -> Vec<PluginModInfo> {
        let Some(for_each) = self.abi.for_each_plugin_mod else {
            return Vec::new();
        };
        r#unsafe::plugin_mods(self.abi.host_context, for_each)
    }
}
