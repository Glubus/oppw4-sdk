use super::{LinkDataEntrySections, LinkDataSectionError};

pub(super) fn parse_sections(
    payload: &[u8],
) -> Result<LinkDataEntrySections, LinkDataSectionError> {
    let offsets = section_offsets(payload)?;
    let table_end = 4 + offsets.len() * 4;
    let mut sections = Vec::with_capacity(offsets.len());
    for (index, start) in offsets.iter().copied().enumerate() {
        sections.push(section_payload(payload, &offsets, table_end, index, start)?);
    }
    Ok(LinkDataEntrySections::from_sections(sections))
}

fn section_offsets(payload: &[u8]) -> Result<Vec<usize>, LinkDataSectionError> {
    if payload.len() < 4 {
        return Err(LinkDataSectionError::InvalidHeader);
    }
    let count = read_section_u32(payload, 0).ok_or(LinkDataSectionError::InvalidHeader)? as usize;
    let table_end = 4 + count * 4;
    if payload.len() < table_end {
        return Err(LinkDataSectionError::InvalidHeader);
    }
    (0..count)
        .map(|index| {
            read_section_u32(payload, 4 + index * 4)
                .map(|offset| offset as usize)
                .ok_or(LinkDataSectionError::InvalidHeader)
        })
        .collect()
}

fn section_payload(
    payload: &[u8],
    offsets: &[usize],
    table_end: usize,
    index: usize,
    start: usize,
) -> Result<Vec<u8>, LinkDataSectionError> {
    if start == 0 {
        return Ok(Vec::new());
    }
    validate_section_start(payload, table_end, index, start)?;
    let end = section_end(payload, offsets, index, start)?;
    Ok(payload[start..end].to_vec())
}

fn validate_section_start(
    payload: &[u8],
    table_end: usize,
    index: usize,
    start: usize,
) -> Result<(), LinkDataSectionError> {
    if start < table_end || start > payload.len() {
        return Err(LinkDataSectionError::InvalidSectionOffset {
            section: index,
            offset: start,
        });
    }
    Ok(())
}

fn section_end(
    payload: &[u8],
    offsets: &[usize],
    index: usize,
    start: usize,
) -> Result<usize, LinkDataSectionError> {
    let end = offsets
        .iter()
        .copied()
        .skip(index + 1)
        .find(|next| *next >= start)
        .unwrap_or(payload.len());
    if end > payload.len() {
        return Err(LinkDataSectionError::InvalidSectionOffset {
            section: index,
            offset: end,
        });
    }
    Ok(end)
}

fn read_section_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let raw = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(raw.try_into().ok()?))
}
