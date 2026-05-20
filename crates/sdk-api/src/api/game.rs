use plugin_abi::{Oppw4ActiveCharacter, Oppw4GameStatus, Oppw4PluginApi};

use crate::{api::r#unsafe, error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct GameService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> GameService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn status(self) -> PluginResult<Oppw4GameStatus> {
        let status = self
            .abi
            .game_status
            .ok_or(PluginError::MissingHostFunction("game_status"))?;
        r#unsafe::game_status(self.abi.host_context, status)
    }

    pub fn active_character(self) -> PluginResult<Oppw4ActiveCharacter> {
        let active_character = self
            .abi
            .active_character
            .ok_or(PluginError::MissingHostFunction("active_character"))?;
        r#unsafe::active_character(self.abi.host_context, active_character)
    }

    pub fn debug_enabled(self) -> bool {
        let Some(debug_enabled) = self.abi.debug_enabled else {
            return false;
        };
        r#unsafe::debug_enabled(self.abi.host_context, debug_enabled)
    }
}
