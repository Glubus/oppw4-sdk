use crate::model::{ScanHit, TargetValue, ValueType, WatchValue};

pub(crate) fn watch_value(value: &WatchValue) -> String {
    format!(
        "addr=0x{:x} type={:?} value={} raw={}",
        value.address,
        value.value_type,
        decode_value(value.value_type, &value.bytes),
        hex(&value.bytes),
    )
}

pub(crate) fn scan_hits(hits: &[ScanHit]) -> String {
    if hits.is_empty() {
        return "none".to_string();
    }
    hits.iter()
        .map(|hit| {
            format!(
                "{}@0x{:x}(+0x{:x})",
                target_value(hit.value),
                hit.address,
                hit.offset
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn decode_value(value_type: ValueType, bytes: &[u8]) -> String {
    match value_type {
        ValueType::U8 => bytes[0].to_string(),
        ValueType::U16 => u16::from_le_bytes([bytes[0], bytes[1]]).to_string(),
        ValueType::U32 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string(),
        ValueType::I32 => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string(),
        ValueType::F32 => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).to_string(),
    }
}

fn target_value(value: TargetValue) -> String {
    match value {
        TargetValue::Integer(value) => value.to_string(),
        TargetValue::Float(value) => value.to_string(),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_u32_watch() {
        let value = WatchValue {
            address: 0x1234,
            value_type: ValueType::U32,
            bytes: 42u32.to_le_bytes().to_vec(),
        };

        assert!(watch_value(&value).contains("value=42"));
    }
}
