use std::{
    env, fs,
    io::{BufWriter, Write},
    path::PathBuf,
    process,
};

use plugin_sdk::linkdata::{LinkDataArchive, LinkDataEntrySections};
use serde::{Deserialize, Serialize};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    match Args::parse(env::args().skip(1).collect())? {
        Args::Dump(args) => run_dump(args),
        Args::Rows(args) => run_rows(args),
        Args::Schema(args) => run_schema(args),
        Args::Decode(args) => run_decode(args),
    }
}

fn run_dump(args: DumpArgs) -> Result<(), String> {
    let bytes = fs::read(&args.linkdata_path).map_err(|error| {
        format!(
            "failed to read LINKDATA path={} error={error}",
            args.linkdata_path.display()
        )
    })?;
    let archive = LinkDataArchive::parse(bytes)
        .map_err(|error| format!("failed to parse LINKDATA archive: {error}"))?;

    fs::create_dir_all(&args.out_dir).map_err(|error| {
        format!(
            "failed to create output dir path={} error={error}",
            args.out_dir.display()
        )
    })?;
    let payload_dir = args.out_dir.join("payloads");
    fs::create_dir_all(&payload_dir).map_err(|error| {
        format!(
            "failed to create payload dir path={} error={error}",
            payload_dir.display()
        )
    })?;

    write_manifest(&args, archive.entries().len())?;
    write_entries(&args, &payload_dir, &archive)?;
    Ok(())
}

fn run_schema(args: SchemaArgs) -> Result<(), String> {
    let mut offset = 0;
    let mut fields = Vec::new();
    for (index, field_type) in args.fields.iter().copied().enumerate() {
        fields.push(FieldSchema {
            id: format!(
                "linkdata.{}.entry_{}.v1:field:0x{offset:04x}:{}",
                args.file_id,
                args.entry,
                field_type.name()
            ),
            name: format!("unknown_{}_{index:02}", field_type.name()),
            label: format!(
                "Unknown {} {index:02}",
                field_type.name().to_ascii_uppercase()
            ),
            offset,
            field_type,
            status: "unknown".to_string(),
            confidence: "observed".to_string(),
            previous_names: Vec::new(),
        });
        offset += field_type.size();
    }

    if offset > args.record_size {
        return Err(format!(
            "field layout is larger than record size: fields={offset} record_size={}",
            args.record_size
        ));
    }

    let schema = RecordSchema {
        kind: "linkdata_record_schema".to_string(),
        id: format!("linkdata.{}.entry_{}.v1", args.file_id, args.entry),
        file_id: args.file_id,
        entry: args.entry,
        label: args
            .label
            .unwrap_or_else(|| format!("LINKDATA entry {}", args.entry)),
        record_size: args.record_size,
        fields,
    };
    write_json_or_stdout(args.out_path, &schema)
}

fn run_decode(args: DecodeArgs) -> Result<(), String> {
    let payload = fs::read(&args.payload_path).map_err(|error| {
        format!(
            "failed to read payload path={} error={error}",
            args.payload_path.display()
        )
    })?;
    let schema_bytes = fs::read(&args.schema_path).map_err(|error| {
        format!(
            "failed to read schema path={} error={error}",
            args.schema_path.display()
        )
    })?;
    let schema: RecordSchema = serde_json::from_slice(&schema_bytes).map_err(|error| {
        format!(
            "failed to parse schema path={} error={error}",
            args.schema_path.display()
        )
    })?;
    validate_schema(&schema)?;

    let row_count = payload.len() / schema.record_size;
    let remainder = payload.len() % schema.record_size;
    let limit = args.limit.unwrap_or(row_count).min(row_count);
    let start = args.start.min(row_count);
    let end = start.saturating_add(limit).min(row_count);
    let rows = (start..end)
        .map(|row| {
            let bytes = &payload[row * schema.record_size..(row + 1) * schema.record_size];
            DecodedRow {
                row,
                offset: row * schema.record_size,
                fields: schema
                    .fields
                    .iter()
                    .map(|field| DecodedField {
                        name: field.name.clone(),
                        offset: field.offset,
                        field_type: field.field_type,
                        value: decode_value(bytes, field),
                    })
                    .collect(),
            }
        })
        .collect();
    let dump = DecodedRowsDump {
        kind: "linkdata_decoded_rows".to_string(),
        schema_id: schema.id,
        payload_path: args.payload_path.display().to_string(),
        payload_size: payload.len(),
        record_size: schema.record_size,
        row_count,
        remainder,
        start,
        limit: end.saturating_sub(start),
        rows,
    };
    write_json_or_stdout(args.out_path, &dump)
}

