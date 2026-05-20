use mlua::Table;
use serde::Deserialize;

mod words;

use words::{json_words_to_bytes, words_to_bytes, JsonWord};

#[derive(Debug, Deserialize)]
struct JsonMoveset {
    #[allow(dead_code)]
    entry: Option<u16>,
    section_count: Option<usize>,
    sections: Vec<JsonSection>,
}

#[derive(Debug, Deserialize)]
struct JsonSection {
    index: usize,
    record_size: Option<usize>,
    #[serde(default)]
    records: Vec<Vec<JsonWord>>,
    #[serde(default)]
    words: Vec<JsonWord>,
}

pub(crate) fn from_lua_table(table: Table) -> mlua::Result<Vec<u8>> {
    if let Some(payload) = table.get::<Option<mlua::String>>("payload")? {
        return Ok(payload.as_bytes().to_vec());
    }
    if let Some(hex) = table.get::<Option<String>>("payload_hex")? {
        return crate::hex::parse_payload(&hex).map_err(mlua::Error::external);
    }
    if let Some(sections) = table.get::<Option<Table>>("sections")? {
        let section_count = table.get::<Option<usize>>("section_count")?.unwrap_or(18);
        return build_from_sections(section_count, sections);
    }
    Err(mlua::Error::external(
        "moveset payload expects payload, payload_hex, or sections",
    ))
}

pub(crate) fn from_json_str(text: &str) -> Result<Vec<u8>, String> {
    let moveset: JsonMoveset =
        serde_json::from_str(text).map_err(|error| format!("invalid moveset json: {error}"))?;
    build_from_json(moveset)
}

pub(crate) fn json_entry(text: &str) -> Result<Option<u16>, String> {
    let moveset: JsonMoveset =
        serde_json::from_str(text).map_err(|error| format!("invalid moveset json: {error}"))?;
    Ok(moveset.entry)
}

pub(crate) fn build_from_sections(section_count: usize, sections: Table) -> mlua::Result<Vec<u8>> {
    if section_count == 0 || section_count > 1024 {
        return Err(mlua::Error::external("invalid moveset section_count"));
    }

    let mut section_bytes = vec![Vec::<u8>::new(); section_count];
    for value in sections.sequence_values::<Table>() {
        let section = value?;
        let index = section.get::<usize>("index")?;
        if index >= section_count {
            return Err(mlua::Error::external(format!(
                "section index {index} >= section_count {section_count}"
            )));
        }
        section_bytes[index] = build_section(section)?;
    }

    let header_len = align_up(4 + section_count * 4 + 4, 0x10);
    let mut output = vec![0u8; header_len];
    write_u32(&mut output, 0, section_count as u32);

    let mut cursor = header_len;
    for (index, bytes) in section_bytes.iter().enumerate() {
        write_u32(&mut output, 4 + index * 4, cursor as u32);
        output.extend_from_slice(bytes);
        cursor += bytes.len();
        while !output.len().is_multiple_of(0x10) {
            output.push(0);
            cursor += 1;
        }
    }

    Ok(output)
}

fn build_from_json(moveset: JsonMoveset) -> Result<Vec<u8>, String> {
    let section_count = moveset.section_count.unwrap_or(18);
    if section_count == 0 || section_count > 1024 {
        return Err("invalid moveset section_count".to_string());
    }

    let mut section_bytes = vec![Vec::<u8>::new(); section_count];
    for section in moveset.sections {
        let index = section.index;
        if index >= section_count {
            return Err(format!(
                "section index {index} >= section_count {section_count}"
            ));
        }
        section_bytes[index] = build_json_section(section)?;
    }
    Ok(assemble_sections(section_bytes))
}

