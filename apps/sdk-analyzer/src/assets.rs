use std::path::Path;

use crate::{
    report::{find_source_span, source_span_at, Diagnostic, DiagnosticSpan},
    sources::{read_string_literal, skip_ws},
};
use sdk_bridge::BridgeModEffect;

pub(crate) fn validate_effect_assets(
    mod_root: &Path,
    source_file: &Path,
    source: &str,
    effects: &[BridgeModEffect],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if effects.is_empty() {
        return;
    }
    for effect in effects {
        match effect {
            BridgeModEffect::ReplaceCostumeAsset { file, .. } => {
                validate_asset_path(
                    source_file,
                    source,
                    mod_root,
                    file,
                    "asset_missing",
                    diagnostics,
                );
            }
        }
    }
}

pub(crate) fn validate_replace_movesets_assets(
    mod_root: &Path,
    source_file: &Path,
    source: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !source.contains("replace_movesets") && !source.contains("replaceMovesets") {
        return;
    }
    for call in replace_movesets_calls(source) {
        validate_asset_path_with_span(
            source_file,
            mod_root,
            &call.asset,
            call.span,
            "moveset_asset_missing",
            diagnostics,
        );
    }
}

fn validate_asset_path(
    source_file: &Path,
    source: &str,
    mod_root: &Path,
    file: &str,
    missing_code: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    validate_asset_path_with_span(
        source_file,
        mod_root,
        file,
        find_source_span(source, file),
        missing_code,
        diagnostics,
    );
}

fn validate_asset_path_with_span(
    source_file: &Path,
    mod_root: &Path,
    file: &str,
    span: Option<DiagnosticSpan>,
    missing_code: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let asset_path = Path::new(file);
    if asset_path.is_absolute()
        || asset_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        diagnostics.push(
            Diagnostic::error(
                source_file,
                "asset_path_invalid",
                format!("asset path must stay inside the mod: {file}"),
            )
            .with_span(span),
        );
    } else if !mod_root.join(asset_path).is_file() {
        diagnostics.push(
            Diagnostic::error(
                source_file,
                missing_code,
                format!("referenced asset does not exist: {file}"),
            )
            .with_span(span),
        );
    }
}

#[derive(Debug)]
struct MovesetCall {
    asset: String,
    span: Option<DiagnosticSpan>,
}

fn replace_movesets_calls(source: &str) -> Vec<MovesetCall> {
    let mut calls = Vec::new();
    for method in [".replace_movesets", ".replaceMovesets"] {
        let mut offset = 0;
        while let Some(index) = source[offset..].find(method) {
            let method_start = offset + index;
            offset = method_start + method.len();
            let args_start = skip_ws(source, offset);
            if !source[args_start..].starts_with('(') {
                continue;
            }
            let value_start = skip_ws(source, args_start + 1);
            if let Some((asset, end)) = read_string_literal(source, value_start) {
                calls.push(MovesetCall {
                    asset,
                    span: source_span_at(
                        source,
                        value_start + 1,
                        end.saturating_sub(value_start + 2),
                    ),
                });
                offset = end;
            }
        }
    }
    calls
}

#[cfg(test)]
mod tests {
    use std::{env, fs, time::SystemTime};

    use super::*;

    #[test]
    fn reports_missing_assets_from_effects() {
        let root = unique_temp_dir("missing-asset");
        fs::create_dir_all(&root).expect("temp dir");
        let source = root.join("main.js");
        let effects = vec![BridgeModEffect::replace_costume_asset(
            Some("luffy"),
            "default",
            "texture.body",
            "missing.g1t",
        )];
        let mut diagnostics = Vec::new();

        validate_effect_assets(&root, &source, "", &effects, &mut diagnostics);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "asset_missing"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_missing_replace_movesets_asset() {
        let root = unique_temp_dir("missing-moveset");
        fs::create_dir_all(&root).expect("temp dir");
        let source_file = root.join("main.js");
        let mut diagnostics = Vec::new();

        validate_replace_movesets_assets(
            &root,
            &source_file,
            r#"character.replace_movesets("missing.bin");"#,
            &mut diagnostics,
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "moveset_asset_missing"
                && diagnostic.span.as_ref().is_some_and(|span| span.line == 1)));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_existing_replace_movesets_asset() {
        let root = unique_temp_dir("existing-moveset");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(root.join("moveset.bin"), []).expect("asset");
        let source_file = root.join("main.js");
        let mut diagnostics = Vec::new();

        validate_replace_movesets_assets(
            &root,
            &source_file,
            r#"character.replace_movesets("moveset.bin");"#,
            &mut diagnostics,
        );

        assert!(diagnostics.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("oppw4-sdk-analyzer-{label}-{nanos}"))
    }
}
