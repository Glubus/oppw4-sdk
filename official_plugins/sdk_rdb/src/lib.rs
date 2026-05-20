use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct SdkRdb;

impl Plugin for SdkRdb {
    const ID: &'static str = "sdk.rdb";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        context.log("sdk.rdb initialized");
        Ok(())
    }
}

export_plugin!(SdkRdb);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_stable_plugin_id() {
        assert_eq!(SdkRdb::ID, "sdk.rdb");
    }
}
