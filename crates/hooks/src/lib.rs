mod inline;
mod log;
mod memory;
mod signals;
mod signature;
mod status;
mod win;
mod winapi_file;

pub use inline::{HookBuilder, InlineHook};
pub use log::{diagnostics_enabled, set_diagnostics_enabled, set_logger};
pub use memory::{module_base, read_memory, scan_memory, write_memory};
pub use signals::{Signal, SignalBus, SignalHook, SignalId};
pub use signature::{Signature, SignatureScanner};
pub(crate) use status::mark_file_open;
pub use status::set_file_open_observer;
pub use winapi_file::{
    install_main_module_hooks, register_file_provider, FileProviderRegistration,
};
