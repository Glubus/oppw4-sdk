use std::sync::OnceLock;

use plugin_sdk::{HostApi, PluginLogger};

const PLUGIN_ID: &str = "skin_patcher";

static LOGGER: OnceLock<PluginLogger> = OnceLock::new();

pub fn initialize(host: HostApi<'_>) {
    let _ = LOGGER.set(PluginLogger::new(PLUGIN_ID, host));
}

pub fn write_line(message: impl AsRef<str>) {
    if let Some(logger) = LOGGER.get() {
        logger.write_line(message);
    }
}
