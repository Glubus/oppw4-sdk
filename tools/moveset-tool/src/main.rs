use std::{env, fs, io::Read, path::PathBuf, process};

use flate2::read::ZlibDecoder;

const LINKDATA_MAGIC: u32 = 0x0007_7df9;
const TABLE_OFFSET: usize = 0x10;
const RECORD_SIZE: usize = 0x10;
const OFFSET_GRANULARITY: usize = 0x100;
const MOVESET_RECORD_SIZES_18: [usize; 18] = [
    16, 96, 32, 32, 16, 64, 32, 64, 0, 64, 48, 0, 48, 32, 32, 16, 16, 16,
];

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1).collect())?;
    let linkdata = fs::read(&args.linkdata_path).map_err(|error| {
        format!(
            "failed to read LINKDATA path={} error={error}",
            args.linkdata_path.display()
        )
    })?;
    let payload = extract_entry(&linkdata, args.entry)?;
    let output = match args.format {
        Format::Bin => Output::Bytes(payload),
        Format::Hex => Output::Text(to_hex_text(&payload)),
        Format::Json => Output::Text(to_structured_text(
            &payload,
            args.entry,
            Syntax::Json,
            !args.typed_words,
        )?),
        Format::Lua => Output::Text(to_structured_text(
            &payload,
            args.entry,
            Syntax::Lua,
            !args.typed_words,
        )?),
    };
    write_output(output, args.out_path)
}

#[derive(Clone, Debug)]
struct Args {
    linkdata_path: PathBuf,
    entry: usize,
    format: Format,
    out_path: Option<PathBuf>,
    typed_words: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Bin,
    Hex,
    Json,
    Lua,
}

enum Output {
    Bytes(Vec<u8>),
    Text(String),
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        if raw.len() < 2 {
            return Err(usage());
        }
        let mut args = raw.into_iter();
        let linkdata_path = PathBuf::from(args.next().unwrap());
        let entry = args.next().unwrap().parse::<usize>().map_err(|_| usage())?;
        let mut format = Format::Json;
        let mut out_path = None;
        let mut typed_words = false;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--format" | "-f" => {
                    let Some(value) = args.next() else {
                        return Err(usage());
                    };
                    format = parse_format(&value)?;
                }
                "--out" | "-o" => {
                    let Some(value) = args.next() else {
                        return Err(usage());
                    };
                    out_path = Some(PathBuf::from(value));
                }
                "--hex-words" => typed_words = false,
                "--typed-words" => typed_words = true,
                _ => return Err(usage()),
            }
        }

        Ok(Self {
            linkdata_path,
            entry,
            format,
            out_path,
            typed_words,
        })
    }
}

fn parse_format(raw: &str) -> Result<Format, String> {
    match raw.to_ascii_lowercase().as_str() {
        "bin" | "raw" => Ok(Format::Bin),
        "hex" => Ok(Format::Hex),
        "json" => Ok(Format::Json),
        "lua" => Ok(Format::Lua),
        _ => Err(usage()),
    }
}

fn usage() -> String {
    "usage: moveset-dump <LINKDATA_A.BIN> <entry-id> [--format bin|hex|json|lua] [--out file] [--typed-words]".to_string()
}

#[derive(Clone, Debug)]
struct LinkDataEntry {
    index: usize,
    data_offset: usize,
    compressed_span: usize,
    uncompressed_size: usize,
}

fn extract_entry(bytes: &[u8], target: usize) -> Result<Vec<u8>, String> {
    let entries = parse_linkdata(bytes)?;
    let entry = entries
        .get(target)
        .ok_or_else(|| format!("entry {target} out of bounds, count={}", entries.len()))?;
    inflate_entry(bytes, entry)
}

fn parse_linkdata(bytes: &[u8]) -> Result<Vec<LinkDataEntry>, String> {
    if bytes.len() < TABLE_OFFSET || read_u32(bytes, 0)? != LINKDATA_MAGIC {
        return Err("invalid LINKDATA header".to_string());
    }
    let count = read_u32(bytes, 4)? as usize;
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let table_offset = TABLE_OFFSET + index * RECORD_SIZE;
        if table_offset + RECORD_SIZE > bytes.len() {
            return Err(format!("truncated LINKDATA table at entry {index}"));
        }
        entries.push(LinkDataEntry {
            index,
            data_offset: read_u32(bytes, table_offset)? as usize * OFFSET_GRANULARITY,
            compressed_span: read_u32(bytes, table_offset + 0x08)? as usize,
            uncompressed_size: read_u32(bytes, table_offset + 0x0c)? as usize,
        });
    }
    Ok(entries)
}

