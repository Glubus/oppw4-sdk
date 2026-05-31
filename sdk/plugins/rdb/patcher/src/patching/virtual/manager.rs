use std::{collections::BTreeMap, io::SeekFrom};

use crate::patching::{VirtualHandle, VirtualHandleTable, VirtualReplacement};

#[derive(Debug)]
pub struct VirtualManager {
    replacements: Vec<VirtualReplacement>,
    archive_replacements: BTreeMap<String, Vec<usize>>,
    lookup: Vec<ReplacementLookup>,
    handles: VirtualHandleTable,
}

#[derive(Debug)]
struct ReplacementLookup {
    index: usize,
    file_name: String,
    hash_tag: String,
}

impl VirtualManager {
    pub fn new(replacements: Vec<VirtualReplacement>) -> Self {
        let archive_replacements = index_replacements_by_archive(&replacements);
        let lookup = replacements
            .iter()
            .enumerate()
            .map(|(index, replacement)| ReplacementLookup {
                index,
                file_name: replacement.file_name.to_ascii_lowercase(),
                hash_tag: format!("0x{:08x}", replacement.hash),
            })
            .collect();
        Self {
            replacements,
            archive_replacements,
            lookup,
            handles: VirtualHandleTable::new(),
        }
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

    pub fn patch_archive_index_external_flags(
        &self,
        archive_name: &str,
        read_offset: u64,
        buffer: &mut [u8],
    ) -> usize {
        let mut patched = 0;
        for replacement in self.replacements_for_archive(archive_name) {
            if patch_replacement_external_size(replacement, read_offset, buffer) {
                patched += 1;
            }
            if patch_replacement_external_flag(replacement, read_offset, buffer) {
                patched += 1;
            }
        }
        patched
    }

    pub fn data_read_hits(
        &self,
        archive_name: &str,
        read_offset: u64,
        read_len: usize,
    ) -> Vec<&VirtualReplacement> {
        let read_end = read_offset + read_len as u64;
        self.replacements_for_archive(archive_name)
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

    fn find_replacement_by_path_fragment(&self, path: &str) -> Option<&VirtualReplacement> {
        let path = path.to_ascii_lowercase();
        let lookup = self
            .lookup
            .iter()
            .find(|lookup| path.contains(&lookup.file_name) || path.contains(&lookup.hash_tag))?;
        self.replacements.get(lookup.index)
    }

    fn replacements_for_archive(
        &self,
        archive_name: &str,
    ) -> impl Iterator<Item = &VirtualReplacement> {
        let archive_name = archive_name.to_ascii_lowercase();
        self.archive_replacements
            .get(&archive_name)
            .into_iter()
            .flat_map(|indexes| indexes.iter())
            .filter_map(|index| self.replacements.get(*index))
    }
}

fn index_replacements_by_archive(
    replacements: &[VirtualReplacement],
) -> BTreeMap<String, Vec<usize>> {
    let mut by_archive = BTreeMap::new();
    for (index, replacement) in replacements.iter().enumerate() {
        by_archive
            .entry(replacement.archive_name.to_ascii_lowercase())
            .or_insert_with(Vec::new)
            .push(index);
    }
    by_archive
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

#[cfg(test)]
mod tests {
    use super::*;

    fn replacement(archive_name: &str, rdb_block_offset: usize) -> VirtualReplacement {
        VirtualReplacement {
            archive_name: archive_name.to_string(),
            file_name: "modded.g1t".to_string(),
            source: crate::patching::ReplacementSource::File("modded.g1t".into()),
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
