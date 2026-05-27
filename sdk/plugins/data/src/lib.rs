use std::path::{Path, PathBuf};

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

mod log;

struct SdkData;

impl Plugin for SdkData {
    const ID: &'static str = "sdk_data";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        log::init(context.host());
        initialize_data(context);
        Ok(())
    }
}

fn initialize_data(context: PluginContext<'_>) {
    let Some(game_root) = context.game_root() else {
        struct_api::mark_data_unavailable();
        context.log("sdk.data unavailable: game root is missing");
        return;
    };
    let data_root = data_root(&game_root);
    match struct_api::initialize_data_root(&data_root) {
        Ok(()) => context.log(format!(
            "sdk.data loaded OPPW4 data from {}",
            data_root.display()
        )),
        Err(error) => {
            struct_api::mark_data_unavailable();
            context.log(format!(
                "sdk.data unavailable at {}: {error:?}",
                data_root.display()
            ));
        }
    }
}

fn data_root(game_root: &Path) -> PathBuf {
    game_root.join("oppw4-data")
}

export_plugin!(SdkData);
