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

fn two_row_section_payload() -> Vec<u8> {
    let mut sections = LinkDataEntrySections::new(1);
    sections
        .section_mut(0)
        .expect("section")
        .extend_from_slice(&[1, 0, 0, 0, 2, 0, 0, 0]);
    sections.rebuild()
}

fn empty_archive() -> LinkDataArchive {
    use plugin_sdk::linkdata::{LINKDATA_MAGIC, LINKDATA_OFFSET_GRANULARITY};

    let mut bytes = vec![0u8; LINKDATA_OFFSET_GRANULARITY];
    bytes[0..4].copy_from_slice(&LINKDATA_MAGIC.to_le_bytes());
    bytes[4..8].copy_from_slice(&0u32.to_le_bytes());
    LinkDataArchive::parse(bytes).expect("archive")
}
