use std::{collections::HashMap, io::SeekFrom};

use crate::patching::{open_virtual_replacement, VirtualFile, VirtualReplacement};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtualHandle(u64);

impl VirtualHandle {
    pub fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub fn as_raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub struct VirtualHandleTable {
    next_id: u64,
    files: HashMap<VirtualHandle, VirtualFile>,
}

impl VirtualHandleTable {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            files: HashMap::new(),
        }
    }

    pub fn open(&mut self, replacement: &VirtualReplacement) -> std::io::Result<VirtualHandle> {
        let file = open_virtual_replacement(replacement)?;
        let handle = self.allocate_handle();
        self.files.insert(handle, file);
        Ok(handle)
    }

    pub fn read(&mut self, handle: VirtualHandle, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.file_mut(handle)?.read(buffer)
    }

    pub fn seek(&mut self, handle: VirtualHandle, position: SeekFrom) -> std::io::Result<u64> {
        self.file_mut(handle)?.seek(position)
    }

    pub fn size(&self, handle: VirtualHandle) -> std::io::Result<u64> {
        self.files
            .get(&handle)
            .map(VirtualFile::size)
            .ok_or_else(unknown_virtual_handle)
    }

    pub fn close(&mut self, handle: VirtualHandle) -> bool {
        self.files.remove(&handle).is_some()
    }

    pub fn contains(&self, handle: VirtualHandle) -> bool {
        self.files.contains_key(&handle)
    }

    fn allocate_handle(&mut self) -> VirtualHandle {
        let handle = VirtualHandle(self.next_id);
        self.next_id += 1;
        handle
    }

    fn file_mut(&mut self, handle: VirtualHandle) -> std::io::Result<&mut VirtualFile> {
        self.files
            .get_mut(&handle)
            .ok_or_else(unknown_virtual_handle)
    }
}

impl Default for VirtualHandleTable {
    fn default() -> Self {
        Self::new()
    }
}

fn unknown_virtual_handle() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::NotFound, "unknown virtual handle")
}
