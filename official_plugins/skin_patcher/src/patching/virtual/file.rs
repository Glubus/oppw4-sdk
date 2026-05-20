use std::{
    fmt,
    io::{Read, Seek, SeekFrom},
};

use crate::patching::{ReadSeek, VirtualReplacement};

pub struct VirtualFile {
    prefix: Vec<u8>,
    reader: Box<dyn ReadSeek + Send>,
    size: u64,
    position: u64,
}

impl fmt::Debug for VirtualFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VirtualFile")
            .field("prefix_len", &self.prefix.len())
            .field("size", &self.size)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl VirtualFile {
    pub fn new(reader: Box<dyn ReadSeek + Send>, size: u64) -> Self {
        Self {
            prefix: Vec::new(),
            reader,
            size,
            position: 0,
        }
    }

    pub fn with_prefix(
        prefix: Vec<u8>,
        reader: Box<dyn ReadSeek + Send>,
        payload_size: u64,
    ) -> Self {
        let size = prefix.len() as u64 + payload_size;
        Self {
            prefix,
            reader,
            size,
            position: 0,
        }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.size.saturating_sub(self.position);
        let requested = buffer.len().min(remaining as usize);
        if requested == 0 {
            return Ok(0);
        }

        let mut total_read = 0;
        if self.position < self.prefix.len() as u64 {
            let prefix_start = self.position as usize;
            let prefix_read = requested.min(self.prefix.len() - prefix_start);
            buffer[..prefix_read]
                .copy_from_slice(&self.prefix[prefix_start..prefix_start + prefix_read]);
            self.position += prefix_read as u64;
            total_read += prefix_read;
        }

        if total_read < requested {
            let payload_position = self.position.saturating_sub(self.prefix.len() as u64);
            self.reader.seek(SeekFrom::Start(payload_position))?;
            let read = self.reader.read(&mut buffer[total_read..requested])?;
            self.position += read as u64;
            total_read += read;
        }

        Ok(total_read)
    }

    pub fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let new_position = match position {
            SeekFrom::Start(position) => position,
            SeekFrom::Current(offset) => add_signed_offset(self.position, offset)?,
            SeekFrom::End(offset) => add_signed_offset(self.size, offset)?,
        };

        self.position = new_position;
        Ok(self.position)
    }
}

pub fn open_virtual_replacement(replacement: &VirtualReplacement) -> std::io::Result<VirtualFile> {
    let reader = replacement.source.open_reader()?;
    let size = match replacement.mod_size {
        Some(size) => size,
        None => replacement.source.payload_size()?,
    };

    if let Some(prefix) = replacement.virtual_prefix.as_ref() {
        let prefix = patch_virtual_prefix(prefix, size);
        return Ok(VirtualFile::with_prefix(prefix, reader, size));
    }

    Ok(VirtualFile::new(reader, size))
}

fn patch_virtual_prefix(prefix: &[u8], payload_size: u64) -> Vec<u8> {
    let mut prefix = prefix.to_vec();
    if prefix.len() < 0x20 {
        return prefix;
    }

    let payload_size_u32 = payload_size as u32;
    let virtual_size = payload_size_u32.wrapping_add(prefix.len() as u32);
    prefix[0x08..0x0c].copy_from_slice(&virtual_size.to_le_bytes());
    prefix[0x10..0x14].copy_from_slice(&payload_size_u32.to_le_bytes());
    prefix[0x18..0x20].copy_from_slice(&payload_size.to_le_bytes());
    if prefix.len() >= 0x30 {
        prefix[0x2c..0x30].copy_from_slice(&0x10000u32.to_le_bytes());
    }
    prefix
}

fn add_signed_offset(base: u64, offset: i64) -> std::io::Result<u64> {
    if offset >= 0 {
        return base.checked_add(offset as u64).ok_or_else(invalid_seek);
    }

    base.checked_sub(offset.unsigned_abs())
        .ok_or_else(invalid_seek)
}

fn invalid_seek() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "invalid virtual file seek",
    )
}

#[cfg(test)]
mod tests {
    use super::patch_virtual_prefix;

    #[test]
    fn virtual_prefix_matches_original_patcher_size_patch() {
        let mut prefix = vec![0xcc; 0x48];
        prefix[0..8].copy_from_slice(b"IDRK0000");
        prefix[0x08..0x0c].copy_from_slice(&0x56u32.to_le_bytes());
        prefix[0x10..0x14].copy_from_slice(&0x0eu32.to_le_bytes());
        prefix[0x18..0x20].copy_from_slice(&0xaa14u64.to_le_bytes());
        prefix[0x2c..0x30].copy_from_slice(&0x120000u32.to_le_bytes());

        let patched = patch_virtual_prefix(&prefix, 0xeb54);

        assert_eq!(&patched[0x08..0x0c], &0xeb9cu32.to_le_bytes());
        assert_eq!(&patched[0x10..0x14], &0xeb54u32.to_le_bytes());
        assert_eq!(&patched[0x18..0x20], &0xeb54u64.to_le_bytes());
        assert_eq!(&patched[0x2c..0x30], &0x10000u32.to_le_bytes());
    }
}
