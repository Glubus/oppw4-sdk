mod active_character;
mod config;
mod difficulty_probe;
mod memory;
mod status;

use std::ptr;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct SdkRuntime;

impl Plugin for SdkRuntime {
    const ID: &'static str = "sdk_runtime";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        config::register_schema(context.host());
        let game = context.host().game();
        unsafe {
            game.register_status_provider(ptr::null_mut(), status::read_game_status)?;
            game.register_active_character_provider(
                ptr::null_mut(),
                active_character::read_active_character,
            )?;
        }
        let config = config::load(context.host());
        active_character::start_probe();
        difficulty_probe::start(context.host().owned(), config.difficulty_probe);
        context.log("sdk.runtime initialized");
        Ok(())
    }
}

export_plugin!(SdkRuntime);
