pub(super) fn set_bool(table: &toml::Value, key: &str, output: &mut bool) {
    if let Some(value) = table.get(key).and_then(toml::Value::as_bool) {
        *output = value;
    }
}

pub(super) fn set_u64_min(table: &toml::Value, key: &str, min: i64, output: &mut u64) {
    if let Some(value) = table.get(key).and_then(toml::Value::as_integer) {
        *output = value.max(min) as u64;
    }
}

pub(super) fn set_usize_range(
    table: &toml::Value,
    key: &str,
    min: i64,
    max: usize,
    output: &mut usize,
) {
    if let Some(value) = table.get(key).and_then(toml::Value::as_integer) {
        *output = (value.max(min) as usize).min(max);
    }
}

pub(super) fn u32_values(table: &toml::Value, key: &str) -> Option<Vec<u32>> {
    let parsed = table
        .get(key)?
        .as_array()?
        .iter()
        .filter_map(toml::Value::as_integer)
        .filter_map(|value| u32::try_from(value).ok())
        .collect::<Vec<_>>();
    (!parsed.is_empty()).then_some(parsed)
}

pub(super) fn u16_values(table: &toml::Value, key: &str) -> Option<Vec<u16>> {
    let parsed = table
        .get(key)?
        .as_array()?
        .iter()
        .filter_map(toml::Value::as_integer)
        .filter_map(|value| u16::try_from(value).ok())
        .collect::<Vec<_>>();
    (!parsed.is_empty()).then_some(parsed)
}

pub(super) fn u32_array<const N: usize>(
    table: &toml::Value,
    key: &str,
    max: u32,
) -> Option<[u32; N]> {
    let values = table.get(key)?.as_array()?;
    if values.len() != N {
        return None;
    }

    let mut output = [0; N];
    for (slot, value) in output.iter_mut().zip(values) {
        let parsed = value.as_integer()?;
        *slot = (parsed.max(0) as u32).min(max);
    }
    Some(output)
}
