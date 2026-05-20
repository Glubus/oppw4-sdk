use std::io::SeekFrom;

use crate::patching::{VirtualHandle, VirtualHandleTable, VirtualReplacement};

#[derive(Debug)]
pub struct VirtualManager {
    replacements: Vec<VirtualReplacement>,
    handles: VirtualHandleTable,
}

impl VirtualManager {
    pub fn new(replacements: Vec<VirtualReplacement>) -> Self {
        Self {
            replacements,
            handles: VirtualHandleTable::new(),
        }
    }

    pub fn open_by_hash(
        &mut self,
        archive_name: &str,
        hash: u32,
    ) -> std::io::Result<Option<VirtualHandle>> {
        let Some(replacement) = self.find_replacement(archive_name, hash).cloned() else {
            return Ok(None);
        };
        self.handles.open(&replacement).map(Some)
    }

    pub fn open_by_file_name(&mut self, file_name: &str) -> std::io::Result<Option<VirtualHandle>> {
        let Some(replacement) = self.find_replacement_by_file_name(file_name).cloned() else {
            return Ok(None);
        };
        self.handles.open(&replacement).map(Some)
    }

    pub fn open_by_path_fragment(&mut self, path: &str) -> std::io::Result<Option<VirtualHandle>> {
        let Some(replacement) = self.find_replacement_by_path_fragment(path).cloned() else {
            return Ok(None);
        };
        self.handles.open(&replacement).map(Some)
    }

    pub fn open_by_path_fragment_with_replacement(
        &mut self,
        path: &str,
    ) -> std::io::Result<Option<(VirtualHandle, VirtualReplacement)>> {
        let Some(replacement) = self.find_replacement_by_path_fragment(path).cloned() else {
            return Ok(None);
        };
        let handle = self.handles.open(&replacement)?;
        Ok(Some((handle, replacement)))
    }

    pub fn read(&mut self, handle: VirtualHandle, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.handles.read(handle, buffer)
    }

    pub fn seek(&mut self, handle: VirtualHandle, position: SeekFrom) -> std::io::Result<u64> {
        self.handles.seek(handle, position)
    }

    pub fn size(&self, handle: VirtualHandle) -> std::io::Result<u64> {
        self.handles.size(handle)
    }

    pub fn close(&mut self, handle: VirtualHandle) -> bool {
        self.handles.close(handle)
    }

    pub fn patch_archive_read(
        &self,
        archive_name: &str,
        read_offset: u64,
        buffer: &mut [u8],
    ) -> std::io::Result<usize> {
        let mut patched = 0;
        for replacement in self
            .replacements
            .iter()
            .filter(|replacement| replacement.archive_name.eq_ignore_ascii_case(archive_name))
        {
            let Some(start) = replacement.virtual_bin_offset else {
                continue;
            };
            let Some(size) = replacement.mod_size else {
                continue;
            };
            patched += patch_replacement_window(replacement, start, size, read_offset, buffer)?;
        }
        Ok(patched)
    }

    pub fn patch_archive_index_read(
        &self,
        archive_name: &str,
        read_offset: u64,
        buffer: &mut [u8],
    ) -> usize {
        let mut patched = 0;
        for replacement in self
            .replacements
            .iter()
            .filter(|replacement| replacement.archive_name.eq_ignore_ascii_case(archive_name))
        {
            if patch_replacement_index_tail(replacement, read_offset, buffer) {
                patched += 1;
            }
        }
        patched
    }

    pub fn patch_archive_index_direct_paths(
        &self,
        archive_name: &str,
        read_offset: u64,
        buffer: &mut [u8],
    ) -> usize {
        let mut patched = 0;
        for replacement in self
            .replacements
            .iter()
            .filter(|replacement| replacement.archive_name.eq_ignore_ascii_case(archive_name))
        {
            if patch_replacement_index_address_len(replacement, read_offset, buffer) {
                patched += 1;
            }
        }
        patched
    }

    pub fn patch_archive_index_external_flags(
        &self,
        archive_name: &str,
        read_offset: u64,
        buffer: &mut [u8],
    ) -> usize {
        let mut patched = 0;
        for replacement in self
            .replacements
            .iter()
            .filter(|replacement| replacement.archive_name.eq_ignore_ascii_case(archive_name))
        {
            if patch_replacement_external_size(replacement, read_offset, buffer) {
                patched += 1;
            }
            if patch_replacement_external_flag(replacement, read_offset, buffer) {
                patched += 1;
            }
        }
        patched
    }

