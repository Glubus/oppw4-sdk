mod discovery;
mod paths;
mod plugin;
mod state;

use std::{
    fs,
    path::Path,
    sync::{Mutex, OnceLock},
};

use crate::log;
use sdk_bridge::{
    BridgeLoadRequest, BridgeModSource, BridgeRegistry, EventEnvelope, EventKey, ModId,
    RegistryModuleDescriptor,
};
use sdk_mod_loader::{DiscoveredMod, ModSource};

use super::{logs, signals};
use plugin::LoadedPlugin;

static LOADED: OnceLock<Mutex<Vec<LoadedPlugin>>> = OnceLock::new();
static BRIDGES: OnceLock<Mutex<BridgeRegistry>> = OnceLock::new();

pub fn initialize(game_root: &Path, plugin_root: &Path, session_stamp: Option<String>) {
    initialize_with_bridge_setup(game_root, plugin_root, session_stamp, |_| {});
}

pub fn initialize_with_bridge_setup(
    game_root: &Path,
    plugin_root: &Path,
    session_stamp: Option<String>,
    setup: impl FnOnce(&mut BridgeRegistry),
) {
    prepare_runtime(game_root, plugin_root, session_stamp);
    log_sdk_status("initializing sdk");
    if let Some(registry) = BRIDGES.get() {
        setup(&mut registry.lock().expect("bridge registry lock"));
    }
    let report = discovery::load_plugins(game_root, plugin_root);
    log::write_line(format!(
        "plugin host: scanned={} manifests={} loaded={}",
        report.scanned, report.manifests, report.loaded
    ));
    log_sdk_status(format!(
        "sdk plugins loaded {}/{}",
        report.loaded, report.manifests
    ));
    load_mods(game_root);
}

fn prepare_runtime(game_root: &Path, plugin_root: &Path, session_stamp: Option<String>) {
    let _ = fs::create_dir_all(plugin_root);
    logs::initialize(
        session_stamp,
        paths::mods_root(game_root)
            .join("_oppw4")
            .join("logs")
            .join("mods"),
    );
    let _ = LOADED.set(Mutex::new(Vec::new()));
    let _ = BRIDGES.set(Mutex::new(BridgeRegistry::new()));
}

fn remember_loaded_plugin(plugin: LoadedPlugin) {
    if let Some(plugins) = LOADED.get() {
        plugins.lock().expect("plugin list lock").push(plugin);
    }
}

fn load_mods(game_root: &Path) {
    let mods_root = paths::mods_root(game_root);
    let mods = sdk_mod_loader::discover_mods(&mods_root);
    if mods.is_empty() {
        logs::write_mod(
            "plugin_host",
            &format!("mods scanned=0 root={}", mods_root.display()),
        );
        return;
    }
    let Some(registry) = BRIDGES.get() else {
        log::write_line("plugin host: bridge registry is not initialized");
        return;
    };
    let total = mods.len();
    let mut loaded = 0usize;
    log_sdk_status(format!("mods loaded {loaded}/{total}"));
    for mod_entry in mods {
        let mod_id = mod_entry.manifest.id.as_str().to_string();
        logs::write_mod(
            "plugin_host",
            &format!(
                "mod discovered id={} entry={} uses={:?}",
                mod_id, mod_entry.manifest.entry_file, mod_entry.manifest.uses_plugins
            ),
        );
        let request = match bridge_load_request(mod_entry) {
            Ok(request) => request,
            Err(error) => {
                let line = format!("mod load request failed id={mod_id} error={error:?}");
                log::write_line(format!("plugin host: {line}"));
                logs::write_mod("plugin_host", &line);
                continue;
            }
        };
        let (load_result, load_logs) = {
            let mut registry = registry.lock().expect("bridge registry lock");
            let result = registry.load_supported_mod(request);
            let logs = registry.drain_load_logs();
            (result, logs)
        };
        match load_result {
            Ok(lifecycle) => {
                loaded += 1;
                for line in load_logs {
                    log::write_line(format!("plugin host: mod log id={mod_id} {line}"));
                    logs::write_mod(&mod_id, &line);
                }
                let line = format!("mod loaded id={mod_id} lifecycle={lifecycle:?}");
                log::write_line(format!("plugin host: {line}"));
                logs::write_mod("plugin_host", &line);
                log_sdk_status(format!("mods loaded {loaded}/{total}"));
            }
            Err(error) => {
                for line in load_logs {
                    log::write_line(format!("plugin host: mod log id={mod_id} {line}"));
                    logs::write_mod(&mod_id, &line);
                }
                let line = format!("mod load failed id={mod_id} error={error:?}");
                log::write_line(format!("plugin host: {line}"));
                logs::write_mod("plugin_host", &line);
            }
        }
    }
    log::write_line(format!("plugin host: mods scanned={total} loaded={loaded}"));
    logs::write_mod(
        "plugin_host",
        &format!("mods scanned={total} loaded={loaded}"),
    );
    if let Some(registry) = BRIDGES.get() {
        let registry = registry.lock().expect("bridge registry lock");
        log_mod_conflicts(&registry);
    }
}

