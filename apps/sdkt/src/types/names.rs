pub(super) fn pascal_case(value: &str) -> String {
    let mut output = String::new();
    let parts = value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "Sdk".to_string();
    }
    if parts.len() == 1 {
        return preserve_pascal_case(parts[0]);
    }
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            output.push(first.to_ascii_uppercase());
            for ch in chars {
                output.push(ch.to_ascii_lowercase());
            }
        }
    }
    if output.is_empty() {
        "Sdk".to_string()
    } else {
        output
    }
}

pub(super) fn preserve_pascal_case(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return "Sdk".to_string();
    };
    let mut output = String::new();
    output.push(first.to_ascii_uppercase());
    output.extend(chars);
    output
}

pub(super) fn ts_type_name(value: &str) -> String {
    if value.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
        pascal_case(value)
    } else {
        preserve_pascal_case(value)
    }
}
