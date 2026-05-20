use crate::bytes::find_bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameHashEntry {
    pub name: String,
    pub hash: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HashToken {
    marker_start: usize,
    digits_start: usize,
    digits_end: usize,
}

pub fn parse_name_hash_catalog(bytes: &[u8]) -> Vec<NameHashEntry> {
    parse_line_catalog(bytes).unwrap_or_else(|| parse_embedded_catalog(bytes))
}

fn parse_line_catalog(bytes: &[u8]) -> Option<Vec<NameHashEntry>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let entries: Vec<_> = text.lines().filter_map(parse_catalog_line).collect();
    (!entries.is_empty()).then_some(entries)
}

fn parse_catalog_line(line: &str) -> Option<NameHashEntry> {
    let (hash_text, name) = line.trim().split_once(',')?;
    let hash = parse_hash_text(hash_text.trim().strip_prefix("0x")?)?;
    let name = name.trim();
    is_catalog_name(name).then(|| NameHashEntry {
        name: name.to_string(),
        hash,
    })
}

fn parse_embedded_catalog(bytes: &[u8]) -> Vec<NameHashEntry> {
    iter_hash_tokens(bytes)
        .filter_map(|token| parse_entry_around_hash(bytes, token))
        .collect()
}

fn iter_hash_tokens(bytes: &[u8]) -> impl Iterator<Item = HashToken> + '_ {
    std::iter::from_fn({
        let mut offset = 0;
        move || find_next_hash_token(bytes, &mut offset)
    })
}

fn find_next_hash_token(bytes: &[u8], offset: &mut usize) -> Option<HashToken> {
    while *offset < bytes.len() {
        let relative_start = find_bytes(&bytes[*offset..], b"0x")?;
        let marker_start = *offset + relative_start;
        let digits_start = marker_start + 2;
        let digits_end = find_hash_digits_end(bytes, digits_start);
        *offset = digits_end.saturating_add(1);

        if digits_end - digits_start == 8 {
            return Some(HashToken {
                marker_start,
                digits_start,
                digits_end,
            });
        }
    }
    None
}

fn find_hash_digits_end(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| !byte.is_ascii_hexdigit())
        .map(|offset| start + offset)
        .unwrap_or(bytes.len())
}

fn parse_entry_around_hash(bytes: &[u8], token: HashToken) -> Option<NameHashEntry> {
    let name = catalog_name_after_hash(bytes, token.digits_end)
        .or_else(|| catalog_name_before_hash(bytes, token.marker_start))?;
    let hash = parse_hash_slice(bytes, token)?;
    Some(NameHashEntry { name, hash })
}

fn parse_hash_slice(bytes: &[u8], token: HashToken) -> Option<u32> {
    let hash_text = std::str::from_utf8(&bytes[token.digits_start..token.digits_end]).ok()?;
    parse_hash_text(hash_text)
}

fn parse_hash_text(hash_text: &str) -> Option<u32> {
    (hash_text.len() == 8)
        .then(|| u32::from_str_radix(hash_text, 16).ok())
        .flatten()
}

fn catalog_name_after_hash(bytes: &[u8], hash_digits_end: usize) -> Option<String> {
    if bytes.get(hash_digits_end).copied() != Some(b',') {
        return None;
    }

    let start = skip_catalog_separators_after(bytes, hash_digits_end);
    let end = find_catalog_name_end(bytes, start);
    if start == end {
        return None;
    }

    let name = std::str::from_utf8(&bytes[start..end]).ok()?;
    is_catalog_name(name).then(|| name.to_string())
}

fn catalog_name_before_hash(bytes: &[u8], hash_start: usize) -> Option<String> {
    let end = skip_catalog_separators_before(bytes, hash_start);
    let start = find_catalog_name_start(bytes, end);
    let name = std::str::from_utf8(&bytes[start..end]).ok()?;
    is_catalog_name(name).then(|| name.to_string())
}

fn skip_catalog_separators_after(bytes: &[u8], mut offset: usize) -> usize {
    while offset < bytes.len() && is_catalog_separator(bytes[offset]) {
        offset += 1;
    }
    offset
}

fn skip_catalog_separators_before(bytes: &[u8], mut offset: usize) -> usize {
    while offset > 0 && is_catalog_separator(bytes[offset - 1]) {
        offset -= 1;
    }
    offset
}

fn find_catalog_name_start(bytes: &[u8], mut offset: usize) -> usize {
    while offset > 0 && bytes[offset - 1] != b',' && !is_catalog_separator(bytes[offset - 1]) {
        offset -= 1;
    }
    offset
}

fn find_catalog_name_end(bytes: &[u8], mut offset: usize) -> usize {
    while offset < bytes.len() && !is_catalog_separator(bytes[offset]) {
        offset += 1;
    }
    offset
}

fn is_catalog_separator(byte: u8) -> bool {
    byte == 0 || byte == b',' || byte == b'\r' || byte == b'\n'
}

fn is_catalog_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    !name.is_empty()
        && lower
            .rsplit_once('.')
            .map(|(_, extension)| is_known_asset_extension(extension))
            .unwrap_or(false)
}

fn is_known_asset_extension(extension: &str) -> bool {
    matches!(
        extension,
        "g1e"
            | "g1m"
            | "g1t"
            | "mtl"
            | "grp"
            | "oid"
            | "oidex"
            | "ktid"
            | "kts"
            | "swg"
            | "texinfo"
            | "kscl"
            | "kidssingletondb"
            | "kidsobjdb"
            | "kidsscndb"
            | "name"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_catalog_parser_reads_hash_then_name_rows() {
        let entries =
            parse_name_hash_catalog(b"0x359b9672,800_294_face_law_dressrosa_External_00.g1t\r\n");

        assert_eq!(
            entries,
            vec![NameHashEntry {
                name: "800_294_face_law_dressrosa_External_00.g1t".to_string(),
                hash: 0x359b9672,
            }]
        );
    }

    #[test]
    fn line_catalog_parser_keeps_screen_layout_kscl_assets() {
        let entries = parse_name_hash_catalog(b"0x386d71a0,105_05_costume_change.kscl\r\n");

        assert_eq!(
            entries,
            vec![NameHashEntry {
                name: "105_05_costume_change.kscl".to_string(),
                hash: 0x386d71a0,
            }]
        );
    }

    #[test]
    fn line_catalog_parser_keeps_kids_database_assets() {
        let entries = parse_name_hash_catalog(
            b"0xfbfc2a79,Layout_105_05_costume_change.kidssingletondb\r\n\
              0xd5407f20,Layout_105_05_costume_change.kidssingletondb.name\r\n",
        );

        assert_eq!(
            entries,
            vec![
                NameHashEntry {
                    name: "Layout_105_05_costume_change.kidssingletondb".to_string(),
                    hash: 0xfbfc2a79,
                },
                NameHashEntry {
                    name: "Layout_105_05_costume_change.kidssingletondb.name".to_string(),
                    hash: 0xd5407f20,
                },
            ]
        );
    }

    #[test]
    fn embedded_catalog_parser_reads_name_before_hash_tokens() {
        let bytes = [
            b"noise\0".as_slice(),
            b"MPR_Bound_Character_MPLC012Newgate_skin_kidsalb.g1t".as_slice(),
            b"\0".as_slice(),
            b"0x88f15f35\0".as_slice(),
        ]
        .concat();
        let entries = parse_name_hash_catalog(&bytes);

        assert_eq!(
            entries,
            vec![NameHashEntry {
                name: "MPR_Bound_Character_MPLC012Newgate_skin_kidsalb.g1t".to_string(),
                hash: 0x88f15f35,
            }]
        );
    }
}
