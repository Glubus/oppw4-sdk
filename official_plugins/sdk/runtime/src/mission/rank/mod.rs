mod threshold_probe;

use plugin_sdk::OwnedHostApi;

use crate::{config::RankThresholdProbeConfig, runtime::exposure::RuntimeExposure};

pub(crate) struct RankThresholdExposure;

impl RuntimeExposure for RankThresholdExposure {
    type Config = RankThresholdProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        threshold_probe::start(host, config);
    }
}
