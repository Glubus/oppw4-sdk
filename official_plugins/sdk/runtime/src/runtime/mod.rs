pub(crate) mod exposure;
pub(crate) mod fx;
pub(crate) mod lua_module;
pub(crate) mod memory;
pub(crate) mod probe;
pub(crate) mod reader;
pub(crate) mod signals;

use plugin_sdk::{HostApi, OwnedHostApi, PluginResult};

use crate::{
    config,
    game::GameRuntime,
    mission::{
        difficulty::{self, DifficultyExposure},
        rank::{self, RankRuntimeExposure, RankThresholdExposure},
        result::{PlayerResultExposure, ResultMemoryExposure, ResultStateExposure},
    },
    reverse::{
        DamageFormulaExposure, EntityCounterExposure, FixedDataExposure, SpawnScalingExposure,
        ValueScanExposure,
    },
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
        difficulty::install_control(owned_host.clone());
        rank::install_control(owned_host.clone());
        lua_module::register(host, difficulty::lua_module());
        lua_module::register(host, rank::lua_module());
        install_exposures(owned_host, config);
        fx::initialize(host)?;
        Ok(())
    }
}

fn install_exposures(host: OwnedHostApi, config: config::RuntimeConfig) {
    DifficultyExposure::install(host.clone(), config.difficulty_probe);
    RewardCommitExposure::install(host.clone(), config.reward_probe);
    ItemRewardExposure::install(host.clone(), config.item_reward_probe);
    ResultStateExposure::install(host.clone(), config.result_state_probe);
    RankRuntimeExposure::install(host.clone(), config.rank_runtime.clone());
    RankThresholdExposure::install(host.clone(), config.rank_threshold_probe);
    rank::install_helper(host.clone(), config.rank_helper_probe, config.rank_runtime);
    PlayerResultExposure::install(host.clone(), config.player_result_probe);
    ResultMemoryExposure::install(host.clone(), config.result_probe);
    EntityCounterExposure::install(host.clone(), config.entity_counter_probe);
    FixedDataExposure::install(host.clone(), config.fixed_data_probe);
    DamageFormulaExposure::install(host.clone(), config.damage_formula_probe);
    SpawnScalingExposure::install(host.clone(), config.spawn_scaling_probe);
    ValueScanExposure::install(host, config.value_probe);
}
