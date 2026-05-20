use std::{env, fs, path::Path, process};

use rdb::{
    parse_block_tail, parse_name_hash_catalog, parse_payload_tail, parse_prefixed_hex_hash,
    parse_rdb, scan_archive_names_with_catalog, scan_virtualized_names_with_catalog, ArchiveScan,
    NameHashEntry, RdbAddressSuffix, RdbBlock, RdbIndex,
};

struct CliArgs {
    command: Command,
}

enum Command {
    ExportCatalog {
        dll_path: String,
        out_path: String,
    },
    Preview {
        rdb_path: String,
    },
    SearchHash {
        rdb_path: String,
        hash: u32,
    },
    ScanArchive {
        rdb_path: String,
        folder: String,
        catalog_path: Option<String>,
    },
    ScanRoot {
        patcher_root: String,
        rdb_root: String,
        catalog_path: String,
    },
}

fn main() {
    let args = parse_args_or_exit();
    run_command(args.command);
}

fn parse_args_or_exit() -> CliArgs {
    match parse_args(env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    }
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<CliArgs, String> {
    let Some(rdb_path) = args.next() else {
        return Err("usage: rdb-tools <path-to-rdb> [hash]".to_string());
    };

    if rdb_path == "--export-catalog" {
        return parse_export_catalog_command(args).map(|command| CliArgs { command });
    }

    if rdb_path == "--scan-root" {
        return parse_scan_root_command(args).map(|command| CliArgs { command });
    }

    let command = match args.next().as_deref() {
        Some("--scan") => parse_scan_command(rdb_path, args)?,
        Some(raw_hash) => Command::SearchHash {
            rdb_path,
            hash: parse_hash(raw_hash)?,
        },
        None => Command::Preview { rdb_path },
    };

    Ok(CliArgs { command })
}

fn parse_export_catalog_command(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let Some(dll_path) = args.next() else {
        return Err(export_catalog_usage());
    };
    let Some(out_path) = args.next() else {
        return Err(export_catalog_usage());
    };

    Ok(Command::ExportCatalog { dll_path, out_path })
}

fn export_catalog_usage() -> String {
    "usage: rdb-tools --export-catalog <source-dll> <out-file>".to_string()
}

fn parse_scan_command(
    rdb_path: String,
    mut args: impl Iterator<Item = String>,
) -> Result<Command, String> {
    let Some(folder) = args.next() else {
        return Err(scan_usage());
    };

    let mut catalog_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--catalog" => {
                let Some(path) = args.next() else {
                    return Err(scan_usage());
                };
                catalog_path = Some(path);
            }
            _ => return Err(format!("unknown scan option: {arg}")),
        }
    }

    Ok(Command::ScanArchive {
        rdb_path,
        folder,
        catalog_path,
    })
}

fn parse_scan_root_command(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    let Some(patcher_root) = args.next() else {
        return Err(scan_root_usage());
    };

    let mut rdb_root = None;
    let mut catalog_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--rdb-root" => rdb_root = args.next(),
            "--catalog" => catalog_path = args.next(),
            _ => return Err(format!("unknown scan-root option: {arg}")),
        }
    }

    Ok(Command::ScanRoot {
        patcher_root,
        rdb_root: rdb_root.ok_or_else(scan_root_usage)?,
        catalog_path: catalog_path.ok_or_else(scan_root_usage)?,
    })
}

fn scan_usage() -> String {
    "usage: rdb-tools <path-to-rdb> --scan <folder> [--catalog <dll>]".to_string()
}

fn scan_root_usage() -> String {
    "usage: rdb-tools --scan-root <patcher-root> --rdb-root <rdb-root> --catalog <dll>".to_string()
}

fn load_rdb_or_exit(path: &str) -> RdbIndex {
    let bytes = read_file_or_exit(path, "RDB");
    match parse_rdb(&bytes) {
        Ok(index) => index,
        Err(error) => {
            eprintln!("failed to parse {path}: {error:?}");
            process::exit(1);
        }
    }
}

fn read_file_or_exit(path: &str, label: &str) -> Vec<u8> {
    match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read {label} {path}: {error}");
            process::exit(1);
        }
    }
}

fn print_index_summary(path: &str, index: &RdbIndex) {
    println!("file: {path}");
    println!(
        "first_block_offset: 0x{:x}",
        index.header.first_block_offset
    );
    println!("declared_count: {}", index.header.declared_count);
    println!("data_prefix: {}", index.header.data_prefix);
    println!("parsed_blocks: {}", index.blocks.len());
}

