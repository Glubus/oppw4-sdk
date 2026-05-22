mod player;
pub(crate) mod rank;
mod result;
mod result_state;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{PlayerResultProbeConfig, ResultProbeConfig, ResultStateProbeConfig},
    exposure::RuntimeExposure,
};

pub(crate) struct PlayerResultExposure;
pub(crate) struct ResultProbeExposure;
pub(crate) struct ResultStateExposure;

impl RuntimeExposure for PlayerResultExposure {
    type Config = PlayerResultProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        player::start(host, config);
    }
}

impl RuntimeExposure for ResultProbeExposure {
    type Config = ResultProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        result::start(host, config);
    }
}

impl RuntimeExposure for ResultStateExposure {
    type Config = ResultStateProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        result_state::install(host, config);
    }
}
