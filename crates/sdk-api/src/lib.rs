pub mod linkdata;
pub mod manifest;
pub mod zip;

pub use manifest::{
    plugin_logs_root, plugin_mods_root, plugin_toml_path, sanitize_plugin_id, PluginDescriptor,
    PluginManifestError, DEFAULT_PLUGIN_VERSION, PLUGIN_LOGS_DIR, PLUGIN_MANIFEST_FILE,
    PLUGIN_MODS_DIR,
};
pub use plugin_abi::{
    HostActiveCharacterFn, HostGameStatusFn, HostPatchLinkDataRowFn, HostRdbPatchReadFn,
    HostRegisterRdbPatchProviderFn, HostRegisterRdbVirtualProviderFn, HostReplaceLinkDataEntryFn,
    Oppw4ActiveCharacter, Oppw4FileProvider, Oppw4GameStatus, Oppw4LinkDataEntryPatch,
    Oppw4LinkDataRowPatch, Oppw4LuaRegisterFn, Oppw4PluginApi, Oppw4ProviderCloseFn,
    Oppw4ProviderFileTimeFn, Oppw4ProviderOpenPathFn, Oppw4ProviderPatchReadFn,
    Oppw4ProviderReadFn, Oppw4ProviderSeekFn, Oppw4ProviderSizeFn, PluginModInfo,
    OPPW4_GAME_FLAG_DLC_CHARACTER_SEEN, OPPW4_GAME_FLAG_VIRTUAL_RESOURCE_SEEN,
    OPPW4_LINKDATA_ROW_OP_INSERT, OPPW4_LINKDATA_ROW_OP_REMOVE, OPPW4_LINKDATA_ROW_OP_REPLACE,
    OPPW4_PLUGIN_API_STRUCT_SIZE, OPPW4_PLUGIN_API_VERSION,
};

mod api;
mod context;
mod entry;
mod error;
mod helpers;
mod log;
mod plugin;
mod r#unsafe;

pub use api::{
    CapabilityService, ConfigService, FileService, GameService, HookService, HostApi,
    LinkDataRowTarget, LinkDataService, LogService, LuaService, MemoryService, ModService,
    OwnedHostApi, PathService, VirtualFileProvider,
};
pub use context::PluginContext;
pub use entry::{plugin_abi_from_raw, validate_plugin_api, PluginInitError};
pub use error::{PluginError, PluginResult};
pub use helpers::cstring_lossy;
pub use log::{mirror_mod_log_to_host, LogPolicy, PluginLogger};
pub use plugin::{init_plugin, Plugin};

#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        #[no_mangle]
        pub unsafe extern "system" fn oppw4_plugin_init(api: *const $crate::Oppw4PluginApi) -> i32 {
            unsafe { $crate::init_plugin::<$plugin>(api) }
        }
    };
}
