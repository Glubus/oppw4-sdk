use serde::Deserialize;

use crate::model::{AddressSpec, DebugConfig, Scan, TargetValue, ValueType, Watch};

#[derive(Default, Deserialize)]
struct DebugFile {
    enabled: Option<bool>,
    interval_ms: Option<u64>,
    #[serde(default)]
    watches: Vec<WatchFile>,
    #[serde(default)]
    scans: Vec<ScanFile>,
}

#[derive(Deserialize)]
struct WatchFile {
    id: String,
    #[serde(rename = "type")]
    value_type: String,
    address: AddressFile,
}

#[derive(Deserialize)]
struct ScanFile {
    id: String,
    #[serde(rename = "type")]
    value_type: String,
    start: AddressFile,
    bytes: Option<usize>,
    #[serde(default)]
    values: Vec<ValueFile>,
    max_hits: Option<usize>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AddressFile {
    Integer(usize),
    String(String),
    PointerChain {
        base: Box<AddressFile>,
        #[serde(default)]
        offsets: Vec<usize>,
    },
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ValueFile {
    Integer(i64),
    Float(f32),
}

pub(crate) fn parse(text: &str) -> Result<DebugConfig, String> {
    let file = toml::from_str::<DebugFile>(text).map_err(|error| error.to_string())?;
    let mut config = DebugConfig {
        enabled: file.enabled.unwrap_or(false),
        interval_ms: file.interval_ms.unwrap_or(500),
        watches: Vec::with_capacity(file.watches.len()),
        scans: Vec::with_capacity(file.scans.len()),
    };

    for watch in file.watches {
        config.watches.push(Watch {
            id: watch.id,
            value_type: parse_type(&watch.value_type)?,
            address: parse_address(watch.address)?,
        });
    }
    for scan in file.scans {
        let value_type = parse_type(&scan.value_type)?;
        config.scans.push(Scan {
            id: scan.id,
            value_type,
            start: parse_address(scan.start)?,
            bytes: scan.bytes.unwrap_or(4096),
            values: parse_values(scan.values, value_type),
            max_hits: scan.max_hits.unwrap_or(32),
        });
    }
    Ok(config)
}

fn parse_values(values: Vec<ValueFile>, value_type: ValueType) -> Vec<TargetValue> {
    values
        .into_iter()
        .map(|value| match (value_type, value) {
            (ValueType::F32, ValueFile::Float(value)) => TargetValue::Float(value),
            (_, ValueFile::Float(value)) => TargetValue::Integer(value as i64),
            (_, ValueFile::Integer(value)) => TargetValue::Integer(value),
        })
        .collect()
}

fn parse_address(value: AddressFile) -> Result<AddressSpec, String> {
    match value {
        AddressFile::Integer(value) => Ok(AddressSpec::Absolute(value)),
        AddressFile::String(value) => parse_address_string(&value),
        AddressFile::PointerChain { base, offsets } => Ok(AddressSpec::PointerChain {
            base: Box::new(parse_address(*base)?),
            offsets,
        }),
    }
}

fn parse_address_string(text: &str) -> Result<AddressSpec, String> {
    let text = text.trim();
    if let Some(rva) = text.strip_prefix("module+") {
        return parse_usize(rva).map(AddressSpec::ModuleRva);
    }
    parse_usize(text).map(AddressSpec::Absolute)
}

fn parse_type(text: &str) -> Result<ValueType, String> {
    match text.to_ascii_lowercase().as_str() {
        "u8" => Ok(ValueType::U8),
        "u16" => Ok(ValueType::U16),
        "u32" => Ok(ValueType::U32),
        "i32" => Ok(ValueType::I32),
        "f32" => Ok(ValueType::F32),
        _ => Err(format!("unsupported value type: {text}")),
    }
}

fn parse_usize(text: &str) -> Result<usize, String> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix("0x") {
        usize::from_str_radix(hex, 16).map_err(|error| error.to_string())
    } else {
        text.parse::<usize>().map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_watch_and_scan() {
        let config = parse(
            r#"
            enabled = true

            [[watches]]
            id = "difficulty"
            type = "u8"
            address = { base = "module+0x100", offsets = [0x18, 0x20] }

            [[scans]]
            id = "souls"
            type = "u32"
            start = "module+0x200"
            bytes = 64
            values = [1, 2]
            "#,
        )
        .expect("config");

        assert!(config.enabled);
        assert_eq!(config.watches.len(), 1);
        assert_eq!(config.scans.len(), 1);
        assert_eq!(
            config.scans[0].values,
            vec![TargetValue::Integer(1), TargetValue::Integer(2)]
        );
    }
}
