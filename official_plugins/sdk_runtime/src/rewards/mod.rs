mod commit;
mod item;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{ItemRewardProbeConfig, RewardProbeConfig},
    exposure::RuntimeExposure,
};

pub(crate) struct RewardCommitExposure;
pub(crate) struct ItemRewardExposure;

impl RuntimeExposure for RewardCommitExposure {
    type Config = RewardProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        commit::install(host, config);
    }
}

impl RuntimeExposure for ItemRewardExposure {
    type Config = ItemRewardProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        item::install(host, config);
    }
}
