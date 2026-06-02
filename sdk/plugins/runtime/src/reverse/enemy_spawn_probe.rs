use plugin_sdk::OwnedHostApi;

use crate::{config::EnemySpawnProbeConfig, runtime::probe::PLUGIN_ID};

pub(crate) fn install(host: OwnedHostApi, config: EnemySpawnProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "enemy_spawn_probe disabled by config");
        return;
    }

    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "enemy_spawn_probe pending: Ghidra signature required for FUN_1415d1320 or a confirmed spawn request site max_logs={}",
            config.max_logs,
        ),
    );
}
