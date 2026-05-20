pub(crate) fn parse_payload(text: &str) -> Result<Vec<u8>, String> {
    let mut hex = String::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default();
        for ch in line.chars() {
            if ch.is_ascii_hexdigit() {
                hex.push(ch);
            } else if ch.is_whitespace() || matches!(ch, ',' | ':' | ';' | '_' | '-') {
                continue;
            } else {
                return Err(format!("invalid hex character: {ch:?}"));
            }
        }
    }
    if !hex.len().is_multiple_of(2) {
        return Err("odd hex length".to_string());
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for index in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[index..index + 2], 16).map_err(|e| e.to_string())?);
    }
    Ok(bytes)
}
