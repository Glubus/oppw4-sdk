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
