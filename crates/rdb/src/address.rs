use crate::RdbBlock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdbPayloadTail {
    pub raw: String,
    pub part_a: u32,
    pub part_b: u32,
    pub suffix: Option<RdbAddressSuffix>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdbAddressSuffix {
    pub marker: char,
    pub value: u32,
}

pub fn parse_payload_tail(payload: &[u8]) -> Option<RdbPayloadTail> {
    for start in 0..payload.len() {
        let tail = &payload[start..];
        let Some(end) = tail.iter().position(|&byte| byte == 0) else {
            continue;
        };
        let Ok(candidate) = std::str::from_utf8(&tail[..end]) else {
            continue;
        };
        if let Some(tail) = parse_tail_candidate(candidate) {
            return Some(tail);
        }
    }

    None
}

pub fn parse_block_tail(block: &RdbBlock) -> Option<RdbPayloadTail> {
    let address_len = block.field_10 as usize;
    if address_len == 0 || address_len > block.length as usize {
        return None;
    }

    let block_tail_offset = block.length as usize - address_len;
    let payload_tail_offset = block_tail_offset.checked_sub(0x30)?;
    let tail = block.payload.get(payload_tail_offset..)?;
    let end = tail
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(tail.len());
    let candidate = std::str::from_utf8(&tail[..end]).ok()?;
    parse_tail_candidate(candidate)
}

fn parse_tail_candidate(candidate: &str) -> Option<RdbPayloadTail> {
    let (part_a, rest) = candidate.split_once('@')?;
    let (part_b, suffix) = split_address_suffix(rest)?;
    if part_a.is_empty() || part_b.is_empty() {
        return None;
    }

    Some(RdbPayloadTail {
        raw: candidate.to_string(),
        part_a: u32::from_str_radix(part_a, 16).ok()?,
        part_b: u32::from_str_radix(part_b, 16).ok()?,
        suffix,
    })
}

fn split_address_suffix(rest: &str) -> Option<(&str, Option<RdbAddressSuffix>)> {
    if let Some((part_b, value)) = rest.split_once('#') {
        return parse_suffix(part_b, '#', value);
    }

    if let Some((part_b, value)) = rest.split_once('&') {
        return parse_suffix(part_b, '&', value);
    }

    Some((rest, None))
}

fn parse_suffix<'a>(
    part_b: &'a str,
    marker: char,
    value: &str,
) -> Option<(&'a str, Option<RdbAddressSuffix>)> {
    if value.is_empty() {
        return None;
    }

    Some((
        part_b,
        Some(RdbAddressSuffix {
            marker,
            value: u32::from_str_radix(value, 16).ok()?,
        }),
    ))
}
