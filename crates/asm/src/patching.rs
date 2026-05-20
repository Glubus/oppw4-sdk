#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rel32Patch {
    pub instruction_offset: usize,
    pub immediate_offset: usize,
    pub instruction_len: usize,
}

pub fn patch_disp32_vec(
    code: &mut [u8],
    base: usize,
    disp_offset: usize,
    target: usize,
) -> Result<(), String> {
    let instruction_end = base + disp_offset + 4;
    let disp = checked_i32(
        target as isize - instruction_end as isize,
        "rip displacement",
    )?;
    write_i32(code, disp_offset, disp)
}

pub fn patch_rel32_vec(
    code: &mut [u8],
    base: usize,
    patch: Rel32Patch,
    target: usize,
) -> Result<(), String> {
    let rel = rel32(
        base + patch.instruction_offset,
        patch.instruction_len,
        target,
    )?;
    write_i32(code, patch.immediate_offset, rel)
}

pub fn rel32(source: usize, instruction_len: usize, target: usize) -> Result<i32, String> {
    let rel = target as isize - (source + instruction_len) as isize;
    checked_i32(
        rel,
        &format!("jump target out of range source=0x{source:x} target=0x{target:x}"),
    )
}

pub fn write_rel32_jump(
    code: &mut [u8],
    source: usize,
    instruction_len: usize,
    target: usize,
) -> Result<(), String> {
    let rel = rel32(source, instruction_len, target)?;
    if code.len() < instruction_len || instruction_len < 5 {
        return Err(format!("jump patch too small len=0x{:x}", code.len()));
    }
    code.fill(0x90);
    code[0] = 0xe9;
    code[1..5].copy_from_slice(&rel.to_le_bytes());
    Ok(())
}

fn checked_i32(value: isize, label: &str) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("{label}: value out of i32 range: {value}"))
}

fn write_i32(code: &mut [u8], offset: usize, value: i32) -> Result<(), String> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| "i32 patch offset overflow".to_string())?;
    let Some(slot) = code.get_mut(offset..end) else {
        return Err(format!(
            "i32 patch out of range offset=0x{offset:x} len=0x{:x}",
            code.len()
        ));
    };
    slot.copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_rel32_jump_with_nops() {
        let mut code = [0u8; 7];
        write_rel32_jump(&mut code, 0x1000, 5, 0x1010).unwrap();
        assert_eq!(code[0], 0xe9);
        assert_eq!(i32::from_le_bytes(code[1..5].try_into().unwrap()), 0x0b);
        assert_eq!(&code[5..], &[0x90, 0x90]);
    }

    #[test]
    fn patches_rip_disp_from_instruction_end() {
        let mut code = [0u8; 8];
        patch_disp32_vec(&mut code, 0x1000, 2, 0x1010).unwrap();
        assert_eq!(i32::from_le_bytes(code[2..6].try_into().unwrap()), 0x0a);
    }
}
