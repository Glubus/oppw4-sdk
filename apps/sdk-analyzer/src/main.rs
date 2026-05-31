use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

use sdk_bridge::{
    parse_mod_manifest, RegistryMethodDescriptor, RegistryModuleDescriptor, RegistryModuleSchema,
    RegistryTypeExtensionDescriptor, RegistryTypeRef,
};
use serde::Serialize;

const DEFAULT_METHODS: &[&str] = &["replace_costume", "replaceCostume"];

#[derive(Debug)]
struct Args {
    command: Command,
    roots: Vec<PathBuf>,
    bridge: String,
    methods: Vec<String>,
    watch: bool,
    interval: Duration,
    output: OutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    Check,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Serialize)]
struct AppReport {
    diagnostics: Vec<Diagnostic>,
    files: Vec<FileReport>,
    summary: ReportSummary,
}

#[derive(Debug, Serialize)]
struct Diagnostic {
    path: String,
    severity: DiagnosticSeverity,
    code: String,
    message: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Serialize)]
struct FileReport {
    path: String,
    effects: Vec<sdk_bridge::BridgeModEffect>,
    warnings: Vec<sdk_bridge::BridgeAnalysisWarning>,
}

#[derive(Debug, Serialize)]
struct ReportSummary {
    diagnostics: usize,
    errors: usize,
    files: usize,
    effects: usize,
    warnings: usize,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args().skip(1))?;
    match args.command {
        Command::Check if args.bridge != "bridge-js" => {
            Err(format!("unsupported analyzer bridge: {}", args.bridge))
        }
        Command::Check if args.watch => watch(args),
        Command::Check => {
            let report = analyze_roots(&args.roots, &method_modules(&args.methods))?;
            print_report(&report, args.output)?;
            finish_check(&report)
        }
    }
}

fn watch(args: Args) -> Result<(), String> {
    let modules = method_modules(&args.methods);
    let mut last_snapshot = BTreeMap::new();
    loop {
        let snapshot = source_snapshot(&args.roots)?;
        if snapshot != last_snapshot {
            last_snapshot = snapshot;
            let report = analyze_roots(&args.roots, &modules)?;
            print_report(&report, args.output)?;
        }
        thread::sleep(args.interval);
    }
}

fn analyze_roots(
    roots: &[PathBuf],
    modules: &[RegistryModuleDescriptor],
) -> Result<AppReport, String> {
    let diagnostics = manifest_diagnostics(roots);
    let mut files = Vec::new();
    for path in source_files(roots)? {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let report = sdk_js_analyzer::analyze(&source, modules);
        files.push(FileReport {
            path: path.display().to_string(),
            effects: report.effects,
            warnings: report.warnings,
        });
    }
    let summary = ReportSummary {
        diagnostics: diagnostics.len(),
        errors: diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.severity, DiagnosticSeverity::Error))
            .count(),
        files: files.len(),
        effects: files.iter().map(|file| file.effects.len()).sum(),
        warnings: files.iter().map(|file| file.warnings.len()).sum(),
    };
    Ok(AppReport {
        diagnostics,
        files,
        summary,
    })
}

fn print_report(report: &AppReport, output: OutputFormat) -> Result<(), String> {
    match output {
        OutputFormat::Human => print!("{}", format_human_report(report)),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(report)
                .map_err(|error| format!("failed to serialize analysis report: {error}"))?;
            println!("{json}");
        }
    }
    Ok(())
}

fn finish_check(report: &AppReport) -> Result<(), String> {
    if report.summary.errors == 0 {
        Ok(())
    } else {
        Err(format!(
            "sdk-analyzer found {} error(s)",
            report.summary.errors
        ))
    }
}

