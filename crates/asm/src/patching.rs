#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rel32Patch {
    pub instruction_offset: usize,
    pub immediate_offset: usize,
    pub instruction_len: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disp32Patch {
    pub offset: usize,
}

impl From<usize> for Disp32Patch {
    fn from(offset: usize) -> Self {
        Self { offset }
    }
}

pub fn patch_disp32_vec<P>(
    code: &mut [u8],
    base: usize,
    patch: P,
    target: usize,
) -> Result<(), String>
where
    P: Into<Disp32Patch>,
{
    let patch = patch.into();
    let instruction_end = checked_addr_add(base, patch.offset, "rip instruction address")?;
    let instruction_end = checked_addr_add(instruction_end, 4, "rip instruction end")?;
    let disp = checked_i32_delta(target, instruction_end, "rip displacement")?;
    write_i32(code, patch.offset, disp)
}

pub fn patch_rel32_vec(
    code: &mut [u8],
    base: usize,
    patch: Rel32Patch,
    target: usize,
) -> Result<(), String> {
    let source = checked_addr_add(base, patch.instruction_offset, "jump instruction address")?;
    let rel = rel32(source, patch.instruction_len, target)?;
    write_i32(code, patch.immediate_offset, rel)
}

pub fn rel32(source: usize, instruction_len: usize, target: usize) -> Result<i32, String> {
    let instruction_end = checked_addr_add(source, instruction_len, "jump instruction end")?;
    checked_i32_delta(
        target,
        instruction_end,
        &format!("jump target out of range source=0x{source:x} target=0x{target:x}"),
    )
}

pub fn write_rel32_jump(
    code: &mut [u8],
    source: usize,
    instruction_len: usize,
    target: usize,
) -> Result<(), String> {
    if code.len() < instruction_len || instruction_len < 5 {
        return Err(format!("jump patch too small len=0x{:x}", code.len()));
    }
    let rel = rel32(source, instruction_len, target)?;
    code.fill(0x90);
    code[0] = 0xe9;
    code[1..5].copy_from_slice(&rel.to_le_bytes());
    Ok(())
}

fn checked_addr_add(left: usize, right: usize, label: &str) -> Result<usize, String> {
    left.checked_add(right)
        .ok_or_else(|| format!("{label}: address overflow left=0x{left:x} right=0x{right:x}"))
}

fn checked_i32_delta(target: usize, source_end: usize, label: &str) -> Result<i32, String> {
    let delta = target as i128 - source_end as i128;
    i32::try_from(delta).map_err(|_| format!("{label}: value out of i32 range: {delta}"))
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

    #[test]
    fn rejects_rel32_out_of_range() {
        let error = rel32(0x1000, 5, usize::MAX).unwrap_err();
        assert!(error.contains("out of i32 range"));
    }

    #[test]
    fn rejects_address_overflow() {
        let mut code = [0u8; 8];
        let error = patch_disp32_vec(&mut code, usize::MAX, 2, 0x1010).unwrap_err();
        assert!(error.contains("address overflow"));
    }

    #[test]
    fn rejects_disp32_out_of_bounds() {
        let mut code = [0u8; 4];
        let error = patch_disp32_vec(&mut code, 0x1000, 2, 0x1010).unwrap_err();
        assert!(error.contains("i32 patch out of range"));
    }
}
