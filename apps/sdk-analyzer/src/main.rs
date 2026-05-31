mod args;
mod assets;
mod imports;
mod install;
mod manifest;
mod report;
mod sdk_contracts;
mod sources;

use std::{env, fs, path::PathBuf, thread};

use args::{parse_args, Command};
use report::{check_exit_code, print_report, AppReport, FileReport, ReportSummary};
use sdk_bridge::RegistryModuleDescriptor;

fn main() {
    match run() {
        Ok(code) => {
            if code != 0 {
                std::process::exit(code);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32, String> {
    let args = parse_args(env::args().skip(1))?;
    match args.command {
        Command::Check if args.bridge != "bridge-js" => {
            Err(format!("unsupported analyzer bridge: {}", args.bridge))
        }
        Command::Check if args.watch => watch(args).map(|_| 0),
        Command::Check => {
            let report = analyze_roots(&args.roots, &sdk_contracts::method_modules(&args.methods))?;
            print_report(&report, args.output)?;
            Ok(check_exit_code(&report))
        }
        Command::Init => install::init_bridge(&args.bridge).map(|_| 0),
        Command::Install => install::install_bridge(&args.bridge).map(|_| 0),
    }
}

fn watch(args: args::Args) -> Result<(), String> {
    let modules = sdk_contracts::method_modules(&args.methods);
    let mut last_snapshot = std::collections::BTreeMap::new();
    loop {
        let snapshot = sources::source_snapshot(&args.roots)?;
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
    let mut diagnostics = manifest::manifest_diagnostics(roots);
    let source_files = sources::source_files(roots)?;
    let mut files = Vec::with_capacity(source_files.len());
    for path in source_files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let analysis = sdk_js_analyzer::analyze(&source, modules);
        let mod_root = sources::mod_root_for_source(roots, &path);
        imports::validate_relative_imports(&mod_root, &path, &source, &mut diagnostics);
        assets::validate_effect_assets(
            &mod_root,
            &path,
            &source,
            &analysis.effects,
            &mut diagnostics,
        );
        assets::validate_replace_movesets_assets(&mod_root, &path, &source, &mut diagnostics);
        files.push(FileReport {
            path: path.display().to_string(),
            effects: analysis.effects,
            warnings: analysis.warnings,
        });
    }
    let summary = ReportSummary {
        diagnostics: diagnostics.len(),
        errors: diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.severity, report::DiagnosticSeverity::Error))
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
