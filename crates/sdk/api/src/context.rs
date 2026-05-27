use std::path::PathBuf;

use crate::{
    validate_plugin_api, HostApi, LogPolicy, Oppw4ActiveCharacter, Oppw4GameStatus, Oppw4PluginApi,
    Plugin, PluginError, PluginModInfo, PluginRegistrar, PluginResult, VirtualFileProvider,
};

#[derive(Clone, Copy)]
pub struct PluginContext<'api> {
    plugin_id: &'static str,
    host: HostApi<'api>,
    log_policy: LogPolicy,
}

impl<'api> PluginContext<'api> {
    pub fn new<P: Plugin>(api: &'api Oppw4PluginApi) -> PluginResult<Self> {
        validate_plugin_api(api).map_err(PluginError::from)?;
        Ok(Self {
            plugin_id: P::ID,
            host: HostApi::new(api),
            log_policy: P::log_policy(),
        })
    }

    pub const fn host(self) -> HostApi<'api> {
        self.host
    }

    pub const fn abi(self) -> &'api Oppw4PluginApi {
        self.host.abi()
    }

    pub const fn api(self) -> &'api Oppw4PluginApi {
        self.abi()
    }

    pub const fn plugin_id(self) -> &'static str {
        self.plugin_id
    }

    pub const fn registrar(self) -> PluginRegistrar<'api> {
        PluginRegistrar::new(self)
    }

    pub fn game_root(self) -> Option<PathBuf> {
        self.host.paths().game_root()
    }

    pub fn plugin_root(self) -> Option<PathBuf> {
        self.host.paths().plugin_root()
    }

    pub fn mods_root(self) -> Option<PathBuf> {
        self.host.paths().mods_root()
    }

    pub fn log(self, message: impl AsRef<str>) {
        if !self.log_policy.host {
            return;
        }
        let _ = self.host.log().write(self.plugin_id, message);
    }

    pub fn game_status(self) -> Option<Oppw4GameStatus> {
        self.host.game().status().ok()
    }

    pub fn active_character(self) -> Option<Oppw4ActiveCharacter> {
        self.host.game().active_character().ok()
    }

    pub fn plugin_mods(self) -> Vec<PluginModInfo> {
        self.host.mods().plugin_mods()
    }

    pub fn register_virtual_file_provider(
        self,
        provider: VirtualFileProvider<'_>,
    ) -> PluginResult<()> {
        self.host.files().register_virtual_provider(provider)
    }

    pub fn register_registry_module(
        self,
        module_name: &str,
        module_context: *mut std::ffi::c_void,
        install: plugin_abi::Oppw4RegistryModuleInstallFn,
    ) -> PluginResult<()> {
        self.host.registry().register_module_descriptor(
            self.plugin_id,
            module_name,
            module_context,
            install,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_abi::null_api;

    struct TestPlugin;

    impl Plugin for TestPlugin {
        const ID: &'static str = "test_plugin";

        fn init(_context: PluginContext<'_>) -> PluginResult<()> {
            Ok(())
        }
    }

    #[test]
    fn context_validates_api_version() {
        let api = null_api();
        let context = PluginContext::new::<TestPlugin>(&api).expect("context");

        assert_eq!(context.plugin_id(), "test_plugin");
    }
}
