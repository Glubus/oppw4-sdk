mod active_character;
mod inline;
mod log;
mod memory;
mod signals;
mod signature;
mod status;
mod win;
mod winapi_file;

pub use active_character::{
    publish_local_player, snapshot as active_character_snapshot, ActiveCharacter,
    ACTIVE_CHARACTER_CHANGED,
};
pub use inline::{HookBuilder, InlineHook};
pub use log::{set_diagnostics_enabled, set_logger};
pub use memory::{module_base, read_memory, scan_memory, write_memory};
pub use signals::{Signal, SignalBus, SignalHook, SignalId};
pub use signature::{Signature, SignatureScanner};
pub use status::{game_status, mark_file_open, GameStatus};
pub use winapi_file::{
    install_main_module_hooks, register_file_provider, FileProviderRegistration,
};