fn build_json_section(section: JsonSection) -> Result<Vec<u8>, String> {
    if !section.words.is_empty() {
        return Ok(json_words_to_bytes(&section.words));
    }

    let record_size = section
        .record_size
        .ok_or_else(|| "JSON section missing record_size".to_string())?;
    if record_size % 4 != 0 {
        return Err("record_size must be a u32 multiple".to_string());
    }
    let single_raw_record = section.records.len() == 1;
    let mut bytes = Vec::new();
    for record in section.records {
        let mut record_bytes = json_words_to_bytes(&record);
        if record_size == 0 {
            bytes.append(&mut record_bytes);
            continue;
        }
        if record_bytes.len() != record_size {
            if single_raw_record {
                bytes.append(&mut record_bytes);
                continue;
            }
            return Err(format!(
                "record size mismatch expected={record_size} actual={}",
                record_bytes.len()
            ));
        }
        bytes.append(&mut record_bytes);
    }
    Ok(bytes)
}

fn build_section(section: Table) -> mlua::Result<Vec<u8>> {
    if let Some(words) = section.get::<Option<Table>>("words")? {
        return words_to_bytes(words);
    }

    let record_size = section.get::<usize>("record_size")?;
    if record_size % 4 != 0 {
        return Err(mlua::Error::external("record_size must be a u32 multiple"));
    }
    let records = section.get::<Table>("records")?;
    let single_raw_record = records.raw_len() == 1;
    let mut bytes = Vec::new();
    for record in records.sequence_values::<Table>() {
        let record = record?;
        let mut record_bytes = words_to_bytes(record)?;
        if record_size == 0 {
            bytes.append(&mut record_bytes);
            continue;
        }
        if record_bytes.len() != record_size {
            if single_raw_record {
                bytes.append(&mut record_bytes);
                continue;
            }
            return Err(mlua::Error::external(format!(
                "record size mismatch expected={record_size} actual={}",
                record_bytes.len()
            )));
        }
        bytes.append(&mut record_bytes);
    }
    Ok(bytes)
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn assemble_sections(section_bytes: Vec<Vec<u8>>) -> Vec<u8> {
    let section_count = section_bytes.len();
    let header_len = align_up(4 + section_count * 4 + 4, 0x10);
    let mut output = vec![0u8; header_len];
    write_u32(&mut output, 0, section_count as u32);

    let mut cursor = header_len;
    for (index, bytes) in section_bytes.iter().enumerate() {
        write_u32(&mut output, 4 + index * 4, cursor as u32);
        output.extend_from_slice(bytes);
        cursor += bytes.len();
        while !output.len().is_multiple_of(0x10) {
            output.push(0);
            cursor += 1;
        }
    }
    output
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_payload_accepts_hex_words() {
        let bytes = from_json_str(
            r#"
            {
              "entry": 247,
              "section_count": 1,
              "sections": [
                {
                  "index": 0,
                  "record_size": 16,
                  "records": [
                    ["0x00000001", "0x00000002", "0x00000003", "0x00000004"]
                  ]
                }
              ]
            }
            "#,
        )
        .expect("json");

        assert_eq!(&bytes[0..4], 1u32.to_le_bytes());
        assert!(bytes
            .windows(16)
            .any(|window| { window == [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0,] }));
    }

    #[test]
    fn json_payload_accepts_typed_words() {
        let bytes = from_json_str(
            r#"
            {
              "section_count": 1,
              "sections": [
                {
                  "index": 0,
                  "record_size": 16,
                  "records": [
                    [{ "u32": 1 }, { "i32": -1 }, { "f32": -1.0 }, { "hex": "0x3f800000" }]
                  ]
                }
              ]
            }
            "#,
        )
        .expect("json");

        assert!(bytes.windows(16).any(|window| {
            window
                == [
                    1, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0x80, 0xbf, 0, 0, 0x80, 0x3f,
                ]
        }));
    }

    #[test]
    fn lua_payload_accepts_dumped_hex_words() {
        let lua = mlua::Lua::new();
        let table: Table = lua
            .load(
                r#"
                return {
                  section_count = 1,
                  sections = {
                    {
                      index = 0,
                      record_size = 16,
                      records = {
                        { 0x00000001, 0xffffffff, 0xbf800000, 0x3f800000 }
                      }
                    }
                  }
                }
                "#,
            )
            .eval()
            .expect("lua table");

        let bytes = from_lua_table(table).expect("lua payload");
        assert!(bytes.windows(16).any(|window| {
            window
                == [
                    1, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0, 0, 0x80, 0xbf, 0, 0, 0x80, 0x3f,
                ]
        }));
    }
}
