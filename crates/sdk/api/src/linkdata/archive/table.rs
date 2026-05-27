use super::super::types::{
    LinkDataEntryId, LINKDATA_MAGIC, LINKDATA_OFFSET_GRANULARITY, LINKDATA_RECORD_SIZE,
    LINKDATA_TABLE_OFFSET,
};
use super::{read_u32, LinkDataEntry, LinkDataError};

pub(super) fn parse_entries(bytes: &[u8]) -> Result<Vec<LinkDataEntry>, LinkDataError> {
    validate_header(bytes)?;
    let count = read_u32(bytes, 4)? as usize;
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        entries.push(parse_entry(bytes, index)?);
    }
    Ok(entries)
}

fn validate_header(bytes: &[u8]) -> Result<(), LinkDataError> {
    if bytes.len() < LINKDATA_TABLE_OFFSET || read_u32(bytes, 0)? != LINKDATA_MAGIC {
        return Err(LinkDataError::InvalidHeader);
    }
    Ok(())
}

fn parse_entry(bytes: &[u8], index: usize) -> Result<LinkDataEntry, LinkDataError> {
    let table_offset = LINKDATA_TABLE_OFFSET + index * LINKDATA_RECORD_SIZE;
    if table_offset + LINKDATA_RECORD_SIZE > bytes.len() {
        return Err(LinkDataError::TruncatedTable { entry: index });
    }
    Ok(LinkDataEntry {
        id: LinkDataEntryId(index as u32),
        table_offset,
        data_offset: read_u32(bytes, table_offset)? as usize * LINKDATA_OFFSET_GRANULARITY,
        field_04: read_u32(bytes, table_offset + 0x04)?,
        compressed_span: read_u32(bytes, table_offset + 0x08)? as usize,
        uncompressed_size: read_u32(bytes, table_offset + 0x0c)? as usize,
    })
}
