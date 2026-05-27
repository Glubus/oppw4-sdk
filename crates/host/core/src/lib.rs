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