fn run_rows(args: RowsArgs) -> Result<(), String> {
    let payload = fs::read(&args.payload_path).map_err(|error| {
        format!(
            "failed to read payload path={} error={error}",
            args.payload_path.display()
        )
    })?;
    if args.record_size == 0 {
        return Err("record size must be greater than 0".to_string());
    }

    let row_count = payload.len() / args.record_size;
    let remainder = payload.len() % args.record_size;
    let limit = args.limit.unwrap_or(row_count).min(row_count);
    let start = args.start.min(row_count);
    let end = start.saturating_add(limit).min(row_count);
    let dump = RowsDump {
        kind: "linkdata_rows_inspection",
        payload_path: args.payload_path.display().to_string(),
        payload_size: payload.len(),
        record_size: args.record_size,
        row_count,
        remainder,
        start,
        limit: end.saturating_sub(start),
        rows: (start..end)
            .map(|row| {
                row_dump(
                    row,
                    &payload[row * args.record_size..(row + 1) * args.record_size],
                )
            })
            .collect(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&dump)
            .map_err(|error| format!("failed to serialize rows: {error}"))?
    );
    Ok(())
}

#[derive(Clone, Debug)]
enum Args {
    Dump(DumpArgs),
    Rows(RowsArgs),
    Schema(SchemaArgs),
    Decode(DecodeArgs),
}

#[derive(Clone, Debug)]
struct DumpArgs {
    linkdata_path: PathBuf,
    out_dir: PathBuf,
    file_id: String,
}

impl Args {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        if raw.is_empty() {
            return Err(usage());
        }

        let mut args = raw.into_iter();
        if args.as_slice().first().is_some_and(|value| value == "rows") {
            args.next();
            return RowsArgs::parse(args.collect()).map(Self::Rows);
        }
        if args
            .as_slice()
            .first()
            .is_some_and(|value| value == "schema")
        {
            args.next();
            return SchemaArgs::parse(args.collect()).map(Self::Schema);
        }
        if args
            .as_slice()
            .first()
            .is_some_and(|value| value == "decode")
        {
            args.next();
            return DecodeArgs::parse(args.collect()).map(Self::Decode);
        }

        let linkdata_path = PathBuf::from(args.next().unwrap());
        let mut out_dir = PathBuf::from("linkdata_dump");
        let mut file_id = "LINKDATA_A".to_string();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--out" | "-o" => {
                    let Some(value) = args.next() else {
                        return Err(usage());
                    };
                    out_dir = PathBuf::from(value);
                }
                "--file-id" => {
                    let Some(value) = args.next() else {
                        return Err(usage());
                    };
                    file_id = value;
                }
                _ => return Err(usage()),
            }
        }

        Ok(Self::Dump(DumpArgs {
            linkdata_path,
            out_dir,
            file_id,
        }))
    }
}

#[derive(Clone, Debug)]
struct SchemaArgs {
    file_id: String,
    entry: u32,
    label: Option<String>,
    record_size: usize,
    fields: Vec<FieldType>,
    out_path: Option<PathBuf>,
}

impl SchemaArgs {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        let mut args = raw.into_iter();
        let mut file_id = "LINKDATA_A".to_string();
        let mut entry = None;
        let mut label = None;
        let mut record_size = None;
        let mut fields = None;
        let mut out_path = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--file-id" => {
                    file_id = args.next().ok_or_else(usage)?;
                }
                "--entry" | "-e" => {
                    let value = args.next().ok_or_else(usage)?;
                    entry = Some(
                        value
                            .parse::<u32>()
                            .map_err(|_| format!("invalid entry: {value}"))?,
                    );
                }
                "--label" => {
                    label = Some(args.next().ok_or_else(usage)?);
                }
                "--record-size" | "-r" => {
                    let value = args.next().ok_or_else(usage)?;
                    record_size = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("invalid record size: {value}"))?,
                    );
                }
                "--fields" | "-f" => {
                    fields = Some(parse_field_layout(&args.next().ok_or_else(usage)?)?);
                }
                "--out" | "-o" => {
                    out_path = Some(PathBuf::from(args.next().ok_or_else(usage)?));
                }
                _ => return Err(usage()),
            }
        }

        Ok(Self {
            file_id,
            entry: entry.ok_or_else(usage)?,
            label,
            record_size: record_size.ok_or_else(usage)?,
            fields: fields.ok_or_else(usage)?,
            out_path,
        })
    }
}

