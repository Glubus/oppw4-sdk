mod probe;
mod reward_row;

use plugin_sdk::OwnedHostApi;

use crate::{config::DifficultyProbeConfig, exposure::RuntimeExposure};

pub(crate) struct DifficultyExposure;

impl RuntimeExposure for DifficultyExposure {
    type Config = DifficultyProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        probe::start(host, config);
    }
}