    pub fn virtual_archive_size(&self, archive_name: &str) -> Option<u64> {
        self.replacements
            .iter()
            .filter(|replacement| replacement.archive_name.eq_ignore_ascii_case(archive_name))
            .filter_map(|replacement| replacement.virtual_bin_offset.zip(replacement.mod_size))
            .map(|(offset, size)| offset + size)
            .max()
    }

    pub fn data_read_hits(
        &self,
        archive_name: &str,
        read_offset: u64,
        read_len: usize,
    ) -> Vec<&VirtualReplacement> {
        let read_end = read_offset + read_len as u64;
        self.replacements
            .iter()
            .filter(|replacement| replacement.archive_name.eq_ignore_ascii_case(archive_name))
            .filter(|replacement| {
                let Some(start) = replacement.original_bin_offset.map(u64::from) else {
                    return false;
                };
                let Some(size) = replacement.original_bin_size.map(u64::from) else {
                    return false;
                };
                let end = start + size;
                read_offset < end && start < read_end
            })
            .collect()
    }

    fn find_replacement(&self, archive_name: &str, hash: u32) -> Option<&VirtualReplacement> {
        self.replacements.iter().find(|replacement| {
            replacement.archive_name.eq_ignore_ascii_case(archive_name) && replacement.hash == hash
        })
    }

    fn find_replacement_by_file_name(&self, file_name: &str) -> Option<&VirtualReplacement> {
        self.replacements
            .iter()
            .find(|replacement| replacement.file_name.eq_ignore_ascii_case(file_name))
    }

    fn find_replacement_by_path_fragment(&self, path: &str) -> Option<&VirtualReplacement> {
        let path = path.to_ascii_lowercase();
        self.replacements.iter().find(|replacement| {
            path.contains(&replacement.file_name.to_ascii_lowercase())
                || path.contains(&format!("0x{:08x}", replacement.hash))
        })
    }
}

fn patch_replacement_window(
    replacement: &VirtualReplacement,
    replacement_offset: u64,
    replacement_size: u64,
    read_offset: u64,
    buffer: &mut [u8],
) -> std::io::Result<usize> {
    let read_end = read_offset + buffer.len() as u64;
    let replacement_end = replacement_offset + replacement_size;
    let overlap_start = read_offset.max(replacement_offset);
    let overlap_end = read_end.min(replacement_end);
    if overlap_start >= overlap_end {
        return Ok(0);
    }

    let target_start = (overlap_start - read_offset) as usize;
    let target_end = (overlap_end - read_offset) as usize;
    let source_start = overlap_start - replacement_offset;
    replacement
        .source
        .read_range(source_start, &mut buffer[target_start..target_end])?;
    Ok(target_end - target_start)
}

fn patch_replacement_index_tail(
    replacement: &VirtualReplacement,
    read_offset: u64,
    buffer: &mut [u8],
) -> bool {
    let Some(bin_offset) = replacement.virtual_bin_offset else {
        return false;
    };
    let Some(mod_size) = replacement.mod_size else {
        return false;
    };
    let Some(tail_offset) = replacement.rdb_tail_offset.map(|offset| offset as u64) else {
        return false;
    };
    let Some(original_tail) = replacement.original_tail.as_deref() else {
        return false;
    };
    let suffix = replacement_suffix(replacement);
    let new_tail = format!("{bin_offset:x}@{mod_size:x}{suffix}");
    if tail_offset < read_offset {
        return false;
    }
    let buffer_offset = (tail_offset - read_offset) as usize;
    let Some(target) = buffer.get_mut(buffer_offset..buffer_offset + original_tail.len() + 1)
    else {
        return false;
    };
    let original_len = target
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(target.len());
    if new_tail.len() > original_len {
        return false;
    }
    target[..new_tail.len()].copy_from_slice(new_tail.as_bytes());
    for byte in &mut target[new_tail.len()..=original_len] {
        *byte = 0;
    }
    true
}

fn patch_replacement_index_address_len(
    replacement: &VirtualReplacement,
    read_offset: u64,
    buffer: &mut [u8],
) -> bool {
    let field_offset = replacement.rdb_block_offset as u64 + 0x10;
    if field_offset < read_offset {
        return false;
    }
    let buffer_offset = (field_offset - read_offset) as usize;
    let Some(target) = buffer.get_mut(buffer_offset..buffer_offset + 4) else {
        return false;
    };
    if target == [0, 0, 0, 0] {
        return false;
    }
    target.copy_from_slice(&0u32.to_le_bytes());
    true
}

