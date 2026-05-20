use std::{ffi::c_void, ptr};

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const ALLOCATION_GRANULARITY: usize = 0x10000;
const NEAR_ALLOC_SEARCH_RANGE: usize = 0x0800_0000;

extern "system" {
    fn VirtualAlloc(
        address: *mut c_void,
        size: usize,
        allocation_type: u32,
        protect: u32,
    ) -> *mut c_void;
}

pub(crate) unsafe fn allocate_near_executable_block(hint: usize, len: usize) -> Option<usize> {
    allocate_near(hint, len)
        .or_else(|| allocate_anywhere(len))
        .map(|memory| memory as usize)
}

pub(crate) unsafe fn write_bytes(address: usize, bytes: &[u8]) {
    ptr::copy_nonoverlapping(bytes.as_ptr(), address as *mut u8, bytes.len());
}

unsafe fn allocate_near(hint: usize, len: usize) -> Option<*mut c_void> {
    let center = align_down(hint, ALLOCATION_GRANULARITY);
    let mut distance = 0usize;
    while distance <= NEAR_ALLOC_SEARCH_RANGE {
        for address in candidate_addresses(center, distance) {
            let memory = VirtualAlloc(
                address as *mut c_void,
                len,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE,
            );
            if !memory.is_null() {
                return Some(memory);
            }
        }
        distance = distance.saturating_add(ALLOCATION_GRANULARITY);
    }
    None
}

unsafe fn allocate_anywhere(len: usize) -> Option<*mut c_void> {
    let memory = VirtualAlloc(
        ptr::null_mut(),
        len,
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE,
    );
    if memory.is_null() {
        None
    } else {
        Some(memory)
    }
}

fn candidate_addresses(center: usize, distance: usize) -> [usize; 2] {
    [
        center.saturating_add(distance),
        center.saturating_sub(distance),
    ]
}

fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}