#[derive(Clone, Debug)]
struct DecodeArgs {
    payload_path: PathBuf,
    schema_path: PathBuf,
    start: usize,
    limit: Option<usize>,
    out_path: Option<PathBuf>,
}

impl DecodeArgs {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        if raw.is_empty() {
            return Err(usage());
        }
        let mut args = raw.into_iter();
        let payload_path = PathBuf::from(args.next().unwrap());
        let mut schema_path = None;
        let mut start = 0;
        let mut limit = None;
        let mut out_path = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--schema" | "-s" => {
                    schema_path = Some(PathBuf::from(args.next().ok_or_else(usage)?));
                }
                "--start" => {
                    let value = args.next().ok_or_else(usage)?;
                    start = value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid start row: {value}"))?;
                }
                "--limit" | "-n" => {
                    let value = args.next().ok_or_else(usage)?;
                    limit = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("invalid row limit: {value}"))?,
                    );
                }
                "--out" | "-o" => {
                    out_path = Some(PathBuf::from(args.next().ok_or_else(usage)?));
                }
                _ => return Err(usage()),
            }
        }

        Ok(Self {
            payload_path,
            schema_path: schema_path.ok_or_else(usage)?,
            start,
            limit,
            out_path,
        })
    }
}

#[derive(Clone, Debug)]
struct RowsArgs {
    payload_path: PathBuf,
    record_size: usize,
    start: usize,
    limit: Option<usize>,
}

impl RowsArgs {
    fn parse(raw: Vec<String>) -> Result<Self, String> {
        if raw.is_empty() {
            return Err(usage());
        }

        let mut args = raw.into_iter();
        let payload_path = PathBuf::from(args.next().unwrap());
        let mut record_size = None;
        let mut start = 0;
        let mut limit = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--record-size" | "-r" => {
                    let Some(value) = args.next() else {
                        return Err(usage());
                    };
                    record_size = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("invalid record size: {value}"))?,
                    );
                }
                "--start" => {
                    let Some(value) = args.next() else {
                        return Err(usage());
                    };
                    start = value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid start row: {value}"))?;
                }
                "--limit" | "-n" => {
                    let Some(value) = args.next() else {
                        return Err(usage());
                    };
                    limit = Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| format!("invalid row limit: {value}"))?,
                    );
                }
                _ => return Err(usage()),
            }
        }

        Ok(Self {
            payload_path,
            record_size: record_size.ok_or_else(usage)?,
            start,
            limit,
        })
    }
}

fn usage() -> String {
    [
        "usage:",
        "  linkdata-dump <LINKDATA_A.BIN> [--out dump-dir] [--file-id LINKDATA_A]",
        "  linkdata-dump rows <payload.bin> --record-size bytes [--start row] [--limit count]",
        "  linkdata-dump schema --entry id --record-size bytes --fields layout [--label text] [--out file]",
        "  linkdata-dump decode <payload.bin> --schema file [--start row] [--limit count] [--out file]",
        "",
        "field layout examples: u32x10,f32x2 or u16,u16,u32,f32",
    ]
    .join("\n")
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RecordSchema {
    kind: String,
    id: String,
    file_id: String,
    entry: u32,
    label: String,
    record_size: usize,
    fields: Vec<FieldSchema>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FieldSchema {
    id: String,
    name: String,
    label: String,
    offset: usize,
    #[serde(rename = "type")]
    field_type: FieldType,
    status: String,
    confidence: String,
    previous_names: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum FieldType {
    U8,
    U16,
    U32,
    I32,
    F32,
}

impl FieldType {
    fn parse(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "i32" => Some(Self::I32),
            "f32" => Some(Self::F32),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::I32 => "i32",
            Self::F32 => "f32",
        }
    }

    fn size(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
        }
    }
}

fn parse_field_layout(raw: &str) -> Result<Vec<FieldType>, String> {
    let mut fields = Vec::new();
    for part in raw.split(',').filter(|part| !part.trim().is_empty()) {
        let part = part.trim();
        let (field_type, count) = match part.split_once('x') {
            Some((kind, count)) => {
                let parsed_count = count
                    .parse::<usize>()
                    .map_err(|_| format!("invalid field count in layout part: {part}"))?;
                (kind, parsed_count)
            }
            None => (part, 1),
        };
        let field_type = FieldType::parse(field_type)
            .ok_or_else(|| format!("unknown field type in layout part: {part}"))?;
        fields.extend(std::iter::repeat(field_type).take(count));
    }
    if fields.is_empty() {
        return Err("field layout must not be empty".to_string());
    }
    Ok(fields)
}

#[derive(Serialize)]
struct Manifest<'a> {
    kind: &'static str,
    file_id: &'a str,
    source_path: String,
    entry_count: usize,
}

