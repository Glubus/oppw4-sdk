use std::slice;

use super::snapshot::{CREW_UNLOCK_WORDS, EVENT_WORDS};

const MAX_EVENT_BYTES: usize = 0x4000;

pub(super) unsafe fn read_u32(base: *const u8, offset: usize) -> u32 {
    (base.add(offset) as *const u32).read_unaligned()
}

unsafe fn read_u64(base: *const u8, offset: usize) -> u64 {
    (base.add(offset) as *const u64).read_unaligned()
}

pub(super) unsafe fn read_u32_block<const N: usize>(base: *const u8, offset: usize) -> [u32; N] {
    let mut values = [0u32; N];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_u32(base, offset + index * 4);
    }
    values
}

pub(super) unsafe fn read_crew_unlocks(
    base: *const u8,
    count: usize,
) -> Vec<[u32; CREW_UNLOCK_WORDS]> {
    let mut unlocks = Vec::with_capacity(count);
    for index in 0..count {
        let offset = 0x820 + index * 8;
        unlocks.push([read_u32(base, offset), read_u32(base, offset + 4)]);
    }
    unlocks
}

pub(super) unsafe fn read_events(base: *const u8, limit: usize) -> Vec<[u32; EVENT_WORDS]> {
    let begin = read_u64(base, 0xb10) as usize;
    let end = read_u64(base, 0xb18) as usize;
    if begin == 0 || end < begin {
        return Vec::new();
    }

    let byte_len = end - begin;
    if byte_len > MAX_EVENT_BYTES {
        return Vec::new();
    }

    let entry_count = (byte_len / (EVENT_WORDS * 4)).min(limit);
    let words = slice::from_raw_parts(begin as *const u32, entry_count * EVENT_WORDS);
    words
        .chunks_exact(EVENT_WORDS)
        .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3], chunk[4]])
        .collect()
}
