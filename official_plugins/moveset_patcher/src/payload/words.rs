use mlua::{Table, Value};
use serde::Deserialize;

#[derive(Clone, Copy, Debug)]
pub(super) struct JsonWord(pub(super) u32);

impl<'de> Deserialize<'de> for JsonWord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Number(number) => number
                .as_u64()
                .filter(|value| *value <= u32::MAX as u64)
                .map(|value| JsonWord(value as u32))
                .ok_or_else(|| serde::de::Error::custom("u32 JSON word expected")),
            serde_json::Value::String(text) => parse_word(&text)
                .map(JsonWord)
                .map_err(serde::de::Error::custom),
            serde_json::Value::Object(object) => parse_json_word_object(&object)
                .map(JsonWord)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "JSON word must be a number, hex string, or typed object",
            )),
        }
    }
}

pub(super) fn words_to_bytes(words: Table) -> mlua::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for value in words.sequence_values::<Value>() {
        let value = value?;
        let word = lua_value_to_word(value)?;
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    Ok(bytes)
}

pub(super) fn json_words_to_bytes(words: &[JsonWord]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for word in words {
        bytes.extend_from_slice(&word.0.to_le_bytes());
    }
    bytes
}

fn lua_value_to_word(value: Value) -> mlua::Result<u32> {
    match value {
        Value::Integer(value) if value >= 0 && value <= u32::MAX as i64 => Ok(value as u32),
        Value::Integer(value) if (i32::MIN as i64..=i32::MAX as i64).contains(&value) => {
            Ok((value as i32) as u32)
        }
        Value::Number(value) if value.is_finite() && value >= 0.0 && value <= u32::MAX as f64 => {
            Ok(value as u32)
        }
        Value::String(text) => parse_word(text.to_str()?.as_ref()).map_err(mlua::Error::external),
        Value::Table(table) => lua_table_to_word(table),
        other => Err(mlua::Error::external(format!(
            "u32 word expected, got {}",
            other.type_name()
        ))),
    }
}

fn lua_table_to_word(table: Table) -> mlua::Result<u32> {
    if let Some(value) = table.get::<Option<String>>("hex")? {
        return parse_word(&value).map_err(mlua::Error::external);
    }
    if let Some(value) = table.get::<Option<u32>>("u32")? {
        return Ok(value);
    }
    if let Some(value) = table.get::<Option<i32>>("i32")? {
        return Ok(value as u32);
    }
    if let Some(value) = table.get::<Option<f32>>("f32")? {
        if value.is_finite() {
            return Ok(value.to_bits());
        }
    }
    Err(mlua::Error::external(
        "Lua word object expects hex, u32, i32, or f32",
    ))
}

fn parse_word(text: &str) -> Result<u32, String> {
    let trimmed = text.trim();
    let raw = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"));
    match raw {
        Some(hex) => u32::from_str_radix(hex, 16).map_err(|error| error.to_string()),
        None => trimmed.parse::<u32>().map_err(|error| error.to_string()),
    }
}

fn parse_json_word_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Result<u32, String> {
    if let Some(value) = object.get("hex").and_then(serde_json::Value::as_str) {
        return parse_word(value);
    }
    if let Some(value) = object.get("u32").and_then(serde_json::Value::as_u64) {
        if value <= u32::MAX as u64 {
            return Ok(value as u32);
        }
    }
    if let Some(value) = object.get("i32").and_then(serde_json::Value::as_i64) {
        if (i32::MIN as i64..=i32::MAX as i64).contains(&value) {
            return Ok((value as i32) as u32);
        }
    }
    if let Some(value) = object.get("f32").and_then(serde_json::Value::as_f64) {
        if value.is_finite() && value >= f32::MIN as f64 && value <= f32::MAX as f64 {
            return Ok((value as f32).to_bits());
        }
    }
    Err("JSON word object expects hex, u32, i32, or f32".to_string())
}
