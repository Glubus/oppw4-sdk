use std::{fs, io, io::Read, path::Path};

use sdk_bridge::{BridgeModContext, BridgeModSource};
use swc_common::{
    sync::Lrc,
    FileName, Globals, Mark, SourceMap, GLOBALS,
};
use swc_ecma_ast::EsVersion;
use swc_ecma_codegen::to_code_default;
use swc_ecma_parser::{parse_file_as_program, EsSyntax, Syntax, TsSyntax};
use swc_ecma_transforms_typescript::typescript::strip;

pub(super) fn read_entry_script(context: &BridgeModContext) -> io::Result<String> {
    read_script(context, &context.entry_file)
}

pub(super) fn read_script(context: &BridgeModContext, entry_name: &str) -> io::Result<String> {
    match &context.source {
        BridgeModSource::Directory(root) => fs::read_to_string(root.join(entry_name)),
        BridgeModSource::Zip { path, root } => {
            read_zip_text(path, &zip_entry_path(root, entry_name))
        }
    }
}

pub(super) fn transpile_script(source_name: &str, source: &str) -> Result<String, String> {
    if !should_transpile(source_name) {
        return Ok(source.to_string());
    }

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom(source_name.to_string()).into(), source.to_string());
    let syntax = syntax_for(source_name);
    let mut recovered_errors = Vec::new();
    let mut program = parse_file_as_program(
        &fm,
        syntax,
        EsVersion::latest(),
        None,
        &mut recovered_errors,
    )
    .map_err(|error| format!("failed to parse {source_name}: {error:?}"))?;
    if !recovered_errors.is_empty() {
        return Err(format_swc_errors(source_name, &recovered_errors));
    }
    GLOBALS.set(&Globals::default(), || {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();
        program = program.apply(strip(unresolved_mark, top_level_mark));
        Ok::<_, String>(to_code_default(cm, None, &program))
    })
}

pub(super) fn script_exists(context: &BridgeModContext, entry_name: &str) -> bool {
    match &context.source {
        BridgeModSource::Directory(root) => root.join(entry_name).is_file(),
        BridgeModSource::Zip { path, root } => {
            zip_entry_exists(path, &zip_entry_path(root, entry_name))
        }
    }
}

fn read_zip_text(path: &std::path::Path, entry_name: &str) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive.by_name(entry_name)?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Ok(text)
}

fn zip_entry_exists(path: &std::path::Path, entry_name: &str) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return false;
    };
    let exists = archive.by_name(entry_name).is_ok();
    exists
}

fn zip_entry_path(root: &str, entry_name: &str) -> String {
    if root.is_empty() {
        entry_name.to_string()
    } else if root.ends_with('/') {
        format!("{root}{entry_name}")
    } else {
        format!("{root}/{entry_name}")
    }
}

fn should_transpile(source_name: &str) -> bool {
    matches!(
        extension_of(source_name).as_deref(),
        Some("ts" | "mjs" | "mts")
    )
}

fn syntax_for(source_name: &str) -> Syntax {
    match extension_of(source_name).as_deref() {
        Some("ts") | Some("mts") => Syntax::Typescript(TsSyntax {
            tsx: false,
            decorators: false,
            dts: false,
            no_early_errors: false,
            disallow_ambiguous_jsx_like: false,
        }),
        Some("mjs") => Syntax::Es(EsSyntax::default()),
        _ => Syntax::Es(EsSyntax::default()),
    }
}

fn extension_of(source_name: &str) -> Option<String> {
    Path::new(source_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn format_swc_errors<T: std::fmt::Debug>(name: &str, errors: &[T]) -> String {
    format!("failed to transpile {name}: {errors:?}")
}
