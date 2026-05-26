mod commit_hook;
mod item_hook;
mod lua;

#[cfg(test)]
mod lua_tests;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{ItemRewardProbeConfig, RewardProbeConfig},
    runtime::{
        core::{
            bus::{RuntimeDispatchReport, RuntimeHandlerError},
            events::{RuntimeEvent, RuntimeMutation},
            live_bus,
            rewards::RewardCommitEvent,
        },
        exposure::RuntimeExposure,
        lua_module,
    },
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

#[allow(dead_code)]
pub(crate) fn register_reward_handler(
    id: impl Into<String>,
    handler: impl Fn(&RuntimeEvent) -> Result<Vec<RuntimeMutation>, RuntimeHandlerError>
        + Send
        + Sync
        + 'static,
) {
    live_bus::register_runtime_handler(id, handler);
}

pub(super) fn dispatch_reward_event(event: &RewardCommitEvent) -> RuntimeDispatchReport {
    live_bus::dispatch_runtime_event(event.clone().into())
}

#[cfg(test)]
pub(crate) fn reset_reward_handlers_for_tests() {
    live_bus::reset_runtime_handlers_for_tests();
}

lua_module::runtime_lua_module! {
    type = RewardsLuaModule,
    module = lua::MODULE_NAME,
    factory = lua::module,
}

pub(crate) fn lua_module(_host: OwnedHostApi) -> RewardsLuaModule {
    RewardsLuaModule
}
