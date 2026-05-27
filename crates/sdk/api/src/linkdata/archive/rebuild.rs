use std::collections::BTreeMap;

use super::super::types::{
    align_up, write_u32, LinkDataEntryId, LINKDATA_OFFSET_GRANULARITY, LINKDATA_RECORD_SIZE,
    LINKDATA_TABLE_OFFSET,
};
use super::{inflate, LinkDataEntry, LinkDataError};

pub(super) fn rebuild_with_entry_payloads(
    source: &[u8],
    entries: &[LinkDataEntry],
    edits: &BTreeMap<LinkDataEntryId, Vec<u8>>,
) -> Result<Vec<u8>, LinkDataError> {
    let mut output = rebuild_header(source, entries.len());
    for entry in entries {
        let payload = entry_payload(source, entry, edits)?;
        align_output(&mut output);
        write_entry_record(&mut output, entry, payload.len());
        output.extend_from_slice(&payload);
    }
    Ok(output)
}

fn rebuild_header(source: &[u8], entry_count: usize) -> Vec<u8> {
    let table_end = LINKDATA_TABLE_OFFSET + entry_count * LINKDATA_RECORD_SIZE;
    let data_start = align_up(table_end, LINKDATA_OFFSET_GRANULARITY);
    let mut output = vec![0u8; data_start];
    output[..LINKDATA_TABLE_OFFSET].copy_from_slice(&source[..LINKDATA_TABLE_OFFSET]);
    output
}

fn entry_payload(
    source: &[u8],
    entry: &LinkDataEntry,
    edits: &BTreeMap<LinkDataEntryId, Vec<u8>>,
) -> Result<Vec<u8>, LinkDataError> {
    edits
        .get(&entry.id)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| inflate::inflate_entry(source, entry))
}

fn align_output(output: &mut Vec<u8>) {
    while !output.len().is_multiple_of(LINKDATA_OFFSET_GRANULARITY) {
        output.push(0);
    }
}

fn write_entry_record(output: &mut [u8], entry: &LinkDataEntry, payload_len: usize) {
    let offset_units = output.len() / LINKDATA_OFFSET_GRANULARITY;
    write_u32(output, entry.table_offset, offset_units as u32);
    write_u32(output, entry.table_offset + 0x04, entry.field_04);
    write_u32(output, entry.table_offset + 0x08, payload_len as u32);
    write_u32(output, entry.table_offset + 0x0c, 0);
}
