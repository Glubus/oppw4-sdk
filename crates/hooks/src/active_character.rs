use std::sync::atomic::{AtomicU16, AtomicU64, AtomicUsize, Ordering};

use crate::{memory, SignalId};

pub const ACTIVE_CHARACTER_CHANGED: SignalId = SignalId::new("active_character_changed");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActiveCharacter {
    pub runtime_id: u16,
    pub alt_id: u16,
    pub flags: u32,
    pub local_player: usize,
    pub fx_owner: usize,
    pub source: usize,
    pub sequence: u64,
}

static RUNTIME_ID: AtomicU16 = AtomicU16::new(u16::MAX);
static ALT_ID: AtomicU16 = AtomicU16::new(u16::MAX);
static LOCAL_PLAYER: AtomicUsize = AtomicUsize::new(0);
static FX_OWNER: AtomicUsize = AtomicUsize::new(0);
static SOURCE: AtomicUsize = AtomicUsize::new(0);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn publish_local_player(local_player: usize) {
    let fx_owner = local_player.checked_add(0x460).unwrap_or(0);
    let source = read_usize(local_player + 0x118).unwrap_or(0);
    let runtime_id = read_u16(source + 0x2).unwrap_or(u16::MAX);
    let alt_id = read_u16(source).unwrap_or(u16::MAX);

    LOCAL_PLAYER.store(local_player, Ordering::Relaxed);
    FX_OWNER.store(fx_owner, Ordering::Relaxed);
    SOURCE.store(source, Ordering::Relaxed);
    ALT_ID.store(alt_id, Ordering::Relaxed);
    RUNTIME_ID.store(runtime_id, Ordering::Release);
    SEQUENCE.fetch_add(1, Ordering::AcqRel);
}

pub fn snapshot() -> ActiveCharacter {
    ActiveCharacter {
        runtime_id: RUNTIME_ID.load(Ordering::Acquire),
        alt_id: ALT_ID.load(Ordering::Relaxed),
        flags: 0,
        local_player: LOCAL_PLAYER.load(Ordering::Relaxed),
        fx_owner: FX_OWNER.load(Ordering::Relaxed),
        source: SOURCE.load(Ordering::Relaxed),
        sequence: SEQUENCE.load(Ordering::Relaxed),
    }
}

fn read_u16(address: usize) -> Option<u16> {
    let mut value = 0u16;
    let result = unsafe {
        memory::read_memory(
            address,
            (&mut value as *mut u16).cast(),
            std::mem::size_of::<u16>(),
        )
    };
    (result == 0).then_some(value)
}

fn read_usize(address: usize) -> Option<usize> {
    let mut value = 0usize;
    let result = unsafe {
        memory::read_memory(
            address,
            (&mut value as *mut usize).cast(),
            std::mem::size_of::<usize>(),
        )
    };
    (result == 0).then_some(value)
}
