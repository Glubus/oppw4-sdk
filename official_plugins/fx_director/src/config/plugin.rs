#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TriggerMode {
    Auto,
    Hotkey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InstallMode {
    ScanOnly,
    LocalPlayerProbe,
    Patch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum StatusGate {
    None,
    VirtualResource,
    DlcCharacter,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PluginDebugConfig {
    pub(crate) observe_effect_ids: bool,
    pub(crate) observe_character_probe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PluginConfig {
    pub(crate) install_mode: InstallMode,
    pub(crate) trigger: TriggerMode,
    pub(crate) hotkey_vk: i32,
    pub(crate) install_delay_ms: u64,
    pub(crate) wait_for: StatusGate,
    pub(crate) refresh_interval_ms: u64,
    pub(crate) debug: PluginDebugConfig,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            install_mode: InstallMode::Patch,
            trigger: TriggerMode::Auto,
            hotkey_vk: 0x77,
            install_delay_ms: 0,
            wait_for: StatusGate::None,
            refresh_interval_ms: 0,
            debug: PluginDebugConfig::default(),
        }
    }
}
