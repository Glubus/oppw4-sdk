use std::{ffi::c_void, ptr};

const MEM_COMMIT: u32 = 0x1000;
const MEM_RESERVE: u32 = 0x2000;
const PAGE_EXECUTE_READWRITE: u32 = 0x40;
const ALLOCATION_GRANULARITY: usize = 0x10000;
const NEAR_ALLOC_SEARCH_RANGE: usize = 0x0800_0000;

pub(crate) struct CaveArena {
    base: usize,
    cursor: usize,
    size: usize,
}

impl CaveArena {
    pub(crate) unsafe fn new(hint: usize, size: usize) -> Option<Self> {
        allocate_near_executable_block(hint, size).map(|base| Self {
            base,
            cursor: 0,
            size,
        })
    }

    pub(crate) unsafe fn alloc(&mut self, bytes: &[u8], alignment: usize) -> Result<usize, String> {
        self.cursor = align_up(self.cursor, alignment.max(1));
        let end = self
            .cursor
            .checked_add(bytes.len())
            .ok_or_else(|| "runtime cave arena overflow".to_string())?;
        if end > self.size {
            return Err("runtime cave arena full".to_string());
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

extern "system" {
    fn VirtualAlloc(
        address: *mut c_void,
        size: usize,
        allocation_type: u32,
        protect: u32,
    ) -> *mut c_void;
}
