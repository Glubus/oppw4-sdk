use std::sync::OnceLock;

use plugin_sdk::{HostApi, PluginLogger};

static LOGGER: OnceLock<PluginLogger> = OnceLock::new();

pub(crate) fn init(host: HostApi<'_>) {
    let _ = LOGGER.set(PluginLogger::new("sdk_data", host));
}
