pub fn parse_prefixed_hex_hash(name: &str) -> Option<u32> {
    let rest = name
        .strip_prefix("0x")
        .or_else(|| name.strip_prefix("0X"))?;
    let hex_len = rest
        .bytes()
        .position(|byte| byte == b'.' || !byte.is_ascii_hexdigit())
        .unwrap_or(rest.len());
    if hex_len == 0 {
        return None;
    }

    u32::from_str_radix(&rest[..hex_len], 16).ok()
}
