use std::path::{Component, Path, PathBuf};

use crate::{
    report::{source_span_at, Diagnostic, DiagnosticSpan},
    sources::{read_string_literal, skip_ws},
};

pub(crate) fn validate_relative_imports(
    mod_root: &Path,
    source_file: &Path,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !source.contains("import") && !source.contains("from") {
        return;
    }
    let source_dir = source_file.parent().unwrap_or_else(|| Path::new("."));
    for import in relative_imports(source) {
        let Some(target) = resolve_relative_module_path(mod_root, source_dir, &import.specifier)
        else {
            diagnostics.push(
                Diagnostic::error(
                    source_file,
                    "import_path_invalid",
                    format!("relative import escapes the mod root: {}", import.specifier),
                )
                .with_span(import.span),
            );
            continue;
        };
        if !module_path_exists(&target) {
            diagnostics.push(
                Diagnostic::error(
                    source_file,
                    "import_missing",
                    format!("relative import does not resolve: {}", import.specifier),
                )
                .with_span(import.span),
            );
        }
    }
}

fn resolve_relative_module_path(
    mod_root: &Path,
    source_dir: &Path,
    specifier: &str,
) -> Option<PathBuf> {
    if !specifier.starts_with('.') {
        return None;
    }
    let mut normalized = source_dir.to_path_buf();
    for component in Path::new(specifier).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                normalized.pop();
                if !normalized.starts_with(mod_root) {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    normalized.starts_with(mod_root).then_some(normalized)
}

fn module_path_exists(path: &Path) -> bool {
    if path.is_file() {
        return true;
    }
    if path.extension().is_none() {
        for extension in ["js", "ts", "mjs", "mts"] {
            if path.with_extension(extension).is_file() {
                return true;
            }
        }
    }
    for index_name in ["index.js", "index.ts", "index.mjs", "index.mts"] {
        if path.join(index_name).is_file() {
            return true;
        }
    }
    false
}

#[derive(Debug)]
struct RelativeImport {
    specifier: String,
    span: Option<DiagnosticSpan>,
}

fn relative_imports(source: &str) -> Vec<RelativeImport> {
    let mut imports = Vec::new();
    collect_from_imports(source, &mut imports);
    collect_side_effect_imports(source, &mut imports);
    collect_dynamic_imports(source, &mut imports);
    imports.retain(|import| import.specifier.starts_with('.'));
    imports
}

fn collect_from_imports(source: &str, imports: &mut Vec<RelativeImport>) {
    let mut offset = 0;
    while let Some(index) = source[offset..].find("from") {
        let start = offset + index;
        offset = start + "from".len();
        if !is_word_boundary(source, start, offset) {
            continue;
        }
        let quote_start = skip_ws(source, offset);
        if let Some((specifier, end)) = read_string_literal(source, quote_start) {
            imports.push(RelativeImport {
                specifier,
                span: source_span_at(source, quote_start + 1, end.saturating_sub(quote_start + 2)),
            });
            offset = end;
        }
    }
}

fn collect_side_effect_imports(source: &str, imports: &mut Vec<RelativeImport>) {
    let mut offset = 0;
    while let Some(index) = source[offset..].find("import") {
        let start = offset + index;
        offset = start + "import".len();
        if !is_word_boundary(source, start, offset) {
            continue;
        }
        let quote_start = skip_ws(source, offset);
        if let Some((specifier, end)) = read_string_literal(source, quote_start) {
            imports.push(RelativeImport {
                specifier,
                span: source_span_at(source, quote_start + 1, end.saturating_sub(quote_start + 2)),
            });
            offset = end;
        }
    }
}

fn collect_dynamic_imports(source: &str, imports: &mut Vec<RelativeImport>) {
    let mut offset = 0;
    while let Some(index) = source[offset..].find("import") {
        let start = offset + index;
        offset = start + "import".len();
        if !is_word_boundary(source, start, offset) {
            continue;
        }
        let paren = skip_ws(source, offset);
        if !source[paren..].starts_with('(') {
            continue;
        }
        let quote_start = skip_ws(source, paren + 1);
        if let Some((specifier, end)) = read_string_literal(source, quote_start) {
            imports.push(RelativeImport {
                specifier,
                span: source_span_at(source, quote_start + 1, end.saturating_sub(quote_start + 2)),
            });
            offset = end;
        }
    }
}

fn is_word_boundary(source: &str, start: usize, end: usize) -> bool {
    let before = start == 0
        || source
            .as_bytes()
            .get(start - 1)
            .is_none_or(|byte| !is_ident_byte(*byte));
    let after = source
        .as_bytes()
        .get(end)
        .is_none_or(|byte| !is_ident_byte(*byte));
    before && after
}

fn is_ident_byte(byte: u8) -> bool {
    byte == b'_' || byte == b'$' || byte.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use std::{env, fs, time::SystemTime};

    use super::*;

    #[test]
    fn reports_missing_relative_import() {
        let root = unique_temp_dir("missing-import");
        fs::create_dir_all(&root).expect("temp dir");
        let source_file = root.join("main.js");
        let mut diagnostics = Vec::new();

        validate_relative_imports(
            &root,
            &source_file,
            r#"import "./missing.js";"#,
            &mut diagnostics,
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "import_missing"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_existing_extensionless_relative_import() {
        let root = unique_temp_dir("existing-import");
        fs::create_dir_all(root.join("events")).expect("temp dir");
        fs::write(root.join("events/player.js"), "").expect("import target");
        let source_file = root.join("main.js");
        let mut diagnostics = Vec::new();

        validate_relative_imports(
            &root,
            &source_file,
            r#"import "./events/player";"#,
            &mut diagnostics,
        );

        assert!(diagnostics.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_existing_extensionless_typescript_import() {
        let root = unique_temp_dir("existing-ts-import");
        fs::create_dir_all(root.join("events")).expect("temp dir");
        fs::write(root.join("events/player.ts"), "").expect("import target");
        let source_file = root.join("main.ts");
        let mut diagnostics = Vec::new();

        validate_relative_imports(
            &root,
            &source_file,
            r#"import "./events/player";"#,
            &mut diagnostics,
        );

        assert!(diagnostics.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_missing_export_from_relative_source() {
        let root = unique_temp_dir("missing-export-from");
        fs::create_dir_all(&root).expect("temp dir");
        let source_file = root.join("main.js");
        let mut diagnostics = Vec::new();

        validate_relative_imports(
            &root,
            &source_file,
            r#"export { value } from "./missing.js";"#,
            &mut diagnostics,
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "import_missing"));
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("oppw4-sdkt-{label}-{nanos}"))
    }
}
