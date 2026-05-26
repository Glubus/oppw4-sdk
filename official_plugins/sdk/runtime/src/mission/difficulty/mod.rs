mod control;
mod ids;
mod lua;
mod reward_row;
mod state_probe;
mod tables;

#[cfg(test)]
mod lua_tests;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::DifficultyProbeConfig,
    runtime::{exposure::RuntimeExposure, lua_module},
};

pub(crate) use ids::DifficultyId;

pub(crate) struct DifficultyExposure;

impl RuntimeExposure for DifficultyExposure {
    type Config = DifficultyProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        state_probe::start(host, config);
    }
}

lua_module::runtime_lua_module! {
    type = DifficultyLuaModule,
    module = lua::MODULE_NAME,
    factory = lua::module,
}

pub(crate) fn lua_module() -> DifficultyLuaModule {
    DifficultyLuaModule
}

pub(crate) fn install_control(host: OwnedHostApi) {
    control::install(host);
}