fn bridge_load_request(
    mod_entry: DiscoveredMod,
) -> Result<BridgeLoadRequest, sdk_bridge::BridgeError> {
    Ok(BridgeLoadRequest {
        mod_id: ModId::new(mod_entry.manifest.id)?,
        name: mod_entry.manifest.name,
        source: bridge_mod_source(mod_entry.source),
        entry_file: mod_entry.manifest.entry_file,
        uses_plugins: mod_entry.manifest.uses_plugins,
    })
}

fn bridge_mod_source(source: ModSource) -> BridgeModSource {
    match source {
        ModSource::Directory(path) => BridgeModSource::Directory(path),
        ModSource::Zip { path, root } => BridgeModSource::Zip { path, root },
    }
}

fn log_mod_conflicts(registry: &BridgeRegistry) {
    for conflict in registry.handler_conflicts() {
        let mods = conflict
            .mod_ids
            .iter()
            .map(|mod_id| mod_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let line = format!(
            "Attention! the mods: {mods} listen to the same event {} and could conflict if they modify the same runtime aspect",
            conflict.event_key.as_str()
        );
        log::write_line(format!("plugin host: {line}"));
        logs::write_mod("plugin_host", &line);
    }
    for conflict in registry.effect_conflicts() {
        let mods = conflict
            .mod_ids
            .iter()
            .map(|mod_id| mod_id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let line = format!(
            "Attention! the mods: {mods} both modify {}",
            conflict.effect.describe()
        );
        log::write_line(format!("plugin host: {line}"));
        logs::write_mod("plugin_host", &line);
    }
}

fn log_sdk_status(message: impl AsRef<str>) {
    let message = message.as_ref();
    log::write_line(format!("plugin host: sdk status {message}"));
    signals::emit_host_json(
        "sdk.host.status",
        serde_json::json!({
            "schema": "sdk.host.status.v1",
            "message": message,
        }),
    );
}

pub(crate) fn register_registry_module(module: RegistryModuleDescriptor) -> i32 {
    let Some(registry) = BRIDGES.get() else {
        return -30;
    };
    registry
        .lock()
        .expect("bridge registry lock")
        .register_module(module);
    0
}

pub(crate) fn dispatch_event(event: EventEnvelope) -> i32 {
    let Some(registry) = BRIDGES.get() else {
        return 0;
    };
    let mut registry = registry.lock().expect("bridge registry lock");
    let report = registry.dispatch_event(&event);
    for log_entry in report.mod_logs {
        log::write_line(format!(
            "plugin host: event log key={} {}",
            event.key.as_str(),
            log_entry.message
        ));
        logs::write_mod(log_entry.mod_id.as_str(), &log_entry.message);
    }
    for line in report.logs {
        log::write_line(format!(
            "plugin host: event log key={} {line}",
            event.key.as_str()
        ));
    }
    for error in report.errors {
        let line = format!(
            "event dispatch failed key={} mod={} bridge={} error={}",
            event.key.as_str(),
            error.mod_id.as_str(),
            error.bridge_id.as_str(),
            error.message
        );
        log::write_line(format!("plugin host: {line}"));
        logs::write_mod(error.mod_id.as_str(), &line);
    }
    if !report.mutations.is_empty() {
        log::write_line(format!(
            "plugin host: event mutations pending key={} count={}",
            event.key.as_str(),
            report.mutations.len()
        ));
    }
    if report.metrics.dispatch_us >= 1_000 {
        log::write_line(format!(
            "plugin host: event dispatch slow key={} handlers={} bridge_batches={} vm_batches={} payload_bytes={} dispatch_us={}",
            event.key.as_str(),
            report.metrics.handler_count,
            report.metrics.bridge_batch_count,
            report.metrics.vm_batch_count,
            report.metrics.payload_bytes,
            report.metrics.dispatch_us,
        ));
    }
    0
}

pub(crate) fn has_event_handlers(signal: &str) -> bool {
    let Ok(key) = EventKey::new(signal) else {
        return false;
    };
    let Some(registry) = BRIDGES.get() else {
        return false;
    };
    registry
        .lock()
        .map(|registry| registry.has_handlers(&key))
        .unwrap_or(true)
}
