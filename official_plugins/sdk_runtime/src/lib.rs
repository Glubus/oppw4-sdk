mod active_character;
mod config;
mod difficulty_probe;
mod difficulty_reward_row;
mod item_reward_probe;
mod memory;
mod player_result_probe;
mod rank_threshold_probe;
mod result_probe;
mod result_state_probe;
mod reward_probe;
mod status;
mod value_probe;

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
        reward_probe::install(context.host().owned(), config.reward_probe);
        item_reward_probe::install(context.host().owned(), config.item_reward_probe);
        result_state_probe::install(context.host().owned(), config.result_state_probe);
        difficulty_probe::start(context.host().owned(), config.difficulty_probe);
        rank_threshold_probe::start(context.host().owned(), config.rank_threshold_probe);
        player_result_probe::start(context.host().owned(), config.player_result_probe);
        result_probe::start(context.host().owned(), config.result_probe);
        value_probe::start(context.host().owned(), config.value_probe);
        context.log("sdk.runtime initialized");
        Ok(())
    }
}

export_plugin!(SdkRuntime);
