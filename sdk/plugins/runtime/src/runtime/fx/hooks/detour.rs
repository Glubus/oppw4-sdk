use std::{
    mem, ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};

use plugin_sdk::HostApi;

use super::{
    arena::{CaveArena, InlineHook},
    data::{read_u32, read_usize, write_u32, write_usize, AuraDataPtrs},
};

const AURA_UPDATE_ENTRY_OFFSET_FROM_ID_SITE: usize = 0xc6;
const AURA_UPDATE_ENTRY_OVERWRITE_LEN: usize = 20;
const AURA_UPDATE_ENTRY_PREFIX: &[u8] = &[
    0x4c, 0x8b, 0xdc, 0x45, 0x89, 0x43, 0x18, 0x55, 0x56, 0x49, 0x8d, 0x6b, 0xa1, 0x48, 0x81, 0xec,
    0xf8, 0x00, 0x00, 0x00,
];

static AURA_UPDATE_ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static AURA_UPDATE_DATA: OnceLock<AuraDataPtrs> = OnceLock::new();

pub(super) fn install_aura_update_hook(
    api: HostApi<'_>,
    arena: &mut CaveArena,
    id_site: usize,
    data: AuraDataPtrs,
) -> Result<(usize, InlineHook), String> {
    let function_entry = id_site
        .checked_sub(AURA_UPDATE_ENTRY_OFFSET_FROM_ID_SITE)
        .ok_or_else(|| "fx director id site before expected function entry".to_string())?;
    verify_aura_update_entry(api, function_entry)?;
    let _ = AURA_UPDATE_DATA.set(data);
    let hook = unsafe {
        InlineHook::install(
            api,
            arena,
            function_entry,
            aura_update_detour as *const () as usize,
            AURA_UPDATE_ENTRY_OVERWRITE_LEN,
        )?
    };
    AURA_UPDATE_ORIGINAL.store(hook.trampoline, Ordering::Release);
    Ok((function_entry, hook))
}

fn verify_aura_update_entry(api: HostApi<'_>, function_entry: usize) -> Result<(), String> {
    let mut bytes = vec![0u8; AURA_UPDATE_ENTRY_PREFIX.len()];
    api.memory()
        .read(function_entry, &mut bytes)
        .map_err(|error| {
            format!("read_memory failed function_entry=0x{function_entry:x}: {error}")
        })?;
    if bytes != AURA_UPDATE_ENTRY_PREFIX {
        return Err(format!(
            "fx update function prefix mismatch entry=0x{function_entry:x}"
        ));
    }
    Ok(())
}

type AuraUpdateFn = unsafe extern "system" fn(usize, usize, u32);

unsafe extern "system" fn aura_update_detour(param_1: usize, param_2: usize, param_3: u32) {
    let original = AURA_UPDATE_ORIGINAL.load(Ordering::Acquire);
    if original == 0 {
        return;
    }
    let original: AuraUpdateFn = mem::transmute(original);
    let Some(data) = AURA_UPDATE_DATA.get().copied() else {
        original(param_1, param_2, param_3);
        return;
    };

    let enabled = read_u32(data.enabled);
    let local_player_filter = read_u32(data.local_player_filter);
    let local_player_fx_owner = read_usize(data.local_player_fx_owner);
    if should_skip_effect_inspection(enabled, local_player_filter, local_player_fx_owner) {
        original(param_1, param_2, param_3);
        return;
    }

    let Some((owner, effect_field)) = aura_target(param_1) else {
        original(param_1, param_2, param_3);
        return;
    };

    let natural_effect_id = ptr::read_unaligned(effect_field);
    write_u32(data.id_hits, read_u32(data.id_hits).wrapping_add(1));
    write_u32(data.observed_effect_id, natural_effect_id);
    write_u32(data.observed_edx, param_3);
    write_usize(data.observed_aura_param, param_1);
    write_usize(data.observed_aura_owner, owner);
    write_usize(data.observed_effect, effect_field as usize - 0x54);

    if read_u32(data.force_effect_id) == 0 {
        original(param_1, param_2, param_3);
        return;
    }

    if local_player_filter != 0 && owner != local_player_fx_owner {
        original(param_1, param_2, param_3);
        return;
    }

    let forced_effect_id = read_u32(data.effect_id);
    write_u32(
        data.id_forced_hits,
        read_u32(data.id_forced_hits).wrapping_add(1),
    );
    ptr::write_unaligned(effect_field, forced_effect_id);
    original(param_1, param_2, param_3);
    ptr::write_unaligned(effect_field, natural_effect_id);
}

fn should_skip_effect_inspection(
    enabled: u32,
    local_player_filter: u32,
    local_player_fx_owner: usize,
) -> bool {
    enabled == 0 || (local_player_filter != 0 && local_player_fx_owner == 0)
}

unsafe fn aura_target(param_1: usize) -> Option<(usize, *mut u32)> {
    if param_1 == 0 {
        return None;
    }
    let owner = ptr::read_unaligned((param_1 + 0x2d8) as *const usize);
    if owner == 0 {
        return None;
    }
    let effect = ptr::read_unaligned((owner + 0x10) as *const usize);
    (effect != 0).then_some((owner, (effect + 0x54) as *mut u32))
}

#[cfg(test)]
mod tests {
    use super::should_skip_effect_inspection;

    #[test]
    fn skips_when_disabled() {
        assert!(should_skip_effect_inspection(0, 0, 0));
    }

    #[test]
    fn skips_local_filter_until_owner_is_known() {
        assert!(should_skip_effect_inspection(1, 1, 0));
        assert!(!should_skip_effect_inspection(1, 1, 0x1234));
    }

    #[test]
    fn allows_global_effects_when_enabled() {
        assert!(!should_skip_effect_inspection(1, 0, 0));
    }
}
