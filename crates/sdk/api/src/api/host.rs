use std::sync::Arc;

use plugin_abi::Oppw4PluginApi;

use super::{
    CapabilityService, ConfigService, DifficultyService, FileService, GameService, HookService,
    LinkDataService, LogService, MemoryService, ModService, PathService, RankService, RdbService,
    RegistryService, SignalService,
};

#[derive(Clone, Copy)]
pub struct HostApi<'api> {
    abi: &'api Oppw4PluginApi,
}

#[derive(Clone)]
pub struct OwnedHostApi {
    abi: Arc<HostApiTable>,
}

struct HostApiTable {
    abi: Oppw4PluginApi,
}

// SAFETY: `HostApiTable` is a shared immutable callback table plus an opaque
// host context pointer. `OwnedHostApi` may be moved into worker threads, so every
// host callback reachable through this table must treat `host_context` as a
// thread-safe handle or reject the operation internally.
unsafe impl Send for HostApiTable {}

// SAFETY: SDK services only read callback pointers from the table and pass the
// opaque context back to the host. The host core owns synchronization for that
// context; plugins must not assume callback implementations are reentrant unless
// the service documentation says so.
unsafe impl Sync for HostApiTable {}

impl<'api> HostApi<'api> {
    pub const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub const fn abi(self) -> &'api Oppw4PluginApi {
        self.abi
    }

    pub const fn paths(self) -> PathService<'api> {
        PathService::new(self.abi)
    }

    pub const fn log(self) -> LogService<'api> {
        LogService::new(self.abi)
    }

    pub const fn memory(self) -> MemoryService<'api> {
        MemoryService::new(self.abi)
    }

    pub const fn capabilities(self) -> CapabilityService<'api> {
        CapabilityService::new(self.abi)
    }

    pub const fn configs(self) -> ConfigService<'api> {
        ConfigService::new(self.abi)
    }

    pub const fn difficulty(self) -> DifficultyService<'api> {
        DifficultyService::new(self.abi)
    }

    pub const fn hooks(self) -> HookService<'api> {
        HookService::new(self.abi)
    }

    pub const fn mods(self) -> ModService<'api> {
        ModService::new(self.abi)
    }

    pub const fn files(self) -> FileService<'api> {
        FileService::new(self.abi)
    }

    pub const fn registry(self) -> RegistryService<'api> {
        RegistryService::new(self.abi)
    }

    pub const fn game(self) -> GameService<'api> {
        GameService::new(self.abi)
    }

    pub const fn linkdata(self) -> LinkDataService<'api> {
        LinkDataService::new(self.abi)
    }

    pub const fn rdb(self) -> RdbService<'api> {
        RdbService::new(self.abi)
    }

    pub const fn rank(self) -> RankService<'api> {
        RankService::new(self.abi)
    }

    pub const fn signals(self) -> SignalService<'api> {
        SignalService::new(self.abi)
    }

    pub fn owned(self) -> OwnedHostApi {
        OwnedHostApi::new(*self.abi)
    }
}

impl OwnedHostApi {
    pub fn new(abi: Oppw4PluginApi) -> Self {
        Self {
            abi: Arc::new(HostApiTable { abi }),
        }
    }

    pub fn as_ref(&self) -> HostApi<'_> {
        HostApi::new(&self.abi.abi)
    }

    pub fn abi(&self) -> &Oppw4PluginApi {
        &self.abi.abi
    }

    pub fn paths(&self) -> PathService<'_> {
        self.as_ref().paths()
    }

    pub fn log(&self) -> LogService<'_> {
        self.as_ref().log()
    }

    pub fn memory(&self) -> MemoryService<'_> {
        self.as_ref().memory()
    }

    pub fn capabilities(&self) -> CapabilityService<'_> {
        self.as_ref().capabilities()
    }

    pub fn configs(&self) -> ConfigService<'_> {
        self.as_ref().configs()
    }

    pub fn difficulty(&self) -> DifficultyService<'_> {
        self.as_ref().difficulty()
    }

    pub fn hooks(&self) -> HookService<'_> {
        self.as_ref().hooks()
    }

    pub fn mods(&self) -> ModService<'_> {
        self.as_ref().mods()
    }

    pub fn files(&self) -> FileService<'_> {
        self.as_ref().files()
    }

    pub fn registry(&self) -> RegistryService<'_> {
        self.as_ref().registry()
    }

    pub fn game(&self) -> GameService<'_> {
        self.as_ref().game()
    }

    pub fn linkdata(&self) -> LinkDataService<'_> {
        self.as_ref().linkdata()
    }

    pub fn rdb(&self) -> RdbService<'_> {
        self.as_ref().rdb()
    }

    pub fn rank(&self) -> RankService<'_> {
        self.as_ref().rank()
    }

    pub fn signals(&self) -> SignalService<'_> {
        self.as_ref().signals()
    }
}

impl<'api> From<&'api Oppw4PluginApi> for HostApi<'api> {
    fn from(abi: &'api Oppw4PluginApi) -> Self {
        Self::new(abi)
    }
}

impl From<Oppw4PluginApi> for OwnedHostApi {
    fn from(abi: Oppw4PluginApi) -> Self {
        Self::new(abi)
    }
}

#[cfg(test)]
mod tests;
