mod commit_hook;
mod control;
mod item_hook;
mod lua;
mod rules;

#[cfg(test)]
mod lua_tests;

use plugin_sdk::OwnedHostApi;
use std::sync::Arc;

use crate::{
    config::{ItemRewardProbeConfig, RewardProbeConfig},
    runtime::{exposure::RuntimeExposure, lua_module},
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

pub(crate) fn install_control(host: OwnedHostApi) {
    control::install(host);
}

lua_module::runtime_lua_module! {
    type = RewardsLuaModule,
    module = lua::MODULE_NAME,
    context = Arc<OwnedHostApi>,
    factory = lua::module_with_host,
}

pub(crate) fn lua_module(host: OwnedHostApi) -> RewardsLuaModule {
    RewardsLuaModule::new(Arc::new(host))
}
