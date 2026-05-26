pub(crate) mod active_character;
mod status;

use std::ptr;

use plugin_sdk::{HostApi, PluginResult};

pub(crate) struct GameRuntime;

impl GameRuntime {
    pub(crate) fn register(host: HostApi<'_>) -> PluginResult<()> {
        status::initialize();
        let game = host.game();
        unsafe {
            game.register_status_provider(ptr::null_mut(), status::read_game_status)?;
            game.register_active_character_provider(
                ptr::null_mut(),
                active_character::read_active_character,
            )?;
        }
        Ok(())
    }

    pub(crate) fn start() {
        active_character::start_probe();
    }
}
