mod easy_cap;
mod helper_probe;
mod threshold_patch;
mod threshold_probe;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{RankHelperProbeConfig, RankRuntimeConfig, RankThresholdProbeConfig},
    runtime::exposure::RuntimeExposure,
};

pub(crate) struct RankRuntimeExposure;
pub(crate) struct RankThresholdExposure;

impl RuntimeExposure for RankRuntimeExposure {
    type Config = RankRuntimeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        easy_cap::set_easy_s_rankable(&host, config.easy_s_rankable);
        threshold_patch::install(host, config);
    }
}

impl RuntimeExposure for RankThresholdExposure {
    type Config = RankThresholdProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        threshold_probe::start(host, config);
    }
}

pub(crate) fn install_helper(
    host: OwnedHostApi,
    config: RankHelperProbeConfig,
    runtime: RankRuntimeConfig,
) {
    helper_probe::install(host, config, runtime);
}
