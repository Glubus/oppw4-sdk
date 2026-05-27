use super::*;

#[test]
fn rejects_different_plugin_for_existing_entry() {
    let mut edits = LinkDataEdits::default();

    assert!(edits
        .replace_entry("a", LinkDataEntryId::new(247), vec![1])
        .is_ok());
    assert_eq!(
        edits
            .replace_entry("b", LinkDataEntryId::new(247), vec![2])
            .expect_err("conflict"),
        "a"
    );
}

#[test]
fn same_plugin_can_replace_own_entry() {
    let mut edits = LinkDataEdits::default();

    assert!(edits
        .replace_entry("a", LinkDataEntryId::new(247), vec![1])
        .is_ok());
    assert!(edits
        .replace_entry("a", LinkDataEntryId::new(247), vec![2])
        .is_ok());
}

#[test]
fn rejects_different_plugin_for_claimed_row() {
    let mut edits = LinkDataEdits::default();
    let entry = LinkDataEntryId::new(3);
    let patch = replace_patch();

    assert!(edits.patch_row("a", entry, patch.clone()).is_ok());
    assert_eq!(
        edits.patch_row("b", entry, patch).expect_err("conflict"),
        "a"
    );
}

#[test]
fn rejects_entry_replacement_after_other_plugin_row_edit() {
    let mut edits = LinkDataEdits::default();
    let entry = LinkDataEntryId::new(3);

    assert!(edits.patch_row("a", entry, replace_first_row()).is_ok());
    assert_eq!(
        edits
            .replace_entry("b", entry, two_row_section_payload())
            .expect_err("conflict"),
        "a"
    );
}

#[test]
fn same_plugin_can_mix_entry_replacement_and_row_edits() {
    let mut edits = LinkDataEdits::default();
    let entry = LinkDataEntryId::new(3);

    assert!(edits
        .replace_entry("a", entry, two_row_section_payload())
        .is_ok());
    assert!(edits.patch_row("a", entry, replace_first_row()).is_ok());

    let archive = empty_archive();
    let payloads = edits.entry_payloads(&archive).expect("payloads");
    let sections = LinkDataEntrySections::parse(payloads.get(&entry).expect("entry payload"))
        .expect("sections");

    assert_eq!(
        sections.section(0).expect("section"),
        [9, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn row_edits_apply_to_archive_payload_when_entry_is_not_replaced() {
    let mut edits = LinkDataEdits::default();
    let entry = LinkDataEntryId::new(0);

    assert!(edits.patch_row("a", entry, replace_first_row()).is_ok());

    let archive = archive_with_entries([two_row_section_payload()]);
    let payloads = edits.entry_payloads(&archive).expect("payloads");
    let sections = LinkDataEntrySections::parse(payloads.get(&entry).expect("entry payload"))
        .expect("sections");

    assert_eq!(
        sections.section(0).expect("section"),
        [9, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn same_plugin_can_insert_and_remove_rows() {
    let mut edits = LinkDataEdits::default();
    let entry = LinkDataEntryId::new(3);

    assert!(edits
        .replace_entry("a", entry, two_row_section_payload())
        .is_ok());
    assert!(edits.patch_row("a", entry, insert_middle_row()).is_ok());
    assert!(edits.patch_row("a", entry, remove_last_row()).is_ok());

    let payloads = edits.entry_payloads(&empty_archive()).expect("payloads");
    let sections = LinkDataEntrySections::parse(payloads.get(&entry).expect("entry payload"))
        .expect("sections");

    assert_eq!(
        sections.section(0).expect("section"),
        [1, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
}

fn replace_patch() -> RowPatch {
    RowPatch::Replace {
        section: 1,
        record_size: 16,
        row: 2,
        payload: vec![0; 16],
    }
}

fn replace_first_row() -> RowPatch {
    RowPatch::Replace {
        section: 0,
        record_size: 4,
        row: 0,
        payload: vec![9, 0, 0, 0],
    }
}

fn insert_middle_row() -> RowPatch {
    RowPatch::Insert {
        section: 0,
        record_size: 4,
        row: 1,
        payload: vec![7, 0, 0, 0],
    }
}

fn remove_last_row() -> RowPatch {
    RowPatch::Remove {
        section: 0,
        record_size: 4,
        row: 2,
    }
}

fn two_row_section_payload() -> Vec<u8> {
    let mut sections = LinkDataEntrySections::new(1);
    sections
        .section_mut(0)
        .expect("section")
        .extend_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0]);
    sections.rebuild()
}

fn empty_archive() -> LinkDataArchive {
    archive_with_entries([])
}

fn archive_with_entries<const N: usize>(entries: [Vec<u8>; N]) -> LinkDataArchive {
    use plugin_sdk::linkdata::{
        LINKDATA_MAGIC, LINKDATA_OFFSET_GRANULARITY, LINKDATA_RECORD_SIZE, LINKDATA_TABLE_OFFSET,
    };

    let table_len = LINKDATA_TABLE_OFFSET + entries.len() * LINKDATA_RECORD_SIZE;
    let payload_start =
        table_len.div_ceil(LINKDATA_OFFSET_GRANULARITY) * LINKDATA_OFFSET_GRANULARITY;
    let payload_len = entries.iter().map(Vec::len).sum::<usize>();
    let mut bytes = vec![0u8; payload_start + payload_len];
    bytes[0..4].copy_from_slice(&LINKDATA_MAGIC.to_le_bytes());
    bytes[4..8].copy_from_slice(&(entries.len() as u32).to_le_bytes());

    let mut payload_offset = payload_start;
    for (index, entry) in entries.iter().enumerate() {
        let table_offset = LINKDATA_TABLE_OFFSET + index * LINKDATA_RECORD_SIZE;
        let entry_offset = (payload_offset / LINKDATA_OFFSET_GRANULARITY) as u32;
        bytes[table_offset..table_offset + 4].copy_from_slice(&entry_offset.to_le_bytes());
        bytes[table_offset + 8..table_offset + 12]
            .copy_from_slice(&(entry.len() as u32).to_le_bytes());
        bytes[payload_offset..payload_offset + entry.len()].copy_from_slice(entry);
        payload_offset += entry.len();
    }

    LinkDataArchive::parse(bytes).expect("archive")
}
