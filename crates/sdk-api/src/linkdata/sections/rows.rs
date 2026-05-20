use std::ops::Range;

use super::LinkDataSectionError;

pub(super) fn row_count(
    sections: &[Vec<u8>],
    section: usize,
    record_size: usize,
) -> Result<usize, LinkDataSectionError> {
    validate_record_size(record_size)?;
    let section_bytes = sections
        .get(section)
        .ok_or(LinkDataSectionError::SectionOutOfBounds { section })?;
    Ok(section_bytes.len() / record_size)
}

pub(super) fn replace_row(
    sections: &mut [Vec<u8>],
    section: usize,
    record_size: usize,
    row: usize,
    bytes: &[u8],
) -> Result<(), LinkDataSectionError> {
    validate_row(record_size, bytes)?;
    let range = row_range(sections, section, record_size, row)?;
    sections[section][range].copy_from_slice(bytes);
    Ok(())
}

pub(super) fn insert_row(
    sections: &mut [Vec<u8>],
    section: usize,
    record_size: usize,
    row: usize,
    bytes: &[u8],
) -> Result<(), LinkDataSectionError> {
    validate_row(record_size, bytes)?;
    let row_count = row_count(sections, section, record_size)?;
    if row > row_count {
        return Err(LinkDataSectionError::RowOutOfBounds { section, row });
    }
    let offset = row * record_size;
    sections[section].splice(offset..offset, bytes.iter().copied());
    Ok(())
}

pub(super) fn remove_row(
    sections: &mut [Vec<u8>],
    section: usize,
    record_size: usize,
    row: usize,
) -> Result<Vec<u8>, LinkDataSectionError> {
    let range = row_range(sections, section, record_size, row)?;
    Ok(sections[section].drain(range).collect())
}

fn row_range(
    sections: &[Vec<u8>],
    section: usize,
    record_size: usize,
    row: usize,
) -> Result<Range<usize>, LinkDataSectionError> {
    validate_record_size(record_size)?;
    let section_bytes = sections
        .get(section)
        .ok_or(LinkDataSectionError::SectionOutOfBounds { section })?;
    let start = row * record_size;
    let end = start + record_size;
    if end > section_bytes.len() {
        return Err(LinkDataSectionError::RowOutOfBounds { section, row });
    }
    Ok(start..end)
}

fn validate_row(record_size: usize, bytes: &[u8]) -> Result<(), LinkDataSectionError> {
    validate_record_size(record_size)?;
    if bytes.len() != record_size {
        return Err(LinkDataSectionError::RowSizeMismatch {
            expected: record_size,
            actual: bytes.len(),
        });
    }
    Ok(())
}

fn validate_record_size(record_size: usize) -> Result<(), LinkDataSectionError> {
    if record_size == 0 {
        return Err(LinkDataSectionError::InvalidRecordSize);
    }
    Ok(())
}
