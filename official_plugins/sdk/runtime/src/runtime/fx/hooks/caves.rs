use super::{
    arena::{CaveArena, CodeCave},
    data::AuraDataPtrs,
    AURA_DURATION_PATTERN,
};

pub(super) fn build_local_player_cave(
    arena: &mut CaveArena,
    return_address: usize,
    data: AuraDataPtrs,
) -> Result<CodeCave, String> {
    let mut code = Vec::new();
    code.extend_from_slice(&[0x48, 0x8b, 0x80, 0xd0, 0x02, 0x00, 0x00]);
    code.extend_from_slice(&[0x48, 0x85, 0xc0]);
    let jump_back_null = asm::emit_jz(&mut code);
    code.extend_from_slice(&[0x51, 0x52]);
    code.extend_from_slice(&[0x48, 0xb9]);
    code.extend_from_slice(&(data.local_player as u64).to_le_bytes());
    code.extend_from_slice(&[0x48, 0x89, 0x01]);
    code.extend_from_slice(&[0x48, 0x8d, 0x90, 0x60, 0x04, 0x00, 0x00]);
    code.extend_from_slice(&[0x48, 0xb9]);
    code.extend_from_slice(&(data.local_player_fx_owner as u64).to_le_bytes());
    code.extend_from_slice(&[0x48, 0x89, 0x11]);
    code.extend_from_slice(&[0x5a, 0x59]);
    let jump_back = asm::emit_jmp(&mut code);

    let base = arena.reserve(code.len(), 16)?;
    asm::patch_rel32_vec(&mut code, base, jump_back_null, return_address)?;
    asm::patch_rel32_vec(&mut code, base, jump_back, return_address)?;
    arena.write_at(base, &code);
    Ok(CodeCave { entry: base })
}

pub(super) fn build_duration_cave(
    arena: &mut CaveArena,
    return_address: usize,
    data: AuraDataPtrs,
) -> Result<CodeCave, String> {
    let mut code = Vec::new();
    let duration_hits_inc = asm::emit_inc_rip_u32(&mut code);
    let enabled_cmp = asm::emit_cmp_rip_u32(&mut code);
    let jump_original_disabled = asm::emit_jz(&mut code);
    let force_cmp = asm::emit_cmp_rip_u32(&mut code);
    let jump_original_not_forced = asm::emit_jz(&mut code);
    let effect_cmp = asm::emit_cmp_edx_rip(&mut code);
    let jump_original_mismatch = asm::emit_jne(&mut code);
    let duration_match_hits_inc = asm::emit_inc_rip_u32(&mut code);

    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0xe0, 1.0);
    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0xe4, 0.0);
    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0x2dc, 0.0);
    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0x2e8, 2.0);
    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0x2d0, 0.0);
    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0x2d4, 0.0);
    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0x2d8, 0.0);

    let timer_load = asm::emit_movss_xmm2_rip(&mut code);
    let speed_load = asm::emit_movss_xmm0_rip(&mut code);
    code.extend_from_slice(&[0xf3, 0x0f, 0x58, 0xd0]);
    let timer_store = asm::emit_movss_rip_xmm2(&mut code);
    code.extend_from_slice(&[0xf3, 0x0f, 0x11, 0x97, 0xe4, 0x02, 0x00, 0x00]);
    asm::emit_mov_dword_rdi_disp_imm(&mut code, 0x2ec, 1.0);
    let loop_end_load = asm::emit_movss_xmm0_rip(&mut code);
    code.extend_from_slice(&[0x0f, 0x2e, 0xd0]);
    let jump_original_not_done = asm::emit_jbe(&mut code);
    let loop_start_load = asm::emit_movss_xmm0_rip(&mut code);
    let timer_reset = asm::emit_movss_rip_xmm0(&mut code);

    let original_label = code.len();
    code.extend_from_slice(AURA_DURATION_PATTERN);
    let jump_back = asm::emit_jmp(&mut code);

    let base = arena.reserve(code.len(), 16)?;
    asm::patch_disp32_vec(&mut code, base, duration_hits_inc, data.duration_hits)?;
    asm::patch_disp32_vec(&mut code, base, enabled_cmp, data.enabled)?;
    asm::patch_rel32_vec(
        &mut code,
        base,
        jump_original_disabled,
        base + original_label,
    )?;
    asm::patch_disp32_vec(&mut code, base, force_cmp, data.force_effect_id)?;
    asm::patch_rel32_vec(
        &mut code,
        base,
        jump_original_not_forced,
        base + original_label,
    )?;
    asm::patch_disp32_vec(&mut code, base, effect_cmp, data.effect_id)?;
    asm::patch_rel32_vec(
        &mut code,
        base,
        jump_original_mismatch,
        base + original_label,
    )?;
    asm::patch_disp32_vec(
        &mut code,
        base,
        duration_match_hits_inc,
        data.duration_match_hits,
    )?;
    asm::patch_disp32_vec(&mut code, base, timer_load, data.timer)?;
    asm::patch_disp32_vec(&mut code, base, speed_load, data.speed)?;
    asm::patch_disp32_vec(&mut code, base, timer_store, data.timer)?;
    asm::patch_disp32_vec(&mut code, base, loop_end_load, data.loop_end)?;
    asm::patch_rel32_vec(
        &mut code,
        base,
        jump_original_not_done,
        base + original_label,
    )?;
    asm::patch_disp32_vec(&mut code, base, loop_start_load, data.loop_start)?;
    asm::patch_disp32_vec(&mut code, base, timer_reset, data.timer)?;
    asm::patch_rel32_vec(&mut code, base, jump_back, return_address)?;
    arena.write_at(base, &code);
    Ok(CodeCave { entry: base })
}
