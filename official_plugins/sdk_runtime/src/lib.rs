mod active_character;
mod memory;
mod status;

use std::ptr;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct SdkRuntime;

impl Plugin for SdkRuntime {
    const ID: &'static str = "sdk_runtime";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let game = context.host().game();
        unsafe {
            game.register_status_provider(ptr::null_mut(), status::read_game_status)?;
            game.register_active_character_provider(
                ptr::null_mut(),
                active_character::read_active_character,
            )?;
        }
        active_character::start_probe();
        context.log("sdk.runtime initialized");
        Ok(())
    }
}

export_plugin!(SdkRuntime);
