use crate::patching::{ModAsset, ReplacementSource};
use rdb::{parse_block_tail, ArchiveScan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualReplacement {
    pub archive_name: String,
    pub file_name: String,
    pub source: ReplacementSource,
    pub mod_size: Option<u64>,
    pub hash: u32,
    pub rdb_block_offset: usize,
    pub original_data_offset: u32,
    pub original_bin_offset: Option<u32>,
    pub original_bin_size: Option<u32>,
    pub virtual_bin_offset: Option<u64>,
    pub rdb_tail_offset: Option<usize>,
    pub original_tail: Option<String>,
    pub virtual_prefix: Option<Vec<u8>>,
}

pub fn attach_mod_file_sizes(
    replacements: Vec<VirtualReplacement>,
) -> std::io::Result<Vec<VirtualReplacement>> {
    replacements.into_iter().map(attach_mod_file_size).collect()
}

pub fn assign_virtual_bin_offsets(
    mut replacements: Vec<VirtualReplacement>,
    archive_name: &str,
    base_offset: u64,
) -> Vec<VirtualReplacement> {
    let mut next_offset = align_virtual_offset(base_offset);
    for replacement in replacements
        .iter_mut()
        .filter(|replacement| replacement.archive_name.eq_ignore_ascii_case(archive_name))
    {
        let Some(mod_size) = replacement.mod_size else {
            continue;
        };
        replacement.virtual_bin_offset = Some(next_offset);
        next_offset = align_virtual_offset(next_offset + mod_size);
    }
    replacements
}

fn attach_mod_file_size(
    mut replacement: VirtualReplacement,
) -> std::io::Result<VirtualReplacement> {
    replacement.mod_size = Some(replacement.source.payload_size()?);
    Ok(replacement)
}

pub fn build_virtualization_table_from_assets(
    scan: &ArchiveScan<'_>,
    assets: &[ModAsset],
) -> Vec<VirtualReplacement> {
    scan.files
        .iter()
        .filter_map(|file| {
            let source = assets
                .iter()
                .find(|asset| asset.file_name.eq_ignore_ascii_case(&file.file_name))?
                .source
                .clone();
            build_replacement(scan, file, source)
        })
        .collect()
}

fn build_replacement(
    scan: &ArchiveScan<'_>,
    file: &rdb::VirtualizedFile<'_>,
    source: ReplacementSource,
) -> Option<VirtualReplacement> {
    let hash = file.hash?;
    let block = file.block?;
    let tail = parse_block_tail(block);
    let rdb_tail_offset = (block.field_10 != 0)
        .then(|| block.offset + block.length as usize - block.field_10 as usize);

    Some(VirtualReplacement {
        archive_name: scan.archive_name.clone(),
        file_name: file.file_name.clone(),
        source,
        mod_size: None,
        hash,
        rdb_block_offset: block.offset,
        original_data_offset: block.data_offset,
        original_bin_offset: tail.as_ref().map(|tail| tail.part_a),
        original_bin_size: tail.as_ref().map(|tail| tail.part_b),
        virtual_bin_offset: None,
        rdb_tail_offset,
        original_tail: tail.map(|tail| tail.raw),
        virtual_prefix: build_virtual_prefix(block),
    })
}

fn align_virtual_offset(offset: u64) -> u64 {
    (offset + 0xffff) & !0xffff
}

fn build_virtual_prefix(block: &rdb::RdbBlock) -> Option<Vec<u8>> {
    let address_len = block.field_10 as usize;
    if address_len == 0 || address_len > block.raw.len() {
        return None;
    }

    let prefix_len = block.raw.len().checked_sub(address_len)?;
    Some(block.raw[..prefix_len].to_vec())
}
