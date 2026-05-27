mod commit_hook;
mod item_hook;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{ItemRewardProbeConfig, RewardProbeConfig},
    runtime::exposure::RuntimeExposure,
};
pub(crate) struct RewardCommitExposure;
pub(crate) struct ItemRewardExposure;

impl RuntimeExposure for RewardCommitExposure {
    type Config = RewardProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        commit_hook::install(host, config);
    }
}

impl RuntimeExposure for ItemRewardExposure {
    type Config = ItemRewardProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        item_hook::install(host, config);
    }
}