fn run_command(command: Command) {
    match command {
        Command::ExportCatalog { dll_path, out_path } => export_catalog(&dll_path, &out_path),
        Command::Preview { rdb_path } => {
            let index = load_rdb_or_exit(&rdb_path);
            print_index_summary(&rdb_path, &index);
            print_blocks(index.blocks.iter().take(12), false);
        }
        Command::SearchHash { rdb_path, hash } => {
            let index = load_rdb_or_exit(&rdb_path);
            print_index_summary(&rdb_path, &index);
            search_hash(&index, hash);
        }
        Command::ScanArchive {
            rdb_path,
            folder,
            catalog_path,
        } => {
            let index = load_rdb_or_exit(&rdb_path);
            print_index_summary(&rdb_path, &index);
            scan_folder(
                &index,
                &folder,
                &load_optional_catalog(catalog_path.as_deref()),
            );
        }
        Command::ScanRoot {
            patcher_root,
            rdb_root,
            catalog_path,
        } => scan_root(&patcher_root, &rdb_root, &load_catalog(&catalog_path)),
    }
}

fn export_catalog(dll_path: &str, out_path: &str) {
    let bytes = read_file_or_exit(dll_path, "catalog source");
    let catalog = parse_name_hash_catalog(&bytes);
    let text = format_catalog_entries(&catalog);
    if let Err(error) = fs::write(out_path, text) {
        eprintln!("failed to write catalog {out_path}: {error}");
        process::exit(1);
    }
    println!("catalog_entries: {}", catalog.len());
    println!("catalog_written: {out_path}");
}

fn format_catalog_entries(catalog: &[NameHashEntry]) -> String {
    let mut output = String::new();
    for entry in catalog {
        output.push_str("0x");
        output.push_str(&format!("{:08x}", entry.hash));
        output.push(',');
        output.push_str(&entry.name);
        output.push_str("\r\n");
    }
    output
}

fn search_hash(index: &RdbIndex, hash: u32) {
    let blocks: Vec<_> = index
        .blocks
        .iter()
        .filter(|block| block.primary_hash == hash)
        .collect();

    println!("search_hash: 0x{hash:08x}");
    println!("matches: {}", blocks.len());
    print_blocks(blocks, true);
}

fn print_blocks<'a>(blocks: impl IntoIterator<Item = &'a RdbBlock>, verbose_payload: bool) {
    for block in blocks {
        print_block_summary(block);
        if verbose_payload {
            print_block_payload(block);
        }
    }
}

fn print_block_summary(block: &RdbBlock) {
    println!(
        "block @ 0x{offset:08x}: len=0x{length:08x} address_len=0x{address_len:x} hash=0x{hash:08x} data_offset=0x{data_offset:08x} payload_len=0x{payload_len:x}",
        offset = block.offset,
        length = block.length,
        address_len = block.field_10,
        hash = block.primary_hash,
        data_offset = block.data_offset,
        payload_len = block.payload.len(),
    );
}

fn print_block_payload(block: &RdbBlock) {
    println!("  payload_hex: {}", hex_preview(&block.payload));
    println!("  payload_text: {}", text_preview(&block.payload));
    print_scanned_tail(block);
    print_address_tail(block);
}

fn print_scanned_tail(block: &RdbBlock) {
    if let Some(tail) = parse_payload_tail(&block.payload) {
        println!(
            "  scanned_tail: {} => part_a=0x{:x} part_b=0x{:x}{}",
            tail.raw,
            tail.part_a,
            tail.part_b,
            format_suffix(&tail.suffix)
        );
    }
}

fn print_address_tail(block: &RdbBlock) {
    if let Some(tail) = parse_block_tail(block) {
        println!(
            "  address_tail: {} => part_a=0x{:x} part_b=0x{:x}{}",
            tail.raw,
            tail.part_a,
            tail.part_b,
            format_suffix(&tail.suffix)
        );
    }
}

fn format_suffix(suffix: &Option<RdbAddressSuffix>) -> String {
    match suffix {
        Some(suffix) => format!(" suffix={}0x{:x}", suffix.marker, suffix.value),
        None => String::new(),
    }
}

fn scan_folder(index: &RdbIndex, folder: &str, catalog: &[NameHashEntry]) {
    let names = match read_file_names(folder) {
        Ok(names) => names,
        Err(error) => {
            eprintln!("failed to scan {folder}: {error}");
            process::exit(1);
        }
    };

    let scanned =
        scan_virtualized_names_with_catalog(index, names.iter().map(String::as_str), catalog);
    print_scan_summary(folder, catalog.len(), &scanned);
    print_scan_entries(&scanned);
}

fn scan_root(patcher_root: &str, rdb_root: &str, catalog: &[NameHashEntry]) {
    let archive_names = read_directory_names_or_exit(patcher_root);
    let mut total_files = 0;
    let mut total_matched = 0;
    let mut total_missing = 0;
    let mut total_unresolved = 0;

    println!("scan_root: {patcher_root}");
    println!("rdb_root: {rdb_root}");
    println!("catalog_entries: {}", catalog.len());

    for archive_name in archive_names {
        let rdb_path = format!("{rdb_root}\\{archive_name}.rdb");
        let folder = format!("{patcher_root}\\{archive_name}");
        let Some((index, names)) = load_archive_inputs(&archive_name, &rdb_path, &folder) else {
            continue;
        };
        let scan = scan_archive_names_with_catalog(
            &archive_name,
            &index,
            names.iter().map(String::as_str),
            catalog,
        );
        let counts = scan.counts();

        total_files += counts.total;
        total_matched += counts.matched;
        total_missing += counts.hash_missing;
        total_unresolved += counts.unresolved_names;

        print_archive_scan_summary(&scan);
    }

    println!(
        "total files={total_files} matched={total_matched} missing={total_missing} unresolved={total_unresolved}"
    );
}

