mod character;
mod config;
mod difficulty;
mod exposure;
mod memory;
mod player_results;
mod reverse;
mod rewards;

use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

use exposure::RuntimeExposure;

struct SdkRuntime;

impl Plugin for SdkRuntime {
    const ID: &'static str = "sdk_runtime";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        let host = context.host();
        config::register_schema(host);
        character::CharacterRuntime::register(host)?;
        let config = config::load(host);
        let owned_host = host.owned();

        character::CharacterRuntime::start();
        difficulty::DifficultyExposure::install(owned_host.clone(), config.difficulty_probe);
        rewards::RewardCommitExposure::install(owned_host.clone(), config.reward_probe);
        rewards::ItemRewardExposure::install(owned_host.clone(), config.item_reward_probe);
        player_results::ResultStateExposure::install(owned_host.clone(), config.result_state_probe);
        player_results::rank::RankThresholdExposure::install(
            owned_host.clone(),
            config.rank_threshold_probe,
        );
        player_results::PlayerResultExposure::install(
            owned_host.clone(),
            config.player_result_probe,
        );
        player_results::ResultProbeExposure::install(owned_host.clone(), config.result_probe);
        reverse::ValueProbeExposure::install(owned_host, config.value_probe);
        context.log("sdk.runtime initialized");
        Ok(())
    }
}

export_plugin!(SdkRuntime);
