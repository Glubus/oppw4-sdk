mod args;
mod assets;
mod config;
mod imports;
mod install;
mod launch;
mod manifest;
mod package;
mod project;
mod report;
mod sdk_contracts;
mod sources;
mod types;

use std::{env, fs, path::PathBuf, thread};

use args::{parse_args, Command};
use report::{check_exit_code, print_report, AppReport, FileReport, ReportSummary};
use sdk_bridge::RegistryModuleDescriptor;

pub fn run() -> Result<i32, String> {
    let args = parse_args(env::args())?;
    match args.command {
        Command::Check => run_check(&args),
        Command::Config => run_config(&args),
        Command::Init => run_init(&args),
        Command::Install => run_install(&args),
        Command::Update => run_update(&args),
        Command::Run => run_run(&args),
        Command::Test => run_test(&args),
        Command::Package => run_package(&args),
    }
}

fn run_check(args: &args::Args) -> Result<i32, String> {
    if args.bridge != "bridge-js" {
        return Err(format!("unsupported analyzer bridge: {}", args.bridge));
    }
    if args.watch {
        watch(args.clone()).map(|_| 0)
    } else {
        let report = analyze_roots(&args.roots, &sdk_contracts::method_modules(&args.methods))?;
        print_report(&report, args.output)?;
        Ok(check_exit_code(&report))
    }
}

fn run_config(args: &args::Args) -> Result<i32, String> {
    let root = project_root(args)?;
    if let Some(mode) = args.config_mode {
        let config = config::load(&root)?;
        return match mode {
            args::ConfigMode::Show => {
                println!("{}", config::format(&config)?);
                Ok(0)
            }
            args::ConfigMode::Get(field) => {
                match field {
                    args::ConfigField::GamePath => print_config_path(config.game_path.as_ref()),
                    args::ConfigField::ModsPath => print_config_path(config.mods_path.as_ref()),
                    args::ConfigField::Profile => print_config_value(config.profile.as_ref()),
                    args::ConfigField::DefaultBridge => {
                        print_config_value(config.default_bridge.as_ref())
                    }
                }
                Ok(0)
            }
        };
    }
    let patch = config::ConfigPatch {
        game_path: args.config_game_path.clone(),
        mods_path: args.config_mods_path.clone(),
        profile: args.config_profile.clone(),
        default_bridge: args.config_default_bridge.clone(),
    };
    let patch_is_empty = patch.is_empty();
    let updated = if patch_is_empty {
        config::load(&root)?
    } else {
        config::update(&root, patch)?
    };
    if args.config_show || patch_is_empty {
        println!("{}", config::format(&updated)?);
    }
    Ok(0)
}

fn print_config_value<T: std::fmt::Display>(value: Option<&T>) {
    if let Some(value) = value {
        println!("{value}");
    }
}

fn print_config_path(value: Option<&std::path::PathBuf>) {
    if let Some(value) = value {
        println!("{}", value.display());
    }
}

fn run_init(args: &args::Args) -> Result<i32, String> {
    let root = project_root(args)?;
    project::init_project(&root, args.force)?;
    install::init_bridge(&root, &config::default_bridge())?;
    types::install_types(&root)?;
    Ok(0)
}

fn run_install(args: &args::Args) -> Result<i32, String> {
    let root = project_root(args)?;
    install::install_bridge(&root, &args.bridge)?;
    types::install_types(&root)?;
    Ok(0)
}

fn run_update(args: &args::Args) -> Result<i32, String> {
    let root = project_root(args)?;
    let current = config::load(&root)?;
    config::save(&root, &current)?;
    install::install_bridge(
        &root,
        &current
            .default_bridge
            .clone()
            .unwrap_or_else(config::default_bridge),
    )?;
    types::install_types(&root)?;
    Ok(0)
}

fn run_run(args: &args::Args) -> Result<i32, String> {
    let root = project_root(args)?;
    let config = config::load(&root)?;
    let plan = launch::build_launch_plan(&root, &config)?;
    let report = analyze_roots(
        &analysis_roots(args, &root)?,
        &sdk_contracts::method_modules(&args.methods),
    )?;
    print_report(&report, args.output)?;
    if report.summary.errors != 0 {
        return Ok(1);
    }
    if args.output == report::OutputFormat::Human {
        println!("preflight ok");
        print!("{}", launch::render_launch_plan(&plan));
        println!("launching game");
    }
    let status = launch::spawn_game(&plan)?;
    if args.output == report::OutputFormat::Human {
        println!("game exited with {status}");
        println!("launch complete");
    }
    Ok(status.code().unwrap_or(1))
}

fn run_test(args: &args::Args) -> Result<i32, String> {
    let root = project_root(args)?;
    let report = analyze_roots(
        &analysis_roots(args, &root)?,
        &sdk_contracts::method_modules(&args.methods),
    )?;
    print_report(&report, args.output)?;
    Ok(check_exit_code(&report))
}

fn run_package(args: &args::Args) -> Result<i32, String> {
    let root = project_root(args)?;
    let report = analyze_roots(
        &analysis_roots(args, &root)?,
        &sdk_contracts::method_modules(&args.methods),
    )?;
    print_report(&report, args.output)?;
    if report.summary.errors != 0 {
        return Ok(1);
    }
    if !args.ignore_warnings && report.summary.warnings != 0 {
        return Ok(1);
    }
    let output = package::package_project(&root, args.package_output.clone(), args.force)?;
    if args.output == report::OutputFormat::Human {
        println!("packaged mod to {}", output.display());
    }
    Ok(0)
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

pub(crate) fn analyze_roots(
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

fn project_root(args: &args::Args) -> Result<PathBuf, String> {
    if let Some(root) = args.project_root() {
        Ok(project::normalize_root(root))
    } else {
        env::current_dir().map_err(|error| format!("failed to resolve current directory: {error}"))
    }
}

fn analysis_roots(args: &args::Args, fallback_root: &PathBuf) -> Result<Vec<PathBuf>, String> {
    if args.roots.is_empty() {
        Ok(vec![fallback_root.clone()])
    } else {
        Ok(args
            .roots
            .iter()
            .map(|root| project::normalize_root(root))
            .collect())
    }
}
