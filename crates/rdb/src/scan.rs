use crate::{parse_prefixed_hex_hash, NameHashEntry, RdbBlock, RdbIndex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualizedFile<'a> {
    pub file_name: String,
    pub hash: Option<u32>,
    pub block: Option<&'a RdbBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveScan<'a> {
    pub archive_name: String,
    pub files: Vec<VirtualizedFile<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveScanCounts {
    pub total: usize,
    pub matched: usize,
    pub hash_missing: usize,
    pub unresolved_names: usize,
}

pub fn scan_archive_names_with_catalog<'a>(
    archive_name: impl Into<String>,
    index: &'a RdbIndex,
    file_names: impl IntoIterator<Item = impl AsRef<str>>,
    catalog: &[NameHashEntry],
) -> ArchiveScan<'a> {
    ArchiveScan {
        archive_name: archive_name.into(),
        files: scan_virtualized_names_with_catalog(index, file_names, catalog),
    }
}

impl ArchiveScan<'_> {
    pub fn counts(&self) -> ArchiveScanCounts {
        ArchiveScanCounts {
            total: self.files.len(),
            matched: self
                .files
                .iter()
                .filter(|file| file.block.is_some())
                .count(),
            hash_missing: self
                .files
                .iter()
                .filter(|file| file.hash.is_some() && file.block.is_none())
                .count(),
            unresolved_names: self.files.iter().filter(|file| file.hash.is_none()).count(),
        }
    }
}

pub fn scan_virtualized_names<'a>(
    index: &'a RdbIndex,
    file_names: impl IntoIterator<Item = impl AsRef<str>>,
) -> Vec<VirtualizedFile<'a>> {
    scan_virtualized_names_with_catalog(index, file_names, &[])
}

pub fn scan_virtualized_names_with_catalog<'a>(
    index: &'a RdbIndex,
    file_names: impl IntoIterator<Item = impl AsRef<str>>,
    catalog: &[NameHashEntry],
) -> Vec<VirtualizedFile<'a>> {
    file_names
        .into_iter()
        .map(|file_name| scan_virtualized_name(index, file_name.as_ref(), catalog))
        .collect()
}

fn scan_virtualized_name<'a>(
    index: &'a RdbIndex,
    file_name: &str,
    catalog: &[NameHashEntry],
) -> VirtualizedFile<'a> {
    let hash = resolve_file_hash(file_name, catalog);
    let block = hash.and_then(|hash| find_block_by_hash(index, hash));

    VirtualizedFile {
        file_name: file_name.to_string(),
        hash,
        block,
    }
}

fn resolve_file_hash(file_name: &str, catalog: &[NameHashEntry]) -> Option<u32> {
    parse_prefixed_hex_hash(file_name).or_else(|| resolve_catalog_hash(file_name, catalog))
}

fn resolve_catalog_hash(file_name: &str, catalog: &[NameHashEntry]) -> Option<u32> {
    catalog
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(file_name))
        .map(|entry| entry.hash)
}

fn find_block_by_hash(index: &RdbIndex, hash: u32) -> Option<&RdbBlock> {
    index.blocks.iter().find(|block| block.primary_hash == hash)
}
