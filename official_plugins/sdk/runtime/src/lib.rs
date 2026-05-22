mod config;
mod game;
mod mission;
mod reverse;
mod rewards;
mod runtime;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct SdkRuntime;

impl Plugin for SdkRuntime {
    const ID: &'static str = "sdk_runtime";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        runtime::Runtime::initialize(context.host())?;
        context.log("sdk.runtime initialized");
        Ok(())
    }
}

export_plugin!(SdkRuntime);