fn patch_replacement_external_size(
    replacement: &VirtualReplacement,
    read_offset: u64,
    buffer: &mut [u8],
) -> bool {
    let Some(mod_size) = replacement.mod_size else {
        return false;
    };
    patch_field(
        replacement.rdb_block_offset as u64 + 0x18,
        read_offset,
        buffer,
        &mod_size.to_le_bytes(),
    )
}

fn patch_replacement_external_flag(
    replacement: &VirtualReplacement,
    read_offset: u64,
    buffer: &mut [u8],
) -> bool {
    patch_field(
        replacement.rdb_block_offset as u64 + 0x2c,
        read_offset,
        buffer,
        &0x10000u32.to_le_bytes(),
    )
}

fn patch_field(field_offset: u64, read_offset: u64, buffer: &mut [u8], value: &[u8]) -> bool {
    if field_offset < read_offset {
        return false;
    }
    let buffer_offset = (field_offset - read_offset) as usize;
    let Some(target) = buffer.get_mut(buffer_offset..buffer_offset + value.len()) else {
        return false;
    };
    if target == value {
        return false;
    }
    target.copy_from_slice(value);
    true
}

fn replacement_suffix(replacement: &VirtualReplacement) -> &str {
    let Some(original_tail) = replacement.original_tail.as_deref() else {
        return "";
    };
    if let Some(index) = original_tail.find('#') {
        return &original_tail[index..];
    }
    if let Some(index) = original_tail.find('&') {
        return &original_tail[index..];
    }
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replacement(archive_name: &str, rdb_block_offset: usize) -> VirtualReplacement {
        VirtualReplacement {
            archive_name: archive_name.to_string(),
            file_name: "modded.g1t".to_string(),
            source: crate::patching::ReplacementSource::File("modded.g1t".into()),
            mode: crate::patching::ReplacementMode::Virtual,
            mod_size: Some(0x1234),
            hash: 0x1234_5678,
            rdb_block_offset,
            original_data_offset: 0,
            original_bin_offset: Some(0x1000),
            original_bin_size: Some(0x2000),
            virtual_bin_offset: None,
            rdb_tail_offset: None,
            original_tail: None,
            virtual_prefix: None,
        }
    }

    #[test]
    fn index_direct_path_patch_zeros_address_len_field() {
        let manager = VirtualManager::new(vec![replacement("ScreenLayout", 0x20)]);
        let mut buffer = vec![0xcc; 0x40];
        buffer[0x30..0x34].copy_from_slice(&0x12u32.to_le_bytes());

        let patched = manager.patch_archive_index_direct_paths("ScreenLayout", 0, &mut buffer);

        assert_eq!(patched, 1);
        assert_eq!(&buffer[0x30..0x34], &[0, 0, 0, 0]);
    }

    #[test]
    fn index_direct_path_patch_respects_archive_and_read_window() {
        let manager = VirtualManager::new(vec![replacement("ScreenLayout", 0x20)]);
        let mut buffer = vec![0xcc; 0x20];

        assert_eq!(
            manager.patch_archive_index_direct_paths("MaterialEditor", 0, &mut buffer),
            0
        );
        assert_eq!(
            manager.patch_archive_index_direct_paths("ScreenLayout", 0x40, &mut buffer),
            0
        );
    }

    #[test]
    fn index_external_flag_patch_matches_old_loader_fields() {
        let manager = VirtualManager::new(vec![replacement("CharacterEditor", 0x40)]);
        let mut buffer = vec![0xcc; 0x80];

        let patched = manager.patch_archive_index_external_flags("CharacterEditor", 0, &mut buffer);

        assert_eq!(patched, 2);
        assert_eq!(&buffer[0x58..0x60], &0x1234u64.to_le_bytes());
        assert_eq!(&buffer[0x6c..0x70], &0x10000u32.to_le_bytes());
    }

    #[test]
    fn index_external_flag_patch_updates_external_metadata_aliases_like_original_loader() {
        let mut replacement = replacement("ScreenLayout", 0x40);
        replacement.file_name = "800_294_face_law_dressrosa_External_00.g1t".to_string();
        replacement.mod_size = Some(0x200038);
        replacement.original_bin_size = Some(0xa4);
        let manager = VirtualManager::new(vec![replacement]);
        let mut buffer = vec![0xcc; 0x80];

        let patched = manager.patch_archive_index_external_flags("ScreenLayout", 0, &mut buffer);

        assert_eq!(patched, 2);
        assert_eq!(&buffer[0x58..0x60], &0x200038u64.to_le_bytes());
        assert_eq!(&buffer[0x6c..0x70], &0x10000u32.to_le_bytes());
    }
}
