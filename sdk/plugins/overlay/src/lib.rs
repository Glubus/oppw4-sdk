mod backend;
mod config;
mod panels;
mod runner;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

const PLUGIN_ID: &str = "sdk_overlay";

struct SdkOverlay;

impl Plugin for SdkOverlay {
    const ID: &'static str = PLUGIN_ID;

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let host = context.host();
        config::register_schema(host);
        panels::subscribe(host);
        let path = config::ensure_config(host);
        runner::start(host.owned(), path);
        context.log("sdk.overlay initialized");
        Ok(())
    }
}

export_plugin!(SdkOverlay);
