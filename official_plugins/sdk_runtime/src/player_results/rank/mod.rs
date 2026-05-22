mod threshold;

use plugin_sdk::OwnedHostApi;

use crate::{config::RankThresholdProbeConfig, exposure::RuntimeExposure};

pub(crate) struct RankThresholdExposure;

impl RuntimeExposure for RankThresholdExposure {
    type Config = RankThresholdProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        threshold::start(host, config);
    }
}
