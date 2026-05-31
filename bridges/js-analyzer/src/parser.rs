pub(crate) fn parse_character_get(source: &str, start: usize) -> Option<(String, usize)> {
    let text = &source[start..];
    let prefixes = [
        "sdk.character.get",
        "sdk.character.find",
        "sdk.character.findById",
        "sdk.character.find_by_id",
    ];
    let prefix = prefixes.iter().find(|prefix| text.starts_with(**prefix))?;
    let args_start = skip_ws(source, start + prefix.len());
    if !source[args_start..].starts_with('(') {
        return None;
    }
    let args_end = matching_delimiter(source, args_start, '(', ')')?;
    let args = &source[args_start + 1..args_end];
    let (character, _) = read_string_literal(args, skip_ws(args, 0))?;
    Some((character, args_end + 1))
}

pub(crate) fn receiver_before(source: &str, method_start: usize) -> Option<&str> {
    let mut start = method_start;
    let bytes = source.as_bytes();
    while start > 0 && bytes[start - 1].is_ascii_whitespace() {
        start -= 1;
    }
    if start == 0 {
        return None;
    }
    if bytes[start - 1] == b')' {
        let open = matching_open_delimiter(source, start - 1, '(', ')')?;
        let expr_start = expression_start_before(source, open)?;
        return Some(&source[expr_start..start]);
    }
    let expr_start = expression_start_before(source, start)?;
    Some(&source[expr_start..start])
}

fn expression_start_before(source: &str, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut start = end;
    while start > 0 {
        let char = bytes[start - 1] as char;
        if char.is_ascii_alphanumeric() || matches!(char, '_' | '$' | '.' | ')' | '(') {
            start -= 1;
        } else {
            break;
        }
    }
    (start < end).then_some(start)
}

pub(crate) fn string_property(source: &str, name: &str) -> Option<String> {
    let value_start = property_value_start(source, name)?;
    let (value, _) = read_string_literal(source, value_start)?;
    Some(value)
}

pub(crate) fn object_property<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let value_start = property_value_start(source, name)?;
    if !source[value_start..].starts_with('{') {
        return None;
    }
    let end = matching_delimiter(source, value_start, '{', '}')?;
    Some(&source[value_start + 1..end])
}

fn property_value_start(source: &str, name: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(index) = source[offset..].find(name) {
        let key_start = offset + index;
        let key_end = key_start + name.len();
        offset = key_end;
        if !is_key_boundary(source, key_start, key_end) {
            continue;
        }
        let colon = skip_ws(source, key_end);
        if source[colon..].starts_with(':') {
            return Some(skip_ws(source, colon + 1));
        }
    }
    None
}

pub(crate) fn string_properties(source: &str) -> Vec<(String, String)> {
    let mut properties = Vec::new();
    let mut offset = 0;
    while let Some((name, after_name)) = read_property_key(source, offset) {
        let colon = skip_ws(source, after_name);
        offset = after_name;
        if !source[colon..].starts_with(':') {
            continue;
        }
        let value_start = skip_ws(source, colon + 1);
        if let Some((value, value_end)) = read_string_literal(source, value_start) {
            properties.push((name.to_string(), value));
            offset = value_end;
        }
    }
    properties
}

fn read_property_key(source: &str, start: usize) -> Option<(&str, usize)> {
    let start = skip_until_identifier(source, start)?;
    read_identifier(source, start)
}

pub(crate) fn read_identifier(source: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let first = *bytes.get(start)? as char;
    if !(first == '_' || first == '$' || first.is_ascii_alphabetic()) {
        return None;
    }
    let mut end = start + 1;
    while let Some(byte) = bytes.get(end) {
        let char = *byte as char;
        if char == '_' || char == '$' || char.is_ascii_alphanumeric() {
            end += 1;
        } else {
            break;
        }
    }
    Some((&source[start..end], end))
}

pub(crate) fn read_string_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let quote = source.as_bytes().get(start).copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut value = String::new();
    let mut index = start + 1;
    let bytes = source.as_bytes();
    while let Some(byte) = bytes.get(index).copied() {
        if byte == quote {
            return Some((value, index + 1));
        }
        if byte == b'\\' {
            index += 1;
            value.push(*bytes.get(index)? as char);
        } else {
            value.push(byte as char);
        }
        index += 1;
    }
    None
}

pub(crate) fn matching_delimiter(
    source: &str,
    open: usize,
    open_char: char,
    close_char: char,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut string_quote = None;
    let bytes = source.as_bytes();
    let mut index = open;
    while let Some(byte) = bytes.get(index).copied() {
        let char = byte as char;
        if let Some(quote) = string_quote {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if char == quote {
                string_quote = None;
            }
        } else if char == '"' || char == '\'' {
            string_quote = Some(char);
        } else if char == open_char {
            depth += 1;
        } else if char == close_char {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn matching_open_delimiter(
    source: &str,
    close: usize,
    open_char: char,
    close_char: char,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, char) in source[..=close].char_indices().rev() {
        if char == close_char {
            depth += 1;
        } else if char == open_char {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

pub(crate) fn skip_ws(source: &str, mut index: usize) -> usize {
    while source
        .as_bytes()
        .get(index)
        .is_some_and(u8::is_ascii_whitespace)
    {
        index += 1;
    }
    index
}

pub(crate) fn skip_ws_after_comma(source: &str, index: usize) -> usize {
    let index = skip_ws(source, index);
    if source[index..].starts_with(',') {
        skip_ws(source, index + 1)
    } else {
        index
    }
}

fn skip_until_identifier(source: &str, mut index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    while let Some(byte) = bytes.get(index) {
        let char = *byte as char;
        if char == '_' || char == '$' || char.is_ascii_alphabetic() {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_key_boundary(source: &str, start: usize, end: usize) -> bool {
    let before = start == 0
        || !source.as_bytes()[start - 1].is_ascii_alphanumeric()
            && source.as_bytes()[start - 1] != b'_';
    let after = source
        .as_bytes()
        .get(end)
        .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_');
    before && after
}
