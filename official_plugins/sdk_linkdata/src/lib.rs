mod linkdata;
mod log;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct SdkLinkData;

impl Plugin for SdkLinkData {
    const ID: &'static str = "sdk_linkdata";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        log::initialize(context.host().owned());
        linkdata::initialize(context.host().owned())?;
        context.log("sdk.linkdata initialized");
        Ok(())
    }
}

export_plugin!(SdkLinkData);
