pub mod linkdata;
pub mod manifest;
pub mod zip;

pub use manifest::{
    parse_mod_manifest, plugin_logs_root, plugin_mods_root, plugin_toml_path, sanitize_plugin_id,
    ModEntry, ModManifest, PluginDescriptor, PluginManifestError, RegistryModuleDescriptor,
    DEFAULT_PLUGIN_VERSION, MOD_MANIFEST_FILE, PLUGIN_LOGS_DIR, PLUGIN_MANIFEST_FILE,
    PLUGIN_MODS_DIR,
};
pub use plugin_abi::{
    HostActiveCharacterFn, HostGameStatusFn, HostPatchLinkDataRowFn, HostRdbPatchReadFn,
    HostRegisterRdbPatchProviderFn, HostRegisterRdbVirtualProviderFn, HostReplaceLinkDataEntryFn,
    Oppw4ActiveCharacter, Oppw4FileProvider, Oppw4GameStatus, Oppw4LinkDataEntryPatch,
    Oppw4LinkDataRowPatch, Oppw4PluginApi, Oppw4ProviderCloseFn, Oppw4ProviderFileTimeFn,
    Oppw4ProviderOpenPathFn, Oppw4ProviderPatchReadFn, Oppw4ProviderReadFn, Oppw4ProviderSeekFn,
    Oppw4ProviderSizeFn, Oppw4RegistryModuleInstallFn, PluginModInfo,
    OPPW4_GAME_FLAG_DLC_CHARACTER_SEEN, OPPW4_GAME_FLAG_VIRTUAL_RESOURCE_SEEN,
    OPPW4_LINKDATA_ROW_OP_INSERT, OPPW4_LINKDATA_ROW_OP_REMOVE, OPPW4_LINKDATA_ROW_OP_REPLACE,
    OPPW4_PLUGIN_API_STRUCT_SIZE, OPPW4_PLUGIN_API_VERSION,
};

mod api;
mod context;
mod entry;
mod error;
mod feature;
mod helpers;
mod log;
mod plugin;
mod r#unsafe;

pub use api::DifficultyService;
pub use api::{
    CapabilityService, ConfigService, FileService, GameService, HookService, HostApi,
    LinkDataRowTarget, LinkDataService, LogService, MemoryService, ModService, OwnedHostApi,
    PathService, RankService, RdbService, RegistryService, VirtualFileProvider, CAP_CONFIG_SCHEMA,
    CAP_FILES_VIRTUALIZE, CAP_HOOKS_INSTALL, CAP_LINKDATA_PATCH, CAP_MEMORY_READ, CAP_MEMORY_SCAN,
    CAP_MEMORY_WRITE, CAP_MOD_DISCOVERY, CAP_PLUGIN_HOST, CAP_RDB_PATCH, CAP_REGISTRY_MODULE,
    CAP_REGISTRY_RUNTIME, CAP_SIGNALS_EMIT, CAP_SIGNALS_SUBSCRIBE,
};
pub use context::PluginContext;
pub use entry::{plugin_abi_from_raw, validate_plugin_api, PluginInitError};
pub use error::{PluginError, PluginResult};
pub use feature::{
    ConfigFeature, PluginFeature, PluginRegistrar, RdbPatchCallbackFeature, RegistryModuleFeature,
    RegistryModuleFeatureBuilder, VirtualFileProviderFeature,
};
pub use helpers::cstring_lossy;
pub use log::{mirror_mod_log_to_host, LogPolicy, PluginLogger};
pub use plugin::{init_plugin, Plugin, RustMod, RustModPlugin};

#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        #[no_mangle]
        pub unsafe extern "system" fn oppw4_plugin_init(api: *const $crate::Oppw4PluginApi) -> i32 {
            unsafe { $crate::init_plugin::<$plugin>(api) }
        }
    };
}

#[macro_export]
macro_rules! export_rust_mod {
    ($mod:ty) => {
        $crate::export_plugin!($crate::RustModPlugin::<$mod>);
    };
}

/// Experimental convenience macro for tiny plugins.
///
/// Prefer implementing [`Plugin`] directly when the plugin needs custom
/// initialization, validation, logging policy, runtime hooks, or non-trivial
/// error handling.
#[macro_export]
macro_rules! sdk_plugin {
    (
        id = $id:literal,
        name = $name:literal,
        features = [$( $feature:expr ),* $(,)?] $(,)?
    ) => {
        pub const PLUGIN_ID: &str = $id;

        struct SdkPlugin;

        impl $crate::Plugin for SdkPlugin {
            const ID: &'static str = PLUGIN_ID;

            fn init(context: $crate::PluginContext<'_>) -> $crate::PluginResult<()> {
                #[allow(unused_mut)]
                let mut registrar = context.registrar();
                $(
                    registrar.add($feature)?;
                )*
                registrar.finish()
            }
        }

        $crate::export_plugin!(SdkPlugin);
    };
}

#[macro_export]
macro_rules! registry_module {
    (
        plugin = $plugin:expr,
        module = $module:expr,
        install = $install:path $(,)?
    ) => {
        $crate::RegistryModuleFeature::new($plugin, $module, $install)
    };
}

#[macro_export]
macro_rules! config_schema {
    (
        id = $id:expr,
        name = $name:expr,
        toml = $toml:expr $(,)?
    ) => {
        $crate::ConfigFeature::new($id, $name, $toml)
    };
    (
        name = $name:expr,
        toml = $toml:expr $(,)?
    ) => {
        $crate::ConfigFeature::new($name, $name, $toml)
    };
}

#[macro_export]
macro_rules! rdb_patch_feature {
    (
        id = $id:expr,
        patch_read = $patch_read:path $(,)?
    ) => {
        $crate::RdbPatchCallbackFeature::new($id, $patch_read)
    };
}