fn format_human_report(report: &AppReport) -> String {
    let mut output = String::new();
    output.push_str("    Checking bridge-js mods\n");
    for diagnostic in &report.diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
        };
        output.push_str(&format!(
            "{severity}[{}]: {}\n  --> {}\n\n",
            diagnostic.code, diagnostic.message, diagnostic.path
        ));
    }
    for file in &report.files {
        for warning in &file.warnings {
            output.push_str(&format!(
                "warning[{}]: {}\n  --> {}\n\n",
                warning.code, warning.message, file.path
            ));
        }
        for effect in &file.effects {
            output.push_str(&format!(
                "note[effect]: {}\n  --> {}\n\n",
                effect.describe(),
                file.path
            ));
        }
    }

    let status = if report.summary.errors == 0 {
        "Finished"
    } else {
        "Failed"
    };
    output.push_str(&format!(
        "{status} sdk-analyzer: {} file(s), {} effect(s), {} warning(s), {} error(s)\n",
        report.summary.files,
        report.summary.effects,
        report.summary.warnings + report.summary.diagnostics - report.summary.errors,
        report.summary.errors,
    ));
    output
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut command = None;
    let mut roots = Vec::new();
    let mut bridge = "bridge-js".to_string();
    let mut methods = DEFAULT_METHODS
        .iter()
        .map(|method| method.to_string())
        .collect::<Vec<_>>();
    let mut watch = false;
    let mut interval = Duration::from_millis(750);
    let mut output = OutputFormat::Human;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(usage()),
            "check" if command.is_none() => command = Some(Command::Check),
            "--json" => output = OutputFormat::Json,
            "--watch" => watch = true,
            "--bridge" => {
                bridge = args
                    .next()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| "--bridge requires a bridge id".to_string())?;
            }
            "--method" => {
                let method = args
                    .next()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| "--method requires a method name".to_string())?;
                methods.push(method);
            }
            "--no-default-methods" => methods.clear(),
            "--interval-ms" => {
                let millis = args
                    .next()
                    .ok_or_else(|| "--interval-ms requires a value".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "--interval-ms must be an integer".to_string())?;
                interval = Duration::from_millis(millis.max(50));
            }
            "--format" => {
                output = match args
                    .next()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| "--format requires human or json".to_string())?
                    .as_str()
                {
                    "human" => OutputFormat::Human,
                    "json" => OutputFormat::Json,
                    value => return Err(format!("unsupported output format: {value}")),
                };
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown argument: {value}\n\n{}", usage()));
            }
            value => roots.push(PathBuf::from(value)),
        }
    }
    let command = command.unwrap_or(Command::Check);
    if roots.is_empty() {
        return Err(usage());
    }
    methods.sort();
    methods.dedup();
    Ok(Args {
        command,
        roots,
        bridge,
        methods,
        watch,
        interval,
        output,
    })
}

fn method_modules(methods: &[String]) -> Vec<RegistryModuleDescriptor> {
    if methods.is_empty() {
        return Vec::new();
    }
    let mut extension = RegistryTypeExtensionDescriptor::new("sdk.Character");
    for method in methods {
        extension = extension.method(RegistryMethodDescriptor::new(
            method,
            method,
            RegistryTypeRef::Json,
        ));
    }
    vec![
        RegistryModuleDescriptor::builder("standalone", "sdk.character")
            .schema(RegistryModuleSchema::new("sdk", "character").extension(extension))
            .build(),
    ]
}

fn source_snapshot(roots: &[PathBuf]) -> Result<BTreeMap<PathBuf, Option<SystemTime>>, String> {
    let mut snapshot = BTreeMap::new();
    for path in source_files(roots)? {
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .ok();
        snapshot.insert(path, modified);
    }
    Ok(snapshot)
}

fn manifest_diagnostics(roots: &[PathBuf]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for root in roots {
        if root.is_dir() {
            let manifest = root.join("mod.toml");
            if manifest.exists() {
                validate_manifest(root, &manifest, &mut diagnostics);
            }
        } else if root.file_name().is_some_and(|name| name == "mod.toml") {
            let root = root.parent().unwrap_or_else(|| Path::new("."));
            validate_manifest(root, root.join("mod.toml").as_path(), &mut diagnostics);
        }
    }
    diagnostics
}

fn validate_manifest(root: &Path, manifest: &Path, diagnostics: &mut Vec<Diagnostic>) {
    let text = match fs::read_to_string(manifest) {
        Ok(text) => text,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                manifest,
                "manifest_read_failed",
                format!("failed to read mod.toml: {error}"),
            ));
            return;
        }
    };
    let manifest_data = match parse_mod_manifest(&text) {
        Ok(manifest_data) => manifest_data,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                manifest,
                "manifest_invalid",
                format!("invalid mod.toml: {error:?}"),
            ));
            return;
        }
    };
    let entry = root.join(&manifest_data.entry_file);
    if !entry.exists() {
        diagnostics.push(Diagnostic::error(
            manifest,
            "entry_missing",
            format!("entry file does not exist: {}", manifest_data.entry_file),
        ));
    } else if !is_js_file(&entry) {
        diagnostics.push(Diagnostic::warning(
            manifest,
            "entry_not_js",
            format!(
                "bridge-js analyzer expects a .js entry: {}",
                manifest_data.entry_file
            ),
        ));
    }
    if manifest_data.uses_plugins.is_empty() {
        diagnostics.push(Diagnostic::warning(
            manifest,
            "uses_plugins_empty",
            "mod.toml does not declare [uses].plugins",
        ));
    }
}

