use std::collections::BTreeMap;

pub(super) struct VirtualFile {
    bytes: Vec<u8>,
    next_handle: u64,
    positions: BTreeMap<u64, usize>,
}

impl VirtualFile {
    pub(super) fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            next_handle: 1,
            positions: BTreeMap::new(),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.bytes.len()
    }

    pub(super) fn has_handle(&self, handle: u64) -> bool {
        self.positions.contains_key(&handle)
    }

    pub(super) fn open(&mut self) -> u64 {
        let handle = self.next_handle;
        self.next_handle += 1;
        self.positions.insert(handle, 0);
        handle
    }

    pub(super) fn close(&mut self, handle: u64) -> Option<()> {
        self.positions.remove(&handle).map(|_| ())
    }

    pub(super) unsafe fn read(
        &mut self,
        handle: u64,
        buffer: *mut u8,
        bytes_to_read: u32,
        requested_offset: i64,
        out_bytes_read: *mut u32,
    ) -> i32 {
        if buffer.is_null() {
            return -1;
        }
        let Some(position) = self.positions.get_mut(&handle) else {
            return 0;
        };
        if requested_offset >= 0 {
            *position = requested_offset as usize;
        }
        let start = (*position).min(self.bytes.len());
        let end = (start + bytes_to_read as usize).min(self.bytes.len());
        let read = end.saturating_sub(start);
        std::ptr::copy_nonoverlapping(self.bytes[start..end].as_ptr(), buffer, read);
        if !out_bytes_read.is_null() {
            *out_bytes_read = read as u32;
        }
        *position = end;
        1
    }

    pub(super) unsafe fn seek(
        &mut self,
        handle: u64,
        distance: i64,
        move_method: u32,
        out_position: *mut u64,
    ) -> i32 {
        let Some(position) = self.positions.get_mut(&handle) else {
            return 0;
        };
        let base = match move_method {
            0 => 0i64,
            1 => *position as i64,
            2 => self.bytes.len() as i64,
            _ => return 0,
        };
        let new_position = base.saturating_add(distance);
        if new_position < 0 {
            return 0;
        }
        *position = new_position as usize;
        if !out_position.is_null() {
            *out_position = *position as u64;
        }
        1
    }
}
