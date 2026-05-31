use std::{path::PathBuf, time::Duration};

use crate::report::OutputFormat;

const DEFAULT_METHODS: &[&str] = &["replace_costume", "replaceCostume"];

#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) command: Command,
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) bridge: String,
    pub(crate) methods: Vec<String>,
    pub(crate) watch: bool,
    pub(crate) interval: Duration,
    pub(crate) output: OutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Command {
    Check,
}

pub(crate) fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
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
}
