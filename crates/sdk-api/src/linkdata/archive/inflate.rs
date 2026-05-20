use std::io::Read;

use flate2::read::ZlibDecoder;

use super::{read_u32, LinkDataEntry, LinkDataError};

pub(super) fn inflate_entry(bytes: &[u8], entry: &LinkDataEntry) -> Result<Vec<u8>, LinkDataError> {
    if entry.uncompressed_size == 0 {
        return raw_entry_payload(bytes, entry);
    }
    inflate_compressed_entry(bytes, entry)
}

fn raw_entry_payload(bytes: &[u8], entry: &LinkDataEntry) -> Result<Vec<u8>, LinkDataError> {
    let end = entry.data_offset + entry.compressed_span;
    bytes
        .get(entry.data_offset..end)
        .map(|payload| payload.to_vec())
        .ok_or(LinkDataError::OutOfBounds {
            entry: entry.id.get(),
        })
}

fn inflate_compressed_entry(bytes: &[u8], entry: &LinkDataEntry) -> Result<Vec<u8>, LinkDataError> {
    let declared = entry.uncompressed_size;
    let block_uncompressed = read_u32(bytes, entry.data_offset)? as usize;
    let expected = declared.min(block_uncompressed);
    let mut cursor = entry.data_offset + 4;
    let mut output = Vec::with_capacity(expected);
    while output.len() < expected {
        let compressed_size = read_u32(bytes, cursor)? as usize;
        cursor += 4;
        cursor = inflate_chunk(bytes, entry, cursor, compressed_size, &mut output)?;
    }
    Ok(output)
}

fn inflate_chunk(
    bytes: &[u8],
    entry: &LinkDataEntry,
    cursor: usize,
    compressed_size: usize,
    output: &mut Vec<u8>,
) -> Result<usize, LinkDataError> {
    let end = cursor + compressed_size;
    let chunk = bytes
        .get(cursor..end)
        .ok_or(LinkDataError::TruncatedChunk {
            entry: entry.id.get(),
        })?;
    let mut decoder = ZlibDecoder::new(chunk);
    decoder
        .read_to_end(output)
        .map_err(|error| LinkDataError::InflateFailed {
            entry: entry.id.get(),
            message: error.to_string(),
        })?;
    Ok(end)
}
