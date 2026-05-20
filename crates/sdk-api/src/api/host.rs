use std::sync::Arc;

use plugin_abi::Oppw4PluginApi;

use super::{
    CapabilityService, FileService, GameService, HookService, LinkDataService, LogService,
    LuaService, MemoryService, ModService, PathService,
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
// host context pointer. The ABI contract requires host services to remain valid
// when plugins move the owned API handle into worker threads.
unsafe impl Send for HostApiTable {}

// SAFETY: SDK services only read callback pointers from the table and pass the
// opaque context back to the host. The host owns synchronization for that
// context.
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

    pub const fn hooks(self) -> HookService<'api> {
        HookService::new(self.abi)
    }

    pub const fn mods(self) -> ModService<'api> {
        ModService::new(self.abi)
    }

    pub const fn files(self) -> FileService<'api> {
        FileService::new(self.abi)
    }

    pub const fn lua(self) -> LuaService<'api> {
        LuaService::new(self.abi)
    }

    pub const fn game(self) -> GameService<'api> {
        GameService::new(self.abi)
    }

    pub const fn linkdata(self) -> LinkDataService<'api> {
        LinkDataService::new(self.abi)
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

    pub fn hooks(&self) -> HookService<'_> {
        self.as_ref().hooks()
    }

    pub fn mods(&self) -> ModService<'_> {
        self.as_ref().mods()
    }

    pub fn files(&self) -> FileService<'_> {
        self.as_ref().files()
    }

    pub fn lua(&self) -> LuaService<'_> {
        self.as_ref().lua()
    }

    pub fn game(&self) -> GameService<'_> {
        self.as_ref().game()
    }

    pub fn linkdata(&self) -> LinkDataService<'_> {
        self.as_ref().linkdata()
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
