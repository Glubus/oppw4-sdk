use std::sync::{Arc, Mutex};

use plugin_sdk::HostApi;

use crate::runtime::fx::config::{load_plugin_config, CycleConfig, FxConfig, PluginConfig};

pub(crate) type SharedFxState = Arc<Mutex<FxRuntimeState>>;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct FxInstallPlan {
    pub(crate) plugin: PluginConfig,
    pub(crate) fx: FxConfig,
    pub(crate) cycle: CycleConfig,
}

#[derive(Clone, Debug)]
pub(crate) struct FxRuntimeState {
    pub(crate) plugin: PluginConfig,
    pub(crate) cycle: CycleConfig,
    pub(crate) current: Option<FxConfig>,
}

impl FxRuntimeState {
    fn new(plugin: PluginConfig) -> Self {
        Self {
            plugin,
            cycle: CycleConfig::default(),
            current: Some(FxConfig::default()),
        }
    }

    pub(crate) fn install_plan(&self) -> Option<FxInstallPlan> {
        self.current.map(|fx| FxInstallPlan {
            plugin: self.plugin,
            fx,
            cycle: self.cycle,
        })
    }

    pub(crate) fn plugin_config(&self) -> PluginConfig {
        self.plugin
    }
}

pub(crate) fn load_config(host: HostApi<'_>) -> SharedFxState {
    Arc::new(Mutex::new(FxRuntimeState::new(load_plugin_config(host))))
}
