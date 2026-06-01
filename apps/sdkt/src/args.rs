use std::{path::PathBuf, time::Duration};

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

use crate::report::OutputFormat;

const DEFAULT_METHODS: &[&str] = &["replace_costume", "replaceCostume"];

#[derive(Clone, Debug)]
pub(crate) struct Args {
    pub(crate) command: Command,
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) bridge: String,
    pub(crate) methods: Vec<String>,
    pub(crate) watch: bool,
    pub(crate) interval: Duration,
    pub(crate) output: OutputFormat,
    pub(crate) config_mode: Option<ConfigMode>,
    pub(crate) config_game_path: Option<PathBuf>,
    pub(crate) config_mods_path: Option<PathBuf>,
    pub(crate) config_profile: Option<String>,
    pub(crate) config_default_bridge: Option<String>,
    pub(crate) config_show: bool,
    pub(crate) package_output: Option<PathBuf>,
    pub(crate) force: bool,
    pub(crate) ignore_warnings: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    Check,
    Config,
    Init,
    Install,
    Update,
    Run,
    Test,
    Package,
}

#[derive(Parser, Debug)]
#[command(
    name = "sdkt",
    version,
    about = "SDK development toolkit",
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<CommandCli>,
}

#[derive(Subcommand, Debug)]
enum CommandCli {
    Check(CheckArgs),
    Config(ConfigArgs),
    Init(ProjectArgs),
    Install(InstallArgs),
    Update(ProjectArgs),
    Run(ProjectArgs),
    Test(ProjectArgs),
    Package(PackageArgs),
}

#[derive(ClapArgs, Debug)]
struct CheckArgs {
    #[arg(value_name = "FILE_OR_DIR", num_args = 1..)]
    roots: Vec<PathBuf>,
    #[arg(long, default_value = "bridge-js")]
    bridge: String,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    watch: bool,
    #[arg(long = "interval-ms", default_value_t = 750)]
    interval_ms: u64,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    json: bool,
    #[arg(long, value_enum)]
    format: Option<OutputFormatArg>,
    #[arg(long = "method")]
    methods: Vec<String>,
    #[arg(long = "no-default-methods", action = clap::ArgAction::SetTrue)]
    no_default_methods: bool,
}

#[derive(ClapArgs, Debug)]
struct ConfigArgs {
    #[command(subcommand)]
    mode: Option<ConfigModeCli>,
    #[arg(long = "game-path")]
    game_path: Option<PathBuf>,
    #[arg(long = "mods-path")]
    mods_path: Option<PathBuf>,
    #[arg(long)]
    profile: Option<String>,
    #[arg(long = "default-bridge")]
    default_bridge: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    show: bool,
}

#[derive(Subcommand, Debug)]
enum ConfigModeCli {
    Show,
    Get(GetConfigArgs),
}

