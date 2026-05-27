use std::sync::OnceLock;

use plugin_sdk::{HostApi, PluginLogger};

use crate::constants::PLUGIN_ID;

static LOGGER: OnceLock<PluginLogger> = OnceLock::new();

pub(crate) fn init(host: HostApi<'_>) {
    let _ = LOGGER.set(PluginLogger::new(PLUGIN_ID, host));
}

pub(crate) fn write(host: HostApi<'_>, message: impl AsRef<str>) {
    let _ = host.log().write(PLUGIN_ID, message);
}
