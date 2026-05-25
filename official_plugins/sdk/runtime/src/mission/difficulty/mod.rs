mod ids;
mod reward_row;
mod state_probe;
mod tables;

use plugin_sdk::OwnedHostApi;

use crate::{config::DifficultyProbeConfig, runtime::exposure::RuntimeExposure};

pub(crate) use ids::DifficultyId;

pub(crate) struct DifficultyExposure;

impl RuntimeExposure for DifficultyExposure {
    type Config = DifficultyProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        state_probe::start(host, config);
    }
}