fn write_manifest(args: &DumpArgs, entry_count: usize) -> Result<(), String> {
    let manifest = Manifest {
        kind: "linkdata_dump",
        file_id: &args.file_id,
        source_path: args.linkdata_path.display().to_string(),
        entry_count,
    };
    let path = args.out_dir.join("manifest.json");
    write_pretty_json(path, &manifest)
}

#[derive(Serialize)]
struct EntryDump<'a> {
    kind: &'static str,
    id: String,
    file_id: &'a str,
    entry: u32,
    table_offset: usize,
    data_offset: usize,
    field_04: u32,
    compressed_span: usize,
    uncompressed_size: usize,
    payload_size: usize,
    payload_path: String,
    sections: SectionsDump,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SectionsDump {
    Parsed {
        section_count: usize,
        sections: Vec<SectionDump>,
    },
    Unparsed {
        reason: String,
    },
}

#[derive(Serialize)]
struct SectionDump {
    index: usize,
    size: usize,
}

fn write_entries(
    args: &DumpArgs,
    payload_dir: &std::path::Path,
    archive: &LinkDataArchive,
) -> Result<(), String> {
    let path = args.out_dir.join("entries.ndjson");
    let file = fs::File::create(&path)
        .map_err(|error| format!("failed to create path={} error={error}", path.display()))?;
    let mut writer = BufWriter::new(file);

    for entry in archive.entries() {
        let entry_id = entry.id.get();
        let payload = archive
            .entry_payload(entry.id)
            .map_err(|error| format!("failed to inflate entry={entry_id}: {error}"))?;
        let payload_path = payload_dir.join(format!("entry_{entry_id:04}.bin"));
        fs::write(&payload_path, &payload).map_err(|error| {
            format!(
                "failed to write payload path={} error={error}",
                payload_path.display()
            )
        })?;

        let dump = EntryDump {
            kind: "linkdata_entry",
            id: format!("linkdata:{}:{entry_id}", args.file_id),
            file_id: &args.file_id,
            entry: entry_id,
            table_offset: entry.table_offset,
            data_offset: entry.data_offset,
            field_04: entry.field_04,
            compressed_span: entry.compressed_span,
            uncompressed_size: entry.uncompressed_size,
            payload_size: payload.len(),
            payload_path: format!("payloads/entry_{entry_id:04}.bin"),
            sections: sections_dump(&payload),
        };
        serde_json::to_writer(&mut writer, &dump)
            .map_err(|error| format!("failed to serialize entry={entry_id}: {error}"))?;
        writer
            .write_all(b"\n")
            .map_err(|error| format!("failed to write entries ndjson: {error}"))?;
    }

    writer
        .flush()
        .map_err(|error| format!("failed to flush entries ndjson: {error}"))
}

#[derive(Serialize)]
struct RowsDump {
    kind: &'static str,
    payload_path: String,
    payload_size: usize,
    record_size: usize,
    row_count: usize,
    remainder: usize,
    start: usize,
    limit: usize,
    rows: Vec<RowDump>,
}

#[derive(Serialize)]
struct RowDump {
    row: usize,
    offset: usize,
    bytes_hex: String,
    u8: Vec<u8>,
    u16: Vec<u16>,
    u32: Vec<u32>,
    f32: Vec<Option<f32>>,
}