fn source_files(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for root in roots {
        collect_source_files(root, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_source_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if is_js_file(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        return Err(format!("path does not exist: {}", path.display()));
    }
    for entry in fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))? {
        let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;
        collect_source_files(&entry.path(), files)?;
    }
    Ok(())
}

fn is_js_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("js"))
}

impl Diagnostic {
    fn error(path: &Path, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.display().to_string(),
            severity: DiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
        }
    }

    fn warning(path: &Path, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.display().to_string(),
            severity: DiagnosticSeverity::Warning,
            code: code.into(),
            message: message.into(),
        }
    }
}

fn usage() -> String {
    "usage: sdk-analyzer check [--bridge bridge-js] [--watch] [--interval-ms n] [--json|--format human|json] [--method name] [--no-default-methods] <file-or-dir>..."
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_roots_methods_and_watch() {
        let args = parse_args([
            "--watch".to_string(),
            "--interval-ms".to_string(),
            "100".to_string(),
            "--method".to_string(),
            "replace_movesets".to_string(),
            "--bridge".to_string(),
            "bridge-js".to_string(),
            "examples/js".to_string(),
        ])
        .expect("args");

        assert_eq!(args.command, Command::Check);
        assert_eq!(args.bridge, "bridge-js");
        assert!(args.watch);
        assert_eq!(args.output, OutputFormat::Human);
        assert_eq!(args.interval, Duration::from_millis(100));
        assert_eq!(args.roots, [PathBuf::from("examples/js")]);
        assert!(args.methods.contains(&"replace_movesets".to_string()));
        assert!(args.methods.contains(&"replace_costume".to_string()));
    }

    #[test]
    fn parses_json_output() {
        let args = parse_args([
            "check".to_string(),
            "--json".to_string(),
            "main.js".to_string(),
        ])
        .expect("args");

        assert_eq!(args.output, OutputFormat::Json);
    }

    #[test]
    fn formats_human_report_like_check_output() {
        let report = AppReport {
            diagnostics: vec![Diagnostic::warning(
                Path::new("mod.toml"),
                "uses_plugins_empty",
                "mod.toml does not declare [uses].plugins",
            )],
            files: vec![FileReport {
                path: "main.js".to_string(),
                effects: Vec::new(),
                warnings: vec![sdk_bridge::analysis_warning(
                    "dynamic_patch",
                    "dynamic patch shape cannot be fully analyzed",
                )],
            }],
            summary: ReportSummary {
                diagnostics: 1,
                errors: 0,
                files: 1,
                effects: 0,
                warnings: 1,
            },
        };

        let formatted = format_human_report(&report);

        assert!(formatted.contains("    Checking bridge-js mods"));
        assert!(formatted.contains("warning[uses_plugins_empty]"));
        assert!(formatted.contains("  --> mod.toml"));
        assert!(formatted.contains("warning[dynamic_patch]"));
        assert!(formatted.contains("Finished sdk-analyzer"));
    }

    #[test]
    fn can_disable_default_methods() {
        let args = parse_args([
            "check".to_string(),
            "--no-default-methods".to_string(),
            "--method".to_string(),
            "custom".to_string(),
            "main.js".to_string(),
        ])
        .expect("args");

        assert_eq!(args.methods, ["custom"]);
    }

    #[test]
    fn method_modules_declare_requested_methods() {
        let modules = method_modules(&["replace_costume".to_string()]);

        assert!(sdk_bridge::registry_declares_method(
            &modules,
            "replace_costume"
        ));
        assert!(!sdk_bridge::registry_declares_method(
            &modules,
            "replace_movesets"
        ));
    }

    #[test]
    fn validates_mod_manifest_entry() {
        let root = unique_temp_dir("manifest-entry");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(
            root.join("mod.toml"),
            r#"
            [mod]
            id = "example"

            [uses]
            plugins = ["sdk_runtime"]

            [entry]
            file = "main.js"
            "#,
        )
        .expect("manifest");
        fs::write(root.join("main.js"), "").expect("entry");

        let diagnostics = manifest_diagnostics(&[root.clone()]);

        assert!(diagnostics.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reports_missing_manifest_entry() {
        let root = unique_temp_dir("missing-entry");
        fs::create_dir_all(&root).expect("temp dir");
        fs::write(
            root.join("mod.toml"),
            r#"
            [mod]
            id = "example"

            [entry]
            file = "missing.js"
            "#,
        )
        .expect("manifest");

        let diagnostics = manifest_diagnostics(&[root.clone()]);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "entry_missing"));
        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        env::temp_dir().join(format!("oppw4-sdk-analyzer-{label}-{nanos}"))
    }
}
