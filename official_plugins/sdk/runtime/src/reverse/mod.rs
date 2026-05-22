mod fixed_data_probe;
mod value_scan_probe;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{FixedDataProbeConfig, ValueProbeConfig},
    runtime::exposure::RuntimeExposure,
};

pub(crate) struct FixedDataExposure;

pub(crate) struct ValueScanExposure;

impl RuntimeExposure for FixedDataExposure {
    type Config = FixedDataProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        fixed_data_probe::start(host, config);
    }
}

impl RuntimeExposure for ValueScanExposure {
    type Config = ValueProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        value_scan_probe::start(host, config);
    }
}
