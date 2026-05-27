use std::{ffi::c_void, sync::OnceLock};

use plugin_abi::{HostActiveCharacterFn, HostGameStatusFn, Oppw4ActiveCharacter, Oppw4GameStatus};

use crate::runtime::debug;

mod r#unsafe;

pub(crate) use r#unsafe::{
    host_active_character, host_debug_enabled, host_game_status,
    host_register_active_character_provider, host_register_game_status_provider,
};

static GAME_STATUS_PROVIDER: OnceLock<GameStatusProvider> = OnceLock::new();
static ACTIVE_CHARACTER_PROVIDER: OnceLock<ActiveCharacterProvider> = OnceLock::new();

#[derive(Clone, Copy)]
struct GameStatusProvider {
    context: usize,
    callback: HostGameStatusFn,
}

#[derive(Clone, Copy)]
struct ActiveCharacterProvider {
    context: usize,
    callback: HostActiveCharacterFn,
}

fn register_game_status_provider(
    provider_context: *mut c_void,
    callback: Option<HostGameStatusFn>,
) -> i32 {
    let Some(callback) = callback else {
        return -1;
    };
    GAME_STATUS_PROVIDER
        .set(GameStatusProvider {
            context: provider_context as usize,
            callback,
        })
        .map(|_| 0)
        .unwrap_or(-2)
}

fn register_active_character_provider(
    provider_context: *mut c_void,
    callback: Option<HostActiveCharacterFn>,
) -> i32 {
    let Some(callback) = callback else {
        return -1;
    };
    ACTIVE_CHARACTER_PROVIDER
        .set(ActiveCharacterProvider {
            context: provider_context as usize,
            callback,
        })
        .map(|_| 0)
        .unwrap_or(-2)
}

fn read_game_status(out_status: &mut Oppw4GameStatus) -> i32 {
    let Some(provider) = GAME_STATUS_PROVIDER.get() else {
        return -40;
    };
    unsafe { (provider.callback)(provider.context as *mut c_void, out_status) }
}

fn read_active_character(out: &mut Oppw4ActiveCharacter) -> i32 {
    let Some(provider) = ACTIVE_CHARACTER_PROVIDER.get() else {
        return -40;
    };
    unsafe { (provider.callback)(provider.context as *mut c_void, out) }
}

fn debug_enabled() -> i32 {
    debug::enabled() as i32
}
