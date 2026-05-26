use std::collections::BTreeMap;
use std::io::Write;

use super::*;

fn fixture() -> Vec<u8> {
    let mut bytes = vec![0u8; LINKDATA_OFFSET_GRANULARITY];
    bytes[0..4].copy_from_slice(&LINKDATA_MAGIC.to_le_bytes());
    bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
    bytes[LINKDATA_TABLE_OFFSET..LINKDATA_TABLE_OFFSET + 4].copy_from_slice(&1u32.to_le_bytes());
    bytes[LINKDATA_TABLE_OFFSET + 8..LINKDATA_TABLE_OFFSET + 12]
        .copy_from_slice(&4u32.to_le_bytes());
    bytes[LINKDATA_TABLE_OFFSET + LINKDATA_RECORD_SIZE
        ..LINKDATA_TABLE_OFFSET + LINKDATA_RECORD_SIZE + 4]
        .copy_from_slice(&1u32.to_le_bytes());
    bytes[LINKDATA_TABLE_OFFSET + LINKDATA_RECORD_SIZE + 8
        ..LINKDATA_TABLE_OFFSET + LINKDATA_RECORD_SIZE + 12]
        .copy_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 3, 4]);
    bytes
}

#[test]
fn parses_entries() {
    let archive = LinkDataArchive::parse(fixture()).expect("archive");

    assert_eq!(archive.entries().len(), 2);
    assert_eq!(
        archive.entry_payload(LinkDataEntryId(0)).expect("payload"),
        [1, 2, 3, 4]
    );
}

#[test]
fn rebuilds_with_entry_replacement() {
    let archive = LinkDataArchive::parse(fixture()).expect("archive");
    let edits = BTreeMap::from([(LinkDataEntryId(1), vec![9, 8])]);

    let rebuilt = archive
        .rebuild_with_entry_payloads(&edits)
        .expect("rebuilt");
    let archive = LinkDataArchive::parse(rebuilt).expect("archive");

    assert_eq!(
        archive.entry_payload(LinkDataEntryId(0)).expect("entry 0"),
        [1, 2, 3, 4]
    );
    assert_eq!(
        archive.entry_payload(LinkDataEntryId(1)).expect("entry 1"),
        [9, 8]
    );
}

#[test]
fn parses_compressed_entry_payload() {
    let payload = [7, 8, 9, 10, 11, 12];
    let compressed = zlib_compress(&payload);
    let mut bytes = vec![0u8; LINKDATA_OFFSET_GRANULARITY];
    bytes[0..4].copy_from_slice(&LINKDATA_MAGIC.to_le_bytes());
    bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
    bytes[LINKDATA_TABLE_OFFSET..LINKDATA_TABLE_OFFSET + 4].copy_from_slice(&1u32.to_le_bytes());
    bytes[LINKDATA_TABLE_OFFSET + 8..LINKDATA_TABLE_OFFSET + 12]
        .copy_from_slice(&(compressed.len() as u32).to_le_bytes());
    bytes[LINKDATA_TABLE_OFFSET + 12..LINKDATA_TABLE_OFFSET + 16]
        .copy_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(compressed.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&compressed);

    let archive = LinkDataArchive::parse(bytes).expect("archive");

    assert_eq!(
        archive.entry_payload(LinkDataEntryId(0)).expect("payload"),
        payload
    );
}

#[test]
fn section_rows_can_be_replaced_inserted_and_removed() {
    let mut sections = LinkDataEntrySections::new(2);
    sections
        .section_mut(0)
        .expect("section 0")
        .extend_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0]);
    sections
        .section_mut(1)
        .expect("section 1")
        .extend_from_slice(&[9, 0, 0, 0]);

    sections
        .replace_row(0, 4, 1, &[3, 0, 0, 0])
        .expect("replace");
    sections.insert_row(0, 4, 1, &[4, 0, 0, 0]).expect("insert");
    let removed = sections.remove_row(1, 4, 0).expect("remove");

    assert_eq!(removed, [9, 0, 0, 0]);
    assert_eq!(
        sections.section(0).expect("section 0"),
        [1, 0, 0, 0, 4, 0, 0, 0, 3, 0, 0, 0]
    );
    assert_eq!(sections.section(1).expect("section 1"), &[] as &[u8]);
}

#[test]
fn rebuilt_sections_parse_back_to_rows() {
    let mut sections = LinkDataEntrySections::new(1);
    sections
        .section_mut(0)
        .expect("section")
        .extend_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);

    let rebuilt = sections.rebuild();
    let parsed = LinkDataEntrySections::parse(&rebuilt).expect("sections");

    assert_eq!(parsed.section_count(), 1);
    assert_eq!(
        parsed.section(0).expect("section"),
        [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]
    );
    assert_eq!(parsed.row_count(0, 4).expect("row count"), 4);
}

#[test]
fn row_operations_validate_bounds_and_sizes() {
    let mut sections = LinkDataEntrySections::new(1);

    assert!(matches!(
        sections.insert_row(0, 4, 1, &[1, 2, 3, 4]),
        Err(LinkDataSectionError::RowOutOfBounds { section: 0, row: 1 })
    ));
    assert!(matches!(
        sections.insert_row(0, 4, 0, &[1, 2]),
        Err(LinkDataSectionError::RowSizeMismatch {
            expected: 4,
            actual: 2
        })
    ));
}

#[test]
fn curated_entries_include_observed_movesets_without_section_claims() {
    let garp = entries::movesets::garp_entry_247::LAYOUT;
    let rayleigh = entries::movesets::rayleigh_entry_248::LAYOUT;

    assert_eq!(garp.entry, LinkDataEntryId::new(247));
    assert_eq!(rayleigh.entry, LinkDataEntryId::new(248));
    assert_eq!(garp.sections, []);
    assert_eq!(rayleigh.sections, []);
    assert!(entries::CURATED
        .iter()
        .any(|entry| entry.entry == LinkDataEntryId::new(247)));
    assert!(entries::CURATED
        .iter()
        .any(|entry| entry.entry == LinkDataEntryId::new(248)));
}

#[test]
fn fixed_data_refs_keep_logical_ids_separate_from_archive_entries() {
    let stream = FixedDataStreamRef::new(FixedDataLogicalId::new(0x14), 2558);
    let candidate = FixedDataSourceCandidate::new(
        FixedDataLogicalId::new(0x14),
        LinkDataFile::A,
        LinkDataEntryId::new(2558),
    );

    assert_eq!(stream.logical_id.get(), 0x14);
    assert_eq!(stream.runtime_stream_id, 2558);
    assert_eq!(
        candidate.file.relative_path(),
        "LINKDATA/CMN/LINKDATA_A.BIN"
    );
    assert_eq!(candidate.entry.get(), 2558);
}

fn zlib_compress(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).expect("compress payload");
    encoder.finish().expect("finish payload")
}
