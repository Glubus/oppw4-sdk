use std::{
    mem,
    sync::atomic::{AtomicU16, AtomicU64, AtomicUsize, Ordering},
};

use crate::runtime::core::player::{self, PlayerSnapshot};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct ActiveCharacter {
    pub(super) runtime_id: u16,
    pub(super) alt_id: u16,
    pub(super) flags: u32,
    pub(super) local_player: usize,
    pub(super) fx_owner: usize,
    pub(super) source: usize,
    pub(super) sequence: u64,
}

static RUNTIME_ID: AtomicU16 = AtomicU16::new(u16::MAX);
static ALT_ID: AtomicU16 = AtomicU16::new(u16::MAX);
static LOCAL_PLAYER: AtomicUsize = AtomicUsize::new(0);
static FX_OWNER: AtomicUsize = AtomicUsize::new(0);
static SOURCE: AtomicUsize = AtomicUsize::new(0);
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn publish_local_player(local_player: usize) {
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
    publish_player_core_snapshot(runtime_id);
}

pub(super) fn snapshot() -> ActiveCharacter {
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

fn publish_player_core_snapshot(runtime_id: u16) {
    if runtime_id == u16::MAX {
        return;
    }
    let snapshot = PlayerSnapshot::new().with_active_character(format!("runtime:{runtime_id}"));
    player::update_snapshot(snapshot);
}

fn read_u16(address: usize) -> Option<u16> {
    read_value(address)
}

fn read_usize(address: usize) -> Option<usize> {
    read_value(address)
}

fn read_value<T: Copy>(address: usize) -> Option<T> {
    let mut value = mem::MaybeUninit::<T>::uninit();
    let result = unsafe {
        hooks::read_memory(
            address,
            value.as_mut_ptr().cast::<u8>(),
            mem::size_of::<T>(),
        )
    };
    (result == 0).then(|| unsafe { value.assume_init() })
}
