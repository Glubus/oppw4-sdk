mod value_scan_probe;

use plugin_sdk::OwnedHostApi;

use crate::{config::ValueProbeConfig, runtime::exposure::RuntimeExposure};

pub(crate) struct ValueScanExposure;

impl RuntimeExposure for ValueScanExposure {
    type Config = ValueProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        value_scan_probe::start(host, config);
    }
}
