use std::sync::{Mutex, OnceLock};

use plugin_sdk::OwnedHostApi;

use crate::runtime::fx::mods::{FxInstallPlan, SharedFxState};

mod arena;
mod bootstrap;
mod caves;
mod data;
mod detour;
mod diagnostics;
mod install;
mod reload;

use arena::InlineHook;
use data::{write_f32, write_u32, AuraDataPtrs};
use install as installer;

const WEAPON_AURA_ID_PATTERN: &[u8] = &[0x44, 0x8b, 0x7b, 0x54, 0x41, 0x8b, 0x50, 0x04];
const WEAPON_AURA_ID_MASK: &[u8] = &[1; 8];
const AURA_DURATION_PATTERN: &[u8] = &[0xf3, 0x0f, 0x10, 0x9f, 0xe8, 0x02, 0x00, 0x00];
const AURA_DURATION_MASK: &[u8] = &[1; 8];
const LOCAL_PLAYER_PATTERN: &[u8] = &[0x48, 0x8b, 0x80, 0xd0, 0x02, 0x00, 0x00, 0xf3];
const LOCAL_PLAYER_MASK: &[u8] = &[1; 8];

static INSTALL: OnceLock<Mutex<Option<InstallState>>> = OnceLock::new();

#[derive(Clone)]
struct WorkerApi {
    api: OwnedHostApi,
    state: Option<SharedFxState>,
}

unsafe impl Send for WorkerApi {}

impl WorkerApi {
    fn new(api: OwnedHostApi, state: SharedFxState) -> Self {
        Self {
            api,
            state: Some(state),
        }
    }

    fn install_now(&self, config: FxInstallPlan) -> Result<(), String> {
        install_now(self.clone(), config)
    }

    fn load_config(&self) -> Option<FxInstallPlan> {
        self.state.as_ref().and_then(|state| state.install_plan())
    }

    fn active_character(&self) -> Option<plugin_sdk::Oppw4ActiveCharacter> {
        self.api.game().active_character().ok()
    }

    fn debug_enabled(&self) -> bool {
        self.api.game().debug_enabled()
    }
}

struct InstallState {
    pub(super) data: AuraDataPtrs,
    pub(super) _aura_update_hook: Option<InlineHook>,
}

fn install_now(worker_api: WorkerApi, config: FxInstallPlan) -> Result<(), String> {
    let api = worker_api.api.clone();
    if let Some(state) = installer::install_now(api.as_ref(), worker_api, config)? {
        let install = INSTALL.get_or_init(|| Mutex::new(None));
        *install.lock().map_err(|_| "install lock poisoned")? = Some(state);
    }
    Ok(())
}

pub(crate) fn install_deferred(api: OwnedHostApi, state: SharedFxState) -> i32 {
    bootstrap::install_deferred(api, state)
}

pub(crate) fn set_enabled(enabled: bool) -> i32 {
    with_state(|state| {
        write_u32(state.data.enabled, enabled as u32);
    })
}

pub(crate) fn set_effect_id(effect_id: u32) -> i32 {
    with_state(|state| {
        write_u32(state.data.effect_id, effect_id);
    })
}

pub(crate) fn set_timing(animation_speed: f32, loop_start: f32, loop_end: f32) -> i32 {
    with_state(|state| {
        write_f32(state.data.speed, animation_speed);
        write_f32(state.data.loop_start, loop_start);
        write_f32(state.data.loop_end, loop_end);
    })
}

fn with_state(action: impl FnOnce(&InstallState)) -> i32 {
    let Some(install) = INSTALL.get() else {
        return -1;
    };
    let Ok(guard) = install.lock() else {
        return -2;
    };
    let Some(state) = guard.as_ref() else {
        return -3;
    };
    action(state);
    0
}
