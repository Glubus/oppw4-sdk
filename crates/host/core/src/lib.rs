#[cfg(windows)]
mod log;
#[cfg(windows)]
mod runtime;

#[cfg(windows)]
pub use log::set_logger;
#[cfg(windows)]
pub use runtime::{
    initialize, initialize_with_bridge_setup, set_debug_enabled, set_file_provider_registrar,
    set_memory,
};

#[cfg(not(windows))]
pub fn set_logger(_logger: fn(String)) {}

#[cfg(not(windows))]
pub fn set_debug_enabled(_enabled: bool) {}

#[cfg(not(windows))]
pub fn set_file_provider_registrar(
    _host_context: *mut std::ffi::c_void,
    _callback: plugin_abi::HostRegisterFileProviderFn,
) {
}

#[cfg(not(windows))]
pub fn set_memory(
    _host_context: *mut std::ffi::c_void,
    _module_base: plugin_abi::HostModuleBaseFn,
    _read: plugin_abi::HostReadMemoryFn,
    _write: plugin_abi::HostWriteMemoryFn,
    _scan: plugin_abi::HostScanMemoryFn,
) {
}

#[cfg(not(windows))]
pub fn initialize(
    _game_root: &std::path::Path,
    _plugin_root: &std::path::Path,
    _session_stamp: Option<String>,
) {
}

#[cfg(not(windows))]
pub fn initialize_with_bridge_setup(
    _game_root: &std::path::Path,
    _plugin_root: &std::path::Path,
    _session_stamp: Option<String>,
    _setup: impl FnOnce(&mut ()),
) {
}

#[cfg(test)]
mod tests {
    #[test]
    fn plugin_host_accepts_registry_method_mutation_contracts() {
        let schema = serde_json::json!({
            "namespace": "sdk",
            "import_name": "character",
            "constructible": false,
            "functions": [],
            "types": [
                {
                    "name": "CharacterSetTotalPayload",
                    "constructible": false,
                    "fields": [
                        {
                            "name": "target",
                            "type_ref": { "kind": "named", "name": "Character" }
                        },
                        {
                            "name": "value",
                            "type_ref": { "kind": "i64" }
                        }
                    ]
                }
            ],
            "extensions": [
                {
                    "target_type": "sdk.Character",
                    "methods": [
                        {
                            "name": "set_total",
                            "mutation": "set_total",
                            "returns": { "kind": "void" }
                        }
                    ]
                }
            ],
            "events": [],
            "mutations": [
                {
                    "name": "set_total",
                    "key": "sdk.character.set_total",
                    "payload": { "kind": "named", "name": "CharacterSetTotalPayload" }
                }
            ]
        });

        let parsed = serde_json::from_value::<sdk_bridge::RegistryModuleSchema>(schema)
            .expect("schema should deserialize");

        assert_eq!(parsed.extensions.len(), 1);
        assert_eq!(parsed.extensions[0].methods[0].name, "set_total");
        assert_eq!(parsed.extensions[0].methods[0].function, None);
        assert_eq!(
            parsed.extensions[0].methods[0].mutation.as_deref(),
            Some("set_total")
        );
    }
}
