pub(crate) mod exposure;
pub(crate) mod memory;
pub(crate) mod probe;
pub(crate) mod reader;
pub(crate) mod signals;

use plugin_sdk::{HostApi, OwnedHostApi, PluginResult};

use crate::{
    config,
    game::GameRuntime,
    mission::{
        difficulty::DifficultyExposure,
        rank::RankThresholdExposure,
        result::{PlayerResultExposure, ResultMemoryExposure, ResultStateExposure},
    },
    reverse::ValueScanExposure,
    rewards::{ItemRewardExposure, RewardCommitExposure},
    runtime::exposure::RuntimeExposure,
};

pub(crate) struct Runtime;

impl Runtime {
    pub(crate) fn initialize(host: HostApi<'_>) -> PluginResult<()> {
        config::register_schema(host);
        GameRuntime::register(host)?;
        let config = config::load(host);
        let owned_host = host.owned();

        GameRuntime::start();
        install_exposures(owned_host, config);
        Ok(())
    }
}

fn install_exposures(host: OwnedHostApi, config: config::RuntimeConfig) {
    DifficultyExposure::install(host.clone(), config.difficulty_probe);
    RewardCommitExposure::install(host.clone(), config.reward_probe);
    ItemRewardExposure::install(host.clone(), config.item_reward_probe);
    ResultStateExposure::install(host.clone(), config.result_state_probe);
    RankThresholdExposure::install(host.clone(), config.rank_threshold_probe);
    PlayerResultExposure::install(host.clone(), config.player_result_probe);
    ResultMemoryExposure::install(host.clone(), config.result_probe);
    ValueScanExposure::install(host, config.value_probe);
}
