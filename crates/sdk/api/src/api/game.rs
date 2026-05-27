use std::ffi::c_void;

use plugin_abi::{
    HostActiveCharacterFn, HostGameStatusFn, Oppw4ActiveCharacter, Oppw4GameStatus, Oppw4PluginApi,
};

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

    /// # Safety
    ///
    /// `provider_context` and `callback` must remain valid while the plugin is
    /// loaded. The callback must write a valid `Oppw4GameStatus` when passed a
    /// non-null output pointer.
    pub unsafe fn register_status_provider(
        self,
        provider_context: *mut c_void,
        callback: HostGameStatusFn,
    ) -> PluginResult<()> {
        let register =
            self.abi
                .register_game_status_provider
                .ok_or(PluginError::MissingHostFunction(
                    "register_game_status_provider",
                ))?;
        let code = unsafe { register(self.abi.host_context, provider_context, Some(callback)) };
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "register_game_status_provider",
                code,
            })
        }
    }

    /// # Safety
    ///
    /// `provider_context` and `callback` must remain valid while the plugin is
    /// loaded. The callback must write a valid `Oppw4ActiveCharacter` when
    /// passed a non-null output pointer.
    pub unsafe fn register_active_character_provider(
        self,
        provider_context: *mut c_void,
        callback: HostActiveCharacterFn,
    ) -> PluginResult<()> {
        let register =
            self.abi
                .register_active_character_provider
                .ok_or(PluginError::MissingHostFunction(
                    "register_active_character_provider",
                ))?;
        let code = unsafe { register(self.abi.host_context, provider_context, Some(callback)) };
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "register_active_character_provider",
                code,
            })
        }
    }

    pub fn debug_enabled(self) -> bool {
        let Some(debug_enabled) = self.abi.debug_enabled else {
            return false;
        };
        r#unsafe::debug_enabled(self.abi.host_context, debug_enabled)
    }
}