fn inflate_entry(bytes: &[u8], entry: &LinkDataEntry) -> Result<Vec<u8>, String> {
    if entry.uncompressed_size == 0 {
        let end = entry.data_offset + entry.compressed_span;
        return bytes
            .get(entry.data_offset..end)
            .map(|payload| payload.to_vec())
            .ok_or_else(|| format!("entry {} out of bounds", entry.index));
    }

    let block_uncompressed = read_u32(bytes, entry.data_offset)? as usize;
    let expected = entry.uncompressed_size.min(block_uncompressed);
    let mut cursor = entry.data_offset + 4;
    let mut output = Vec::with_capacity(expected);
    while output.len() < expected {
        let compressed_size = read_u32(bytes, cursor)? as usize;
        cursor += 4;
        let end = cursor + compressed_size;
        let chunk = bytes
            .get(cursor..end)
            .ok_or_else(|| format!("entry {} truncated zlib chunk", entry.index))?;
        ZlibDecoder::new(chunk)
            .read_to_end(&mut output)
            .map_err(|error| format!("entry {} inflate failed: {error}", entry.index))?;
        cursor = end;
    }
    output.truncate(expected);
    Ok(output)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Syntax {
    Json,
    Lua,
}

fn to_structured_text(
    payload: &[u8],
    entry: usize,
    syntax: Syntax,
    hex_words: bool,
) -> Result<String, String> {
    let sections = parse_sections(payload)?;
    let mut out = String::new();
    match syntax {
        Syntax::Json => {
            out.push_str("{\n");
            out.push_str(&format!("  \"entry\": {entry},\n"));
            out.push_str(&format!("  \"section_count\": {},\n", sections.len()));
            out.push_str("  \"sections\": [\n");
            write_sections(&mut out, payload, &sections, syntax, hex_words);
            out.push_str("  ]\n}\n");
        }
        Syntax::Lua => {
            out.push_str("return {\n");
            out.push_str(&format!("  entry = {entry},\n"));
            out.push_str(&format!("  section_count = {},\n", sections.len()));
            out.push_str("  sections = {\n");
            write_sections(&mut out, payload, &sections, syntax, hex_words);
            out.push_str("  },\n}\n");
        }
    }
    Ok(out)
}

#[derive(Clone, Debug)]
struct Section {
    index: usize,
    start: usize,
    end: usize,
    record_size: usize,
}

fn parse_sections(payload: &[u8]) -> Result<Vec<Section>, String> {
    let count = read_u32(payload, 0)? as usize;
    if count == 0 || count > 128 || 4 + count * 4 > payload.len() {
        return Ok(vec![Section {
            index: 0,
            start: 0,
            end: payload.len(),
            record_size: 0,
        }]);
    }

    let mut offsets = Vec::with_capacity(count);
    for index in 0..count {
        let offset = read_u32(payload, 4 + index * 4)? as usize;
        if offset > payload.len() {
            return Err(format!(
                "section {index} offset out of bounds: 0x{offset:x}"
            ));
        }
        offsets.push(offset);
    }

    let record_sizes = if count == MOVESET_RECORD_SIZES_18.len() {
        MOVESET_RECORD_SIZES_18.as_slice()
    } else {
        &[]
    };
    let mut sections = Vec::with_capacity(count);
    for index in 0..count {
        let start = offsets[index];
        let end = offsets
            .iter()
            .copied()
            .filter(|offset| *offset > start)
            .min()
            .unwrap_or(payload.len())
            .min(payload.len());
        let record_size = record_sizes.get(index).copied().unwrap_or(0);
        sections.push(Section {
            index,
            start,
            end,
            record_size,
        });
    }
    Ok(sections)
}

fn write_sections(
    out: &mut String,
    payload: &[u8],
    sections: &[Section],
    syntax: Syntax,
    hex_words: bool,
) {
    for (position, section) in sections.iter().enumerate() {
        let last_section = position + 1 == sections.len();
        match syntax {
            Syntax::Json => {
                out.push_str("    {\n");
                out.push_str(&format!("      \"index\": {},\n", section.index));
                if uses_records(payload, section) {
                    out.push_str(&format!(
                        "      \"record_size\": {},\n",
                        section.record_size
                    ));
                    out.push_str("      \"records\": [\n");
                } else {
                    out.push_str("      \"words\": [\n");
                }
            }
            Syntax::Lua => {
                if uses_records(payload, section) {
                    out.push_str(&format!(
                        "    {{ index = {}, record_size = {}, records = {{\n",
                        section.index, section.record_size
                    ));
                } else {
                    out.push_str(&format!("    {{ index = {}, words = {{\n", section.index));
                }
            }
        }
        write_records(
            out,
            &payload[section.start..section.end],
            record_size_for_output(payload, section),
            syntax,
            hex_words,
        );
        match syntax {
            Syntax::Json => {
                out.push_str("      ]\n");
                out.push_str(if last_section { "    }\n" } else { "    },\n" });
            }
            Syntax::Lua => {
                out.push_str(if last_section {
                    "    } }\n"
                } else {
                    "    } },\n"
                });
            }
        }
    }
}

fn uses_records(_payload: &[u8], section: &Section) -> bool {
    let len = section.end.saturating_sub(section.start);
    section.record_size > 0 && len > 0 && len.is_multiple_of(section.record_size)
}

fn record_size_for_output(payload: &[u8], section: &Section) -> usize {
    if uses_records(payload, section) {
        section.record_size
    } else {
        0
    }
}

fn write_records(
    out: &mut String,
    bytes: &[u8],
    record_size: usize,
    syntax: Syntax,
    hex_words: bool,
) {
    let size = if record_size > 0 && bytes.len().is_multiple_of(record_size) {
        record_size
    } else {
        bytes.len().max(4)
    };
    for (index, record) in bytes.chunks(size).enumerate() {
        let last = (index + 1) * size >= bytes.len();
        let words = record
            .chunks(4)
            .filter(|chunk| chunk.len() == 4)
            .map(|chunk| {
                format_word(
                    u32::from_le_bytes(chunk.try_into().unwrap()),
                    syntax,
                    hex_words,
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        match syntax {
            Syntax::Json => out.push_str(&format!(
                "        [{}]{}\n",
                words,
                if last { "" } else { "," }
            )),
            Syntax::Lua => out.push_str(&format!(
                "      {{ {} }}{}\n",
                words,
                if last { "" } else { "," }
            )),
        }
    }
}

fn format_word(value: u32, syntax: Syntax, hex_words: bool) -> String {
    if hex_words {
        return match syntax {
            Syntax::Json => format!("\"0x{value:08x}\""),
            Syntax::Lua => format!("0x{value:08x}"),
        };
    }
    let object = match value {
        0 => return "0".to_string(),
        0xffff_ffff => typed_word(syntax, "i32", "-1", None),
        0xbf80_0000 => typed_word(syntax, "f32", "-1.0", None),
        0x3f80_0000 => typed_word(syntax, "f32", "1.0", None),
        0x42c8_0000 => typed_word(syntax, "f32", "100.0", None),
        value if value < 1_000_000 => typed_word(syntax, "u32", &value.to_string(), None),
        value => {
            let float = f32::from_bits(value);
            if float.is_finite() && float.abs() >= 0.001 && float.abs() <= 100_000.0 {
                typed_word(
                    syntax,
                    "f32",
                    &format_float(float),
                    Some(format!("0x{value:08x}")),
                )
            } else {
                typed_word(syntax, "hex", &format!("\"0x{value:08x}\""), None)
            }
        }
    };
    object
}

fn typed_word(syntax: Syntax, key: &str, value: &str, hex: Option<String>) -> String {
    match syntax {
        Syntax::Json => match hex {
            Some(hex) => format!("{{\"{key}\":{value},\"hex\":\"{hex}\"}}"),
            None => format!("{{\"{key}\":{value}}}"),
        },
        Syntax::Lua => match hex {
            Some(hex) => format!("{{ {key} = {value}, hex = \"{hex}\" }}"),
            None => format!("{{ {key} = {value} }}"),
        },
    }
}

fn format_float(value: f32) -> String {
    let text = format!("{value:?}");
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

fn to_hex_text(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            if index % 16 == 0 {
                out.push('\n');
            } else {
                out.push(' ');
            }
        }
        out.push_str(&format!("{byte:02x}"));
    }
    out.push('\n');
    out
}

fn write_output(output: Output, out_path: Option<PathBuf>) -> Result<(), String> {
    match (output, out_path) {
        (Output::Bytes(bytes), Some(path)) => fs::write(&path, bytes)
            .map_err(|error| format!("failed to write path={} error={error}", path.display())),
        (Output::Text(text), Some(path)) => fs::write(&path, text)
            .map_err(|error| format!("failed to write path={} error={error}", path.display())),
        (Output::Text(text), None) => {
            print!("{text}");
            Ok(())
        }
        (Output::Bytes(_), None) => Err("binary output requires --out".to_string()),
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let Some(raw) = bytes.get(offset..offset + 4) else {
        return Err(format!("read_u32 out of bounds at 0x{offset:x}"));
    };
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}