fn row_dump(row: usize, bytes: &[u8]) -> RowDump {
    RowDump {
        row,
        offset: row * bytes.len(),
        bytes_hex: bytes_hex(bytes),
        u8: bytes.to_vec(),
        u16: bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
            .collect(),
        u32: bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect(),
        f32: bytes
            .chunks_exact(4)
            .map(|chunk| {
                let value = f32::from_bits(u32::from_le_bytes(chunk.try_into().unwrap()));
                value.is_finite().then_some(value)
            })
            .collect(),
    }
}

#[derive(Serialize)]
struct DecodedRowsDump {
    kind: String,
    schema_id: String,
    payload_path: String,
    payload_size: usize,
    record_size: usize,
    row_count: usize,
    remainder: usize,
    start: usize,
    limit: usize,
    rows: Vec<DecodedRow>,
}

#[derive(Serialize)]
struct DecodedRow {
    row: usize,
    offset: usize,
    fields: Vec<DecodedField>,
}

#[derive(Serialize)]
struct DecodedField {
    name: String,
    offset: usize,
    #[serde(rename = "type")]
    field_type: FieldType,
    value: serde_json::Value,
}

fn validate_schema(schema: &RecordSchema) -> Result<(), String> {
    if schema.record_size == 0 {
        return Err("schema record_size must be greater than 0".to_string());
    }
    for field in &schema.fields {
        let end = field.offset + field.field_type.size();
        if end > schema.record_size {
            return Err(format!(
                "field out of record bounds name={} offset={} type={} record_size={}",
                field.name,
                field.offset,
                field.field_type.name(),
                schema.record_size
            ));
        }
    }
    Ok(())
}

fn decode_value(record: &[u8], field: &FieldSchema) -> serde_json::Value {
    let offset = field.offset;
    match field.field_type {
        FieldType::U8 => record
            .get(offset)
            .copied()
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
        FieldType::U16 => record
            .get(offset..offset + 2)
            .map(|raw| serde_json::Value::from(u16::from_le_bytes(raw.try_into().unwrap())))
            .unwrap_or(serde_json::Value::Null),
        FieldType::U32 => record
            .get(offset..offset + 4)
            .map(|raw| serde_json::Value::from(u32::from_le_bytes(raw.try_into().unwrap())))
            .unwrap_or(serde_json::Value::Null),
        FieldType::I32 => record
            .get(offset..offset + 4)
            .map(|raw| serde_json::Value::from(i32::from_le_bytes(raw.try_into().unwrap())))
            .unwrap_or(serde_json::Value::Null),
        FieldType::F32 => record
            .get(offset..offset + 4)
            .and_then(|raw| {
                let value = f32::from_le_bytes(raw.try_into().unwrap());
                value.is_finite().then_some(value)
            })
            .and_then(|value| serde_json::Number::from_f64(f64::from(value)))
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
    }
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn sections_dump(payload: &[u8]) -> SectionsDump {
    if payload.len() < 4 {
        return SectionsDump::Unparsed {
            reason: "unknown_layout: payload smaller than section header".to_string(),
        };
    }
    let Some(count) = read_u32(payload, 0).map(|value| value as usize) else {
        return SectionsDump::Unparsed {
            reason: "unknown_layout: missing section count".to_string(),
        };
    };
    let table_end = 4 + count.saturating_mul(4);
    if count == 0 || count > 128 || table_end > payload.len() {
        return SectionsDump::Unparsed {
            reason: "unknown_layout: no valid section table header".to_string(),
        };
    }

    match LinkDataEntrySections::parse(payload) {
        Ok(sections) => SectionsDump::Parsed {
            section_count: sections.section_count(),
            sections: (0..sections.section_count())
                .map(|index| SectionDump {
                    index,
                    size: sections.section(index).map_or(0, <[u8]>::len),
                })
                .collect(),
        },
        Err(error) => SectionsDump::Unparsed {
            reason: format!("unknown_layout: {error}"),
        },
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let raw = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(raw.try_into().ok()?))
}

fn write_pretty_json(path: PathBuf, value: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize json: {error}"))?;
    fs::write(&path, format!("{json}\n"))
        .map_err(|error| format!("failed to write path={} error={error}", path.display()))
}

fn write_json_or_stdout(path: Option<PathBuf>, value: &impl Serialize) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize json: {error}"))?;
    match path {
        Some(path) => fs::write(&path, format!("{json}\n"))
            .map_err(|error| format!("failed to write path={} error={error}", path.display())),
        None => {
            println!("{json}");
            Ok(())
        }
    }
}
