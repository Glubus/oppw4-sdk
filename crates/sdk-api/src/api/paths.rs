use std::path::PathBuf;

use plugin_abi::Oppw4PluginApi;

use crate::helpers::path_from_cstr;

#[derive(Clone, Copy)]
pub struct PathService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> PathService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn game_root(self) -> Option<PathBuf> {
        path_from_cstr(self.abi.game_root_utf8)
    }

    pub fn plugin_root(self) -> Option<PathBuf> {
        path_from_cstr(self.abi.plugin_root_utf8)
    }

    pub fn mods_root(self) -> Option<PathBuf> {
        path_from_cstr(self.abi.mods_root_utf8)
    }
}