fn load_archive_inputs(
    archive_name: &str,
    rdb_path: &str,
    folder: &str,
) -> Option<(RdbIndex, Vec<String>)> {
    let bytes = match fs::read(rdb_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("archive {archive_name}: skipped, cannot read RDB {rdb_path}: {error}");
            return None;
        }
    };
    let index = match parse_rdb(&bytes) {
        Ok(index) => index,
        Err(error) => {
            println!("archive {archive_name}: skipped, cannot parse RDB {rdb_path}: {error:?}");
            return None;
        }
    };
    let names = match read_file_names(folder) {
        Ok(names) => names,
        Err(error) => {
            println!("archive {archive_name}: skipped, cannot scan folder {folder}: {error}");
            return None;
        }
    };

    Some((index, names))
}

fn print_archive_scan_summary(scan: &ArchiveScan<'_>) {
    let counts = scan.counts();
    println!(
        "archive {}: files={} matched={} missing={} unresolved={}",
        scan.archive_name,
        counts.total,
        counts.matched,
        counts.hash_missing,
        counts.unresolved_names
    );
    for file in scan.files.iter().filter(|file| file.block.is_none()) {
        match file.hash {
            Some(hash) => println!("  missing hash=0x{hash:08x} file={}", file.file_name),
            None => println!("  unresolved file={}", file.file_name),
        }
    }
}

fn read_directory_names_or_exit(folder: &str) -> Vec<String> {
    let mut names = Vec::new();
    let entries = match fs::read_dir(Path::new(folder)) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("failed to scan root {folder}: {error}");
            process::exit(1);
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!("failed to read root entry in {folder}: {error}");
                process::exit(1);
            }
        };
        match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("failed to read entry type in {folder}: {error}");
                process::exit(1);
            }
        }
    }

    names.sort();
    names
}

fn print_scan_summary(folder: &str, catalog_entries: usize, scanned: &[rdb::VirtualizedFile<'_>]) {
    let matched = scanned.iter().filter(|file| file.block.is_some()).count();
    let hash_missing = scanned
        .iter()
        .filter(|file| file.hash.is_some() && file.block.is_none())
        .count();
    let named = scanned.iter().filter(|file| file.hash.is_none()).count();

    println!("scan_folder: {folder}");
    println!("files: {}", scanned.len());
    println!("catalog_entries: {catalog_entries}");
    println!("hash_matches: {matched}");
    println!("hash_missing: {hash_missing}");
    println!("non_hash_names: {named}");
}

fn print_scan_entries(scanned: &[rdb::VirtualizedFile<'_>]) {
    for file in scanned {
        match (file.hash, file.block) {
            (Some(hash), Some(block)) => {
                println!(
                    "match hash=0x{hash:08x} file={} block=0x{:x} data_offset=0x{:x}",
                    file.file_name, block.offset, block.data_offset
                );
            }
            (Some(hash), None) => {
                println!("missing hash=0x{hash:08x} file={}", file.file_name);
            }
            (None, None) => {
                println!("name-route file={}", file.file_name);
            }
            (None, Some(_)) => unreachable!("non-hash names cannot match by hash"),
        }
    }
}

fn load_optional_catalog(path: Option<&str>) -> Vec<NameHashEntry> {
    path.map(load_catalog).unwrap_or_default()
}

fn load_catalog(path: &str) -> Vec<NameHashEntry> {
    let bytes = read_file_or_exit(path, "catalog");
    parse_name_hash_catalog(&bytes)
}

fn read_file_names(folder: &str) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(Path::new(folder))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

fn parse_hash(raw: &str) -> Result<u32, String> {
    let raw = raw.trim();
    if let Some(hash) = parse_prefixed_hex_hash(raw) {
        return Ok(hash);
    }

    let hex = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .unwrap_or(raw);
    u32::from_str_radix(hex, 16).map_err(|_| format!("invalid hash: {raw}"))
}

fn hex_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(96)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn text_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&byte| match byte {
            0 => '.',
            0x20..=0x7e => byte as char,
            _ => '?',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_catalog_entries_as_hash_name_lines() {
        let text = format_catalog_entries(&[
            NameHashEntry {
                name: "800_294_face_law_dressrosa_External_00.g1t".to_string(),
                hash: 0x359b9672,
            },
            NameHashEntry {
                name: "801_294_chara_law_dressrosa_External_00.g1t".to_string(),
                hash: 0x3bff0f13,
            },
        ]);

        assert_eq!(
            text,
            "0x359b9672,800_294_face_law_dressrosa_External_00.g1t\r\n\
             0x3bff0f13,801_294_chara_law_dressrosa_External_00.g1t\r\n"
        );
    }
}
