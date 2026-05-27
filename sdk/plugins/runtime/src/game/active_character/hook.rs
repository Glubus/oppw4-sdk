use std::sync::atomic::{AtomicUsize, Ordering};

use crate::runtime::memory::CaveArena;

const CAVE_ARENA_SIZE: usize = 0x400;
const LOCAL_PLAYER_PATTERN: &[u8] = &[0x48, 0x8b, 0x80, 0xd0, 0x02, 0x00, 0x00, 0xf3];
const LOCAL_PLAYER_MASK: &[u8] = &[1; 8];
const LOCAL_PLAYER_ORIGINAL_LEN: usize = 7;

static RAW_LOCAL_PLAYER: AtomicUsize = AtomicUsize::new(0);

pub(super) fn install_local_player_hook() {
    let address = unsafe {
        hooks::scan_memory(
            LOCAL_PLAYER_PATTERN.as_ptr(),
            LOCAL_PLAYER_MASK.as_ptr(),
            LOCAL_PLAYER_PATTERN.len(),
        )
    };
    if address == 0 {
        return;
    }

    let _ = unsafe {
        install_fixed_entry_hook(
            address,
            &LOCAL_PLAYER_PATTERN[..LOCAL_PLAYER_ORIGINAL_LEN],
            build_local_player_cave,
        )
    };
}

pub(super) fn raw_local_player() -> usize {
    RAW_LOCAL_PLAYER.load(Ordering::Acquire)
}

unsafe fn install_fixed_entry_hook(
    address: usize,
    original: &[u8],
    build_cave: fn(usize, &[u8]) -> Vec<u8>,
) -> Result<(), String> {
    let mut current = vec![0u8; original.len()];
    let read = unsafe { hooks::read_memory(address, current.as_mut_ptr(), current.len()) };
    if read != 0 {
        return Err(format!("read failed result={read} address=0x{address:x}"));
    }
    if current != original {
        return Err(format!(
            "unexpected bytes address=0x{address:x} expected={} got={}",
            hex(original),
            hex(&current)
        ));
    }

    let Some(mut arena) = (unsafe { CaveArena::new(address, CAVE_ARENA_SIZE) }) else {
        return Err("cave allocation failed".to_string());
    };

    let cave = build_cave(address + original.len(), original);
    let cave_address = unsafe { arena.alloc(&cave, 16) }?;
    let mut patch = vec![0x90; original.len()];
    asm::write_rel32_jump(&mut patch, address, 5, cave_address)?;

    let write = unsafe { hooks::write_memory(address, patch.as_ptr(), patch.len()) };
    if write != 0 {
        return Err(format!("write failed result={write} address=0x{address:x}"));
    }

    Ok(())
}

fn build_local_player_cave(return_address: usize, original: &[u8]) -> Vec<u8> {
    let mut code = Vec::new();
    code.extend_from_slice(original);
    code.extend_from_slice(&[0x51]);
    code.extend_from_slice(&[0x48, 0xb9]);
    code.extend_from_slice(&raw_local_player_address().to_le_bytes());
    code.extend_from_slice(&[0x48, 0x89, 0x01]);
    code.extend_from_slice(&[0x59]);
    asm::emit_abs_jmp(&mut code, return_address);
    code
}

fn raw_local_player_address() -> u64 {
    (&RAW_LOCAL_PLAYER as *const AtomicUsize as usize) as u64
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_player_cave_preserves_original_instruction_first() {
        let original = &LOCAL_PLAYER_PATTERN[..LOCAL_PLAYER_ORIGINAL_LEN];
        let code = build_local_player_cave(0x1234, original);

        assert!(code.starts_with(original));
        assert!(code.windows(3).any(|window| window == [0x48, 0x89, 0x01]));
    }

    #[test]
    fn local_player_cave_does_not_call_back_into_rust() {
        let original = &LOCAL_PLAYER_PATTERN[..LOCAL_PLAYER_ORIGINAL_LEN];
        let code = build_local_player_cave(0x1234, original);

        assert!(!code.windows(2).any(|window| window == [0xff, 0xd0]));
        assert!(code.windows(2).any(|window| window == [0x51, 0x48]));
        assert!(code.windows(2).any(|window| window == [0x01, 0x59]));
    }
}
