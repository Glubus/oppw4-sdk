mod value_probe;

use plugin_sdk::OwnedHostApi;

use crate::{config::ValueProbeConfig, exposure::RuntimeExposure};

pub(crate) struct ValueProbeExposure;

impl RuntimeExposure for ValueProbeExposure {
    type Config = ValueProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        value_probe::start(host, config);
    }
}
