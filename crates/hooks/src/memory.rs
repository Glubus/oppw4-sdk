use std::{ffi::c_void, mem, ptr, slice};

use crate::win;

const MEM_COMMIT: u32 = 0x1000;
const PAGE_NOACCESS: u32 = 0x01;
const PAGE_GUARD: u32 = 0x100;

pub fn module_base() -> usize {
    win::main_module() as usize
}

/// Reads `len` bytes from a process address into `out`.
///
/// # Safety
///
/// `out` must be valid for writes of `len` bytes. The caller must ensure the
/// target address belongs to the current process and that copying from it does
/// not violate aliasing requirements for any Rust references.
pub unsafe fn read_memory(address: usize, out: *mut u8, len: usize) -> i32 {
    if address == 0 || out.is_null() {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if !is_readable_range(address, len) {
        return -2;
    }
    ptr::copy_nonoverlapping(address as *const u8, out, len);
    0
}

/// Writes `len` bytes from `bytes` into a process address.
///
/// # Safety
///
/// `bytes` must be valid for reads of `len` bytes. The caller must ensure the
/// target address belongs to the current process and that overwriting it is
/// compatible with all code and data that may observe the region.
pub unsafe fn write_memory(address: usize, bytes: *const u8, len: usize) -> i32 {
    if address == 0 || bytes.is_null() {
        return -1;
    }
    if len == 0 {
        return 0;
    }

    let mut old_protect = 0;
    if !win::make_memory_writable(address as *mut c_void, len, &mut old_protect) {
        return -2;
    }
    ptr::copy_nonoverlapping(bytes, address as *mut u8, len);
    let _ = win::flush_instruction_cache(address as *const c_void, len);
    let _ = win::restore_memory_protection(address as *mut c_void, len, old_protect);
    0
}

/// Scans the current module image for a masked byte pattern.
///
/// # Safety
///
/// `pattern` and `mask` must both be valid for reads of `len` bytes. The mask
/// uses zero bytes as wildcards and non-zero bytes as exact-match positions.
pub unsafe fn scan_memory(pattern: *const u8, mask: *const u8, len: usize) -> usize {
    if pattern.is_null() || mask.is_null() || len == 0 {
        return 0;
    }
    let base = module_base();
    let Some(size) = module_image_size(base) else {
        return 0;
    };
    let image = slice::from_raw_parts(base as *const u8, size);
    let pattern = slice::from_raw_parts(pattern, len);
    let mask = slice::from_raw_parts(mask, len);
    scan_slice(image, pattern, mask)
        .map(|offset| base + offset)
        .unwrap_or(0)
}

unsafe fn module_image_size(module: usize) -> Option<usize> {
    if module == 0 {
        return None;
    }
    let dos = module as *const u8;
    if slice::from_raw_parts(dos, 2) != b"MZ" {
        return None;
    }
    let nt_offset = *(dos.add(0x3c) as *const u32) as usize;
    let nt = dos.add(nt_offset);
    if slice::from_raw_parts(nt, 4) != b"PE\0\0" {
        return None;
    }
    let optional_header = nt.add(24);
    let magic = *(optional_header as *const u16);
    if magic != 0x20b {
        return None;
    }
    Some(*(optional_header.add(0x38) as *const u32) as usize)
}

fn scan_slice(haystack: &[u8], pattern: &[u8], mask: &[u8]) -> Option<usize> {
    if pattern.is_empty() || pattern.len() != mask.len() || pattern.len() > haystack.len() {
        return None;
    }
    haystack.windows(pattern.len()).position(|window| {
        window
            .iter()
            .zip(pattern.iter())
            .zip(mask.iter())
            .all(|((byte, expected), mask)| *mask == 0 || byte == expected)
    })
}

fn is_readable_range(address: usize, len: usize) -> bool {
    let Some(end) = address.checked_add(len.saturating_sub(1)) else {
        return false;
    };
    let mut cursor = address;
    while cursor <= end {
        let Some(region) = (unsafe { query_memory(cursor) }) else {
            return false;
        };
        if region.state != MEM_COMMIT
            || region.protect & (PAGE_NOACCESS | PAGE_GUARD) != 0
            || region.region_size == 0
        {
            return false;
        }
        let region_end = region.base_address.saturating_add(region.region_size);
        if region_end == 0 || region_end <= cursor {
            return false;
        }
        if region_end > end {
            return true;
        }
        cursor = region_end;
    }
    true
}

unsafe fn query_memory(address: usize) -> Option<MemoryBasicInformation> {
    let mut info = MemoryBasicInformation::default();
    let written = VirtualQuery(
        address as *const c_void,
        (&mut info as *mut MemoryBasicInformation).cast(),
        mem::size_of::<MemoryBasicInformation>(),
    );
    (written != 0).then_some(info)
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MemoryBasicInformation {
    base_address: usize,
    allocation_base: usize,
    allocation_protect: u32,
    partition_id: u16,
    _alignment: u16,
    region_size: usize,
    state: u32,
    protect: u32,
    type_: u32,
}

extern "system" {
    fn VirtualQuery(address: *const c_void, buffer: *mut c_void, length: usize) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_slice_supports_wildcards() {
        let haystack = [0x10, 0x44, 0x8b, 0xaa, 0x41, 0xff];
        let pattern = [0x44, 0x8b, 0x00, 0x41];
        let mask = [1, 1, 0, 1];

        assert_eq!(scan_slice(&haystack, &pattern, &mask), Some(1));
    }

    #[test]
    fn scan_slice_rejects_mismatched_mask() {
        assert_eq!(scan_slice(&[1, 2, 3], &[2, 3], &[1]), None);
    }
}
