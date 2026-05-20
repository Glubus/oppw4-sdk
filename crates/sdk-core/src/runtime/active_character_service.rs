use std::{
    ffi::c_void,
    ptr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
    thread,
    time::Duration,
};

use crate::log;

const CAVE_ARENA_SIZE: usize = 0x400;
const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const ALLOCATION_GRANULARITY: usize = 0x10000;
const NEAR_ALLOC_SEARCH_RANGE: usize = 0x0800_0000;

const LOCAL_PLAYER_PATTERN: &[u8] = &[0x48, 0x8b, 0x80, 0xd0, 0x02, 0x00, 0x00, 0xf3];
const LOCAL_PLAYER_MASK: &[u8] = &[1; 8];
const LOCAL_PLAYER_ORIGINAL_LEN: usize = 7;

static STARTED: OnceLock<()> = OnceLock::new();
static RAW_LOCAL_PLAYER: AtomicUsize = AtomicUsize::new(0);

extern "system" {
    fn VirtualAlloc(
        address: *mut c_void,
        size: usize,
        allocation_type: u32,
        protect: u32,
    ) -> *mut c_void;
}

pub(crate) fn start() {
    if STARTED.set(()).is_err() {
        return;
    }

    let _ = thread::Builder::new()
        .name("oppw4_sdk_active_character".to_string())
        .spawn(|| {
            log::write_line("sdk active character service started");
            install_local_player_hook();
            poll_active_character();
        });
}

fn install_local_player_hook() {
    let address = unsafe {
        hooks::scan_memory(
            LOCAL_PLAYER_PATTERN.as_ptr(),
            LOCAL_PLAYER_MASK.as_ptr(),
            LOCAL_PLAYER_PATTERN.len(),
        )
    };
    if address == 0 {
        log::write_line("sdk active character hook skipped: signature not found");
        return;
    }

    let result = unsafe {
        install_fixed_entry_hook(
            address,
            &LOCAL_PLAYER_PATTERN[..LOCAL_PLAYER_ORIGINAL_LEN],
            build_local_player_cave,
        )
    };

    match result {
        Ok(()) => log::write_line(format!(
            "sdk active character hook installed site=0x{address:x}"
        )),
        Err(error) => log::write_line(format!(
            "sdk active character hook skipped site=0x{address:x}: {error}"
        )),
    }
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

fn poll_active_character() {
    let mut last_local_player = 0usize;
    loop {
        thread::sleep(Duration::from_millis(100));
        let local_player = RAW_LOCAL_PLAYER.load(Ordering::Acquire);
        if local_player == 0 || local_player == last_local_player {
            continue;
        }
        last_local_player = local_player;
        hooks::publish_local_player(local_player);
    }
}

struct CaveArena {
    base: usize,
    cursor: usize,
    size: usize,
}

impl CaveArena {
    unsafe fn new(hint: usize, size: usize) -> Option<Self> {
        allocate_near_executable_block(hint, size).map(|base| Self {
            base,
            cursor: 0,
            size,
        })
    }

    unsafe fn alloc(&mut self, bytes: &[u8], alignment: usize) -> Result<usize, String> {
        self.cursor = align_up(self.cursor, alignment.max(1));
        let end = self
            .cursor
            .checked_add(bytes.len())
            .ok_or_else(|| "active character cave arena overflow".to_string())?;
        if end > self.size {
            return Err("active character cave arena full".to_string());
        }

        let address = self.base + self.cursor;
        unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len()) };
        self.cursor = end;
        Ok(address)
    }
}

unsafe fn allocate_near_executable_block(hint: usize, len: usize) -> Option<usize> {
    allocate_near(hint, len)
        .or_else(|| allocate_anywhere(len))
        .map(|memory| memory as usize)
}

unsafe fn allocate_near(hint: usize, len: usize) -> Option<*mut c_void> {
    let center = align_down(hint, ALLOCATION_GRANULARITY);
    let mut distance = 0usize;
    while distance <= NEAR_ALLOC_SEARCH_RANGE {
        for address in [
            center.saturating_add(distance),
            center.saturating_sub(distance),
        ] {
            let memory = unsafe {
                VirtualAlloc(
                    address as *mut c_void,
                    len,
                    MEM_COMMIT | MEM_RESERVE,
                    PAGE_EXECUTE_READWRITE,
                )
            };
            if !memory.is_null() {
                return Some(memory);
            }
        }
        distance = distance.saturating_add(ALLOCATION_GRANULARITY);
    }
    None
}

unsafe fn allocate_anywhere(len: usize) -> Option<*mut c_void> {
    let memory = unsafe {
        VirtualAlloc(
            ptr::null_mut(),
            len,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        )
    };
    if memory.is_null() {
        None
    } else {
        Some(memory)
    }
}

fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
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
