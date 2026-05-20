use crate::{config::TargetMode, mods::FxInstallPlan};

use super::arena::CaveArena;

#[derive(Clone, Copy)]
pub(super) struct AuraDataPtrs {
    pub(super) enabled: usize,
    pub(super) force_effect_id: usize,
    pub(super) effect_id: usize,
    pub(super) local_player_filter: usize,
    pub(super) local_player: usize,
    pub(super) local_player_fx_owner: usize,
    pub(super) speed: usize,
    pub(super) timer: usize,
    pub(super) loop_start: usize,
    pub(super) loop_end: usize,
    pub(super) id_hits: usize,
    pub(super) id_forced_hits: usize,
    pub(super) duration_hits: usize,
    pub(super) duration_match_hits: usize,
    pub(super) observed_effect_id: usize,
    pub(super) observed_edx: usize,
    pub(super) observed_aura_param: usize,
    pub(super) observed_aura_owner: usize,
    pub(super) observed_effect: usize,
}

pub(super) fn build_shared_data(
    arena: &mut CaveArena,
    config: FxInstallPlan,
) -> Result<AuraDataPtrs, String> {
    let mut bytes = Vec::new();
    append_data(&mut bytes, config);
    let base = arena.alloc(&bytes, 8)?;
    Ok(data_ptrs(base, 0))
}

pub(super) fn write_u32(address: usize, value: u32) {
    unsafe { *(address as *mut u32) = value };
}

pub(super) fn read_u32(address: usize) -> u32 {
    unsafe { *(address as *const u32) }
}

pub(super) fn read_usize(address: usize) -> usize {
    unsafe { *(address as *const usize) }
}

pub(super) fn write_usize(address: usize, value: usize) {
    unsafe { *(address as *mut usize) = value };
}

pub(super) fn write_f32(address: usize, value: f32) {
    unsafe { *(address as *mut f32) = value };
}

fn append_data(code: &mut Vec<u8>, config: FxInstallPlan) -> usize {
    let offset = code.len();
    code.extend_from_slice(&(config.fx.enabled as u32).to_le_bytes());
    code.extend_from_slice(&(config.fx.force_effect_id as u32).to_le_bytes());
    code.extend_from_slice(&config.fx.effect_id.to_le_bytes());
    code.extend_from_slice(&((config.fx.target == TargetMode::LocalPlayer) as u32).to_le_bytes());
    code.extend_from_slice(&0u64.to_le_bytes());
    code.extend_from_slice(&0u64.to_le_bytes());
    code.extend_from_slice(&config.fx.animation_speed.to_bits().to_le_bytes());
    code.extend_from_slice(&0.0f32.to_bits().to_le_bytes());
    code.extend_from_slice(&config.fx.loop_start.to_bits().to_le_bytes());
    code.extend_from_slice(&config.fx.loop_end.to_bits().to_le_bytes());
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&0u32.to_le_bytes());
    code.extend_from_slice(&0u64.to_le_bytes());
    code.extend_from_slice(&0u64.to_le_bytes());
    code.extend_from_slice(&0u64.to_le_bytes());
    offset
}

fn data_ptrs(base: usize, offset: usize) -> AuraDataPtrs {
    AuraDataPtrs {
        enabled: base + offset,
        force_effect_id: base + offset + 4,
        effect_id: base + offset + 8,
        local_player_filter: base + offset + 12,
        local_player: base + offset + 16,
        local_player_fx_owner: base + offset + 24,
        speed: base + offset + 32,
        timer: base + offset + 36,
        loop_start: base + offset + 40,
        loop_end: base + offset + 44,
        id_hits: base + offset + 48,
        id_forced_hits: base + offset + 52,
        duration_hits: base + offset + 56,
        duration_match_hits: base + offset + 60,
        observed_effect_id: base + offset + 64,
        observed_edx: base + offset + 68,
        observed_aura_param: base + offset + 72,
        observed_aura_owner: base + offset + 80,
        observed_effect: base + offset + 88,
    }
}