#[derive(ClapArgs, Debug)]
struct GetConfigArgs {
    field: ConfigField,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConfigMode {
    Show,
    Get(ConfigField),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum ConfigField {
    GamePath,
    ModsPath,
    Profile,
    DefaultBridge,
}

#[derive(ClapArgs, Debug)]
struct ProjectArgs {
    #[arg(value_name = "PROJECT_ROOT")]
    root: Option<PathBuf>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    force: bool,
}

#[derive(ClapArgs, Debug)]
struct InstallArgs {
    #[arg(value_name = "BRIDGE", default_value = "bridge-js")]
    bridge: String,
}

#[derive(ClapArgs, Debug)]
struct PackageArgs {
    #[arg(value_name = "PROJECT_ROOT")]
    root: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    out: Option<PathBuf>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    force: bool,
    #[arg(long = "ignore-warnings", action = clap::ArgAction::SetTrue)]
    ignore_warnings: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormatArg {
    Human,
    Json,
}

pub(crate) fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let cli = Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let defaults = DEFAULT_METHODS
        .iter()
        .map(|method| method.to_string())
        .collect::<Vec<_>>();

    Ok(match cli.command.expect("subcommand is required by clap") {
        CommandCli::Check(args) => {
            let mut methods = if args.no_default_methods {
                Vec::new()
            } else {
                defaults
            };
            methods.extend(args.methods);
            methods.sort();
            methods.dedup();
            Args {
                command: Command::Check,
                roots: args.roots,
                bridge: args.bridge,
                methods,
                watch: args.watch,
                interval: Duration::from_millis(args.interval_ms.max(50)),
                output: match (args.json, args.format.unwrap_or(OutputFormatArg::Human)) {
                    (true, _) => OutputFormat::Json,
                    (false, OutputFormatArg::Human) => OutputFormat::Human,
                    (false, OutputFormatArg::Json) => OutputFormat::Json,
                },
                config_mode: None,
                config_game_path: None,
                config_mods_path: None,
                config_profile: None,
                config_default_bridge: None,
                config_show: false,
                package_output: None,
                force: false,
                ignore_warnings: false,
            }
        }
        CommandCli::Config(args) => Args {
            command: Command::Config,
            roots: Vec::new(),
            bridge: "bridge-js".to_string(),
            methods: DEFAULT_METHODS
                .iter()
                .map(|method| method.to_string())
                .collect(),
            watch: false,
            interval: Duration::from_millis(750),
            output: OutputFormat::Human,
            config_mode: args.mode.map(|mode| match mode {
                ConfigModeCli::Show => ConfigMode::Show,
                ConfigModeCli::Get(args) => ConfigMode::Get(args.field),
            }),
            config_game_path: args.game_path,
            config_mods_path: args.mods_path,
            config_profile: args.profile,
            config_default_bridge: args.default_bridge,
            config_show: args.show,
            package_output: None,
            force: false,
            ignore_warnings: false,
        },
        CommandCli::Init(args) => Args {
            command: Command::Init,
            roots: args.root.into_iter().collect(),
            bridge: "bridge-js".to_string(),
            methods: DEFAULT_METHODS
                .iter()
                .map(|method| method.to_string())
                .collect(),
            watch: false,
            interval: Duration::from_millis(750),
            output: OutputFormat::Human,
            config_mode: None,
            config_game_path: None,
            config_mods_path: None,
            config_profile: None,
            config_default_bridge: None,
            config_show: false,
            package_output: None,
            force: args.force,
            ignore_warnings: false,
        },
        CommandCli::Install(args) => Args {
            command: Command::Install,
            roots: Vec::new(),
            bridge: args.bridge,
            methods: DEFAULT_METHODS
                .iter()
                .map(|method| method.to_string())
                .collect(),
            watch: false,
            interval: Duration::from_millis(750),
            output: OutputFormat::Human,
            config_mode: None,
            config_game_path: None,
            config_mods_path: None,
            config_profile: None,
            config_default_bridge: None,
            config_show: false,
            package_output: None,
            force: false,
            ignore_warnings: false,
        },
        CommandCli::Update(args) => Args {
            command: Command::Update,
            roots: args.root.into_iter().collect(),
            bridge: "bridge-js".to_string(),
            methods: DEFAULT_METHODS
                .iter()
                .map(|method| method.to_string())
                .collect(),
            watch: false,
            interval: Duration::from_millis(750),
            output: OutputFormat::Human,
            config_mode: None,
            config_game_path: None,
            config_mods_path: None,
            config_profile: None,
            config_default_bridge: None,
            config_show: false,
            package_output: None,
            force: args.force,
            ignore_warnings: false,
        },
        CommandCli::Run(args) => Args {
            command: Command::Run,
            roots: args.root.into_iter().collect(),
            bridge: "bridge-js".to_string(),
            methods: DEFAULT_METHODS
                .iter()
                .map(|method| method.to_string())
                .collect(),
            watch: false,
            interval: Duration::from_millis(750),
            output: OutputFormat::Human,
            config_mode: None,
            config_game_path: None,
            config_mods_path: None,
            config_profile: None,
            config_default_bridge: None,
            config_show: false,
            package_output: None,
            force: false,
            ignore_warnings: false,
        },
        CommandCli::Test(args) => Args {
            command: Command::Test,
            roots: args.root.into_iter().collect(),
            bridge: "bridge-js".to_string(),
            methods: DEFAULT_METHODS
                .iter()
                .map(|method| method.to_string())
                .collect(),
            watch: false,
            interval: Duration::from_millis(750),
            output: OutputFormat::Human,
            config_mode: None,
            config_game_path: None,
            config_mods_path: None,
            config_profile: None,
            config_default_bridge: None,
            config_show: false,
            package_output: None,
            force: false,
            ignore_warnings: false,
        },
        CommandCli::Package(args) => Args {
            command: Command::Package,
            roots: args.root.into_iter().collect(),
            bridge: "bridge-js".to_string(),
            methods: DEFAULT_METHODS
                .iter()
                .map(|method| method.to_string())
                .collect(),
            watch: false,
            interval: Duration::from_millis(750),
            output: OutputFormat::Human,
            config_mode: None,
            config_game_path: None,
            config_mods_path: None,
            config_profile: None,
            config_default_bridge: None,
            config_show: false,
            package_output: args.out,
            force: args.force,
            ignore_warnings: args.ignore_warnings,
        },
    })
}

impl Args {
    pub(crate) fn project_root(&self) -> Option<&PathBuf> {
        self.roots.first()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_check_with_watch_and_methods() {
        let args = parse_args([
            "sdkt".to_string(),
            "check".to_string(),
            "--watch".to_string(),
            "--interval-ms".to_string(),
            "100".to_string(),
            "--method".to_string(),
            "replace_movesets".to_string(),
            "examples/js".to_string(),
        ])
        .expect("args");

        assert_eq!(args.command, Command::Check);
        assert!(args.watch);
        assert_eq!(args.interval, Duration::from_millis(100));
        assert_eq!(args.roots, [PathBuf::from("examples/js")]);
        assert!(args.methods.contains(&"replace_movesets".to_string()));
        assert!(args.methods.contains(&"replace_costume".to_string()));
    }

    #[test]
    fn parses_package_ignore_warnings() {
        let args = parse_args([
            "sdkt".to_string(),
            "package".to_string(),
            "--ignore-warnings".to_string(),
            "--out".to_string(),
            "mod.zip".to_string(),
        ])
        .expect("args");

        assert_eq!(args.command, Command::Package);
        assert!(args.ignore_warnings);
        assert_eq!(args.package_output, Some(PathBuf::from("mod.zip")));
    }

    #[test]
    fn parses_init_project() {
        let args = parse_args(["sdkt".to_string(), "init".to_string()]).expect("args");

        assert_eq!(args.command, Command::Init);
        assert!(args.roots.is_empty());
        assert!(!args.force);
    }

    #[test]
    fn parses_config_get() {
        let args = parse_args([
            "sdkt".to_string(),
            "config".to_string(),
            "get".to_string(),
            "game-path".to_string(),
        ])
        .expect("args");

        assert!(matches!(
            args.config_mode,
            Some(ConfigMode::Get(ConfigField::GamePath))
        ));
    }

    #[test]
    fn parses_config_show() {
        let args = parse_args(["sdkt".to_string(), "config".to_string(), "show".to_string()])
            .expect("args");

        assert_eq!(args.config_mode, Some(ConfigMode::Show));
    }
}
