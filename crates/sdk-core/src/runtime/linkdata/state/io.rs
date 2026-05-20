use super::LinkDataState;

impl LinkDataState {
    pub(in crate::runtime::linkdata) unsafe fn read(
        &mut self,
        handle: u64,
        buffer: *mut u8,
        bytes_to_read: u32,
        requested_offset: i64,
        out_bytes_read: *mut u32,
    ) -> i32 {
        self.file
            .as_mut()
            .map(|file| {
                file.read(
                    handle,
                    buffer,
                    bytes_to_read,
                    requested_offset,
                    out_bytes_read,
                )
            })
            .unwrap_or(0)
    }

    pub(in crate::runtime::linkdata) fn close(&mut self, handle: u64) -> i32 {
        self.file
            .as_mut()
            .and_then(|file| file.close(handle))
            .map(|_| 1)
            .unwrap_or(0)
    }

    pub(in crate::runtime::linkdata) unsafe fn size(&mut self, out_size: *mut u64) -> i32 {
        if out_size.is_null() {
            return -1;
        }
        let Some(file) = self.file.as_ref() else {
            return 0;
        };
        *out_size = file.len() as u64;
        1
    }

    pub(in crate::runtime::linkdata) unsafe fn seek(
        &mut self,
        handle: u64,
        distance: i64,
        move_method: u32,
        out_position: *mut u64,
    ) -> i32 {
        self.file
            .as_mut()
            .map(|file| file.seek(handle, distance, move_method, out_position))
            .unwrap_or(0)
    }
}
