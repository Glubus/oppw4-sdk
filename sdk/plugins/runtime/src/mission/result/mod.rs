mod memory_probe;
mod player_probe;
mod state_hook;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{PlayerResultProbeConfig, ResultProbeConfig, ResultStateProbeConfig},
    runtime::exposure::RuntimeExposure,
};

pub(crate) struct PlayerResultExposure;
pub(crate) struct ResultMemoryExposure;
pub(crate) struct ResultStateExposure;

impl RuntimeExposure for PlayerResultExposure {
    type Config = PlayerResultProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        player_probe::start(host, config);
    }
}

impl RuntimeExposure for ResultMemoryExposure {
    type Config = ResultProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        memory_probe::start(host, config);
    }
}

impl RuntimeExposure for ResultStateExposure {
    type Config = ResultStateProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        state_hook::install(host, config);
    }
}

pub(crate) use state_hook::{latest_reward_context, ResultRewardContext};
