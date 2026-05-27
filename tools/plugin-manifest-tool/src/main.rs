use std::{env, fs, path::Path, process};

use plugin_sdk::{PluginDescriptor, PLUGIN_MANIFEST_FILE};

fn main() {
    let path = match parse_args(env::args().skip(1)) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    };

    match validate_manifest(&path) {
        Ok(descriptor) => {
            println!(
                "ok id={} version={} entry={}",
                descriptor.id, descriptor.version, descriptor.entry
            );
            print_list("dependencies", &descriptor.dependencies);
            let registry_modules = descriptor
                .registry_modules
                .iter()
                .map(|module| module.module.clone())
                .collect::<Vec<_>>();
            print_list("registry_modules", &registry_modules);
            print_list("requires", &descriptor.capabilities_required);
            print_list("provides", &descriptor.capabilities_provided);
        }
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<String, String> {
    let Some(path) = args.next() else {
        return Err(format!(
            "usage: plugin-manifest <path-to-{PLUGIN_MANIFEST_FILE}>"
        ));
    };
    if args.next().is_some() {
        return Err(format!(
            "usage: plugin-manifest <path-to-{PLUGIN_MANIFEST_FILE}>"
        ));
    }
    Ok(path)
}

fn validate_manifest(path: &str) -> Result<PluginDescriptor, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", Path::new(path).display()))?;
    PluginDescriptor::parse_toml(&text)
        .map_err(|error| format!("invalid {}: {error:?}", Path::new(path).display()))
}

fn print_list(label: &str, values: &[String]) {
    if values.is_empty() {
        println!("{label}: []");
    } else {
        println!("{label}: {}", values.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_manifest_path() {
        assert_eq!(
            parse_args(["plugin.toml".to_string()].into_iter()).expect("args"),
            "plugin.toml"
        );
    }

    #[test]
    fn rejects_missing_manifest_path() {
        assert_eq!(
            parse_args(Vec::<String>::new().into_iter()).expect_err("usage"),
            "usage: plugin-manifest <path-to-plugin.toml>"
        );
    }
}
