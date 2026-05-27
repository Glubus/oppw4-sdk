mod config;
mod format;
mod memory;
mod model;
mod runner;
mod script;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

const PLUGIN_ID: &str = "sdk_debug";

struct SdkDebug;

impl Plugin for SdkDebug {
    const ID: &'static str = PLUGIN_ID;

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let host = context.host();
        config::register_schema(host);
        let path = config::ensure_debug_script(host);
        runner::start(host.owned(), path);
        context.log("sdk.debug initialized");
        Ok(())
    }
}

export_plugin!(SdkDebug);
