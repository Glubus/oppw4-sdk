use std::{fs, path::Path};

const CONFIG_DIR: &str = ".sdkt";

pub(crate) fn init_bridge(root: &Path, bridge: &str) -> Result<(), String> {
    ensure_supported_bridge(bridge)?;
    install_bridge(root, bridge)
}

pub(crate) fn install_bridge(root: &Path, bridge: &str) -> Result<(), String> {
    ensure_supported_bridge(bridge)?;
    let bridge_dir = root.join(CONFIG_DIR).join("bridges");
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
    use std::fs;

    use super::*;

    #[test]
    fn rejects_unknown_bridge() {
        assert!(ensure_supported_bridge("bridge-lua").is_err());
    }

    #[test]
    fn installs_bridge_inside_project_root() {
        let root = temp_root("install");
        fs::create_dir_all(&root).expect("root");

        install_bridge(&root, "bridge-js").expect("install");

        assert!(root
            .join(CONFIG_DIR)
            .join("bridges")
            .join("bridge-js.toml")
            .is_file());
        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("oppw4-sdkt-{label}-{nanos}"))
    }
}
