mod damage_formula_probe;
mod enemy_spawn_probe;
mod entity_counter_probe;
mod fixed_data_probe;
mod spawn_scaling_probe;
mod value_scan_probe;

use plugin_sdk::OwnedHostApi;

use crate::{
    config::{
        DamageFormulaProbeConfig, EnemySpawnProbeConfig, EnemyStatsProbeConfig,
        EntityCounterProbeConfig, FixedDataProbeConfig, SpawnScalingProbeConfig, ValueProbeConfig,
    },
    runtime::exposure::RuntimeExposure,
};

pub(crate) struct FixedDataExposure;

pub(crate) struct EntityCounterExposure;

pub(crate) struct DamageFormulaExposure;

pub(crate) struct EnemySpawnExposure;

pub(crate) struct SpawnScalingExposure;

pub(crate) struct ValueScanExposure;

impl RuntimeExposure for FixedDataExposure {
    type Config = FixedDataProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        fixed_data_probe::start(host, config);
    }
}

impl RuntimeExposure for EntityCounterExposure {
    type Config = EntityCounterProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        entity_counter_probe::start(host, config);
    }
}

impl RuntimeExposure for DamageFormulaExposure {
    type Config = (DamageFormulaProbeConfig, EnemyStatsProbeConfig);

    fn install(host: OwnedHostApi, config: Self::Config) {
        damage_formula_probe::install(host, config.0, config.1);
    }
}

impl RuntimeExposure for EnemySpawnExposure {
    type Config = EnemySpawnProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        enemy_spawn_probe::install(host, config);
    }
}

impl RuntimeExposure for SpawnScalingExposure {
    type Config = SpawnScalingProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        spawn_scaling_probe::start(host, config);
    }
}

impl RuntimeExposure for ValueScanExposure {
    type Config = ValueProbeConfig;

    fn install(host: OwnedHostApi, config: Self::Config) {
        value_scan_probe::start(host, config);
    }
}
