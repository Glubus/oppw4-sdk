use std::{fs, path::Path};

pub(crate) fn read_payload(path: &Path) -> Result<Vec<u8>, String> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("bin") => {
            fs::read(path).map_err(|error| error.to_string())
        }
        Some(extension)
            if extension.eq_ignore_ascii_case("txt") || extension.eq_ignore_ascii_case("hex") =>
        {
            let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
            parse_payload(&text)
        }
        Some(extension) => Err(format!("unsupported patch extension: {extension}")),
        None => Err("patch missing extension".to_string()),
    }
}

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
