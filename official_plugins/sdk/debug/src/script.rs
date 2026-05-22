use mlua::{Lua, Table, Value};

use crate::model::{AddressSpec, DebugConfig, Scan, TargetValue, ValueType, Watch};

pub(crate) fn parse(text: &str) -> Result<DebugConfig, String> {
    let lua = Lua::new();
    let value = lua
        .load(text)
        .set_name("sdk_debug/debug.lua")
        .eval::<Value>()
        .map_err(|error| error.to_string())?;
    let table = match value {
        Value::Table(table) => table,
        _ => return Err("debug.lua must return a table".to_string()),
    };

    Ok(DebugConfig {
        enabled: table.get("enabled").unwrap_or(false),
        interval_ms: table
            .get::<Option<u64>>("interval_ms")
            .unwrap_or(None)
            .unwrap_or(500),
        watches: parse_watches(table.get::<Option<Table>>("watches").unwrap_or(None))?,
        scans: parse_scans(table.get::<Option<Table>>("scans").unwrap_or(None))?,
    })
}

fn parse_watches(table: Option<Table>) -> Result<Vec<Watch>, String> {
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    let mut watches = Vec::new();
    for pair in table.sequence_values::<Table>() {
        let row = pair.map_err(|error| error.to_string())?;
        watches.push(Watch {
            id: required_string(&row, "id")?,
            value_type: parse_type(&required_string(&row, "type")?)?,
            address: parse_address(row.get("address").map_err(|error| error.to_string())?)?,
        });
    }
    Ok(watches)
}

fn parse_scans(table: Option<Table>) -> Result<Vec<Scan>, String> {
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    let mut scans = Vec::new();
    for pair in table.sequence_values::<Table>() {
        let row = pair.map_err(|error| error.to_string())?;
        let value_type = parse_type(&required_string(&row, "type")?)?;
        scans.push(Scan {
            id: required_string(&row, "id")?,
            value_type,
            start: parse_address(row.get("start").map_err(|error| error.to_string())?)?,
            bytes: row
                .get::<Option<usize>>("bytes")
                .map_err(|error| error.to_string())?
                .unwrap_or(4096),
            values: parse_values(
                row.get::<Option<Table>>("values").unwrap_or(None),
                value_type,
            )?,
            max_hits: row
                .get::<Option<usize>>("max_hits")
                .map_err(|error| error.to_string())?
                .unwrap_or(32),
        });
    }
    Ok(scans)
}

fn parse_values(table: Option<Table>, value_type: ValueType) -> Result<Vec<TargetValue>, String> {
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    let mut values = Vec::new();
    for value in table.sequence_values::<Value>() {
        let value = value.map_err(|error| error.to_string())?;
        values.push(match (value_type, value) {
            (ValueType::F32, Value::Number(value)) => TargetValue::Float(value as f32),
            (_, Value::Integer(value)) => TargetValue::Integer(value),
            (_, Value::Number(value)) => TargetValue::Integer(value as i64),
            (_, other) => return Err(format!("unsupported scan value: {}", other.type_name())),
        });
    }
    Ok(values)
}

fn parse_address(value: Value) -> Result<AddressSpec, String> {
    match value {
        Value::Integer(value) => usize::try_from(value)
            .map(AddressSpec::Absolute)
            .map_err(|_| "negative address is invalid".to_string()),
        Value::String(value) => parse_address_string(&value.to_string_lossy()),
        Value::Table(table) => {
            let base = parse_address(table.get("base").map_err(|error| error.to_string())?)?;
            let offsets = parse_offsets(table.get::<Option<Table>>("offsets").unwrap_or(None))?;
            Ok(AddressSpec::PointerChain {
                base: Box::new(base),
                offsets,
            })
        }
        other => Err(format!("unsupported address value: {}", other.type_name())),
    }
}

fn parse_offsets(table: Option<Table>) -> Result<Vec<usize>, String> {
    let Some(table) = table else {
        return Ok(Vec::new());
    };
    table
        .sequence_values::<usize>()
        .map(|value| value.map_err(|error| error.to_string()))
        .collect()
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

fn required_string(table: &Table, key: &str) -> Result<String, String> {
    table
        .get::<Option<String>>(key)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("missing required field: {key}"))
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
            return {
              enabled = true,
              watches = {
                { id = "difficulty", type = "u8", address = { base = "module+0x100", offsets = { 0x18, 0x20 } } },
              },
              scans = {
                { id = "souls", type = "u32", start = "module+0x200", bytes = 64, values = { 1, 2 } },
              },
            }
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
