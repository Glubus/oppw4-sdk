use std::{fs, path::Path};

const CONFIG_DIR: &str = ".sdk-analyzer";

pub(crate) fn init_bridge(bridge: &str) -> Result<(), String> {
    ensure_supported_bridge(bridge)?;
    fs::create_dir_all(CONFIG_DIR).map_err(|error| format!("{CONFIG_DIR}: {error}"))?;
    write_file(
        Path::new(CONFIG_DIR).join("config.toml"),
        format!(
            "default_bridge = \"{bridge}\"\nbridge_dir = \"bridges\"\nplugin_dir = \"plugins\"\n"
        ),
    )?;
    println!("initialized sdk-analyzer config for {bridge}");
    Ok(())
}

pub(crate) fn install_bridge(bridge: &str) -> Result<(), String> {
    ensure_supported_bridge(bridge)?;
    let bridge_dir = Path::new(CONFIG_DIR).join("bridges");
    fs::create_dir_all(&bridge_dir)
        .map_err(|error| format!("{}: {error}", bridge_dir.display()))?;
    write_file(
        bridge_dir.join(format!("{bridge}.toml")),
        "kind = \"builtin\"\nname = \"bridge-js\"\nanalyzer = \"sdk-js-analyzer\"\n".to_string(),
    )?;
    println!("installed builtin analyzer bridge {bridge}");
    Ok(())
}

fn ensure_supported_bridge(bridge: &str) -> Result<(), String> {
    if bridge == "bridge-js" {
        Ok(())
    } else {
        Err(format!("unsupported analyzer bridge: {bridge}"))
    }
}

fn write_file(path: impl AsRef<Path>, contents: String) -> Result<(), String> {
    let path = path.as_ref();
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_bridge() {
        assert!(ensure_supported_bridge("bridge-lua").is_err());
    }
}
