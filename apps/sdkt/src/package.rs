use std::{
    ffi::OsStr,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use sdk_mod_loader::parse_mod_manifest;

pub(crate) fn package_project(
    root: &Path,
    output: Option<PathBuf>,
    force: bool,
) -> Result<PathBuf, String> {
    let root = normalize_root(root);
    let manifest_path = root.join("mod.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("{}: {error}", manifest_path.display()))?;
    let manifest = parse_mod_manifest(&manifest_text)
        .map_err(|error| format!("{}: {error:?}", manifest_path.display()))?;
    let entry_path = root.join(&manifest.entry_file);
    if !entry_path.exists() {
        return Err(format!(
            "entry file does not exist: {}",
            entry_path.display()
        ));
    }

    let output = output.unwrap_or_else(|| root.join(format!("{}.zip", manifest.id)));
    if output.exists() && !force {
        return Err(format!(
            "output already exists: {} (use --force to overwrite)",
            output.display()
        ));
    }

    let file =
        fs::File::create(&output).map_err(|error| format!("{}: {error}", output.display()))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    write_tree(&mut writer, &root, &root, &output, options)?;
    writer
        .finish()
        .map_err(|error| format!("failed to finalize archive: {error}"))?;
    Ok(output)
}

fn write_tree(
    writer: &mut zip::ZipWriter<fs::File>,
    root: &Path,
    dir: &Path,
    output: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| format!("{}: {error}", dir.display()))?;
        let path = entry.path();
        if should_skip(root, &path, output) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if file_type.is_dir() {
            write_tree(writer, root, &path, output, options.clone())?;
        } else if file_type.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|_| format!("{} is outside root {}", path.display(), root.display()))?;
            let name = rel.to_string_lossy().replace('\\', "/");
            writer
                .start_file(name, options.clone())
                .map_err(|error| format!("{}: {error}", path.display()))?;
            let mut file =
                fs::File::open(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            writer
                .write_all(&bytes)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    Ok(())
}

fn should_skip(root: &Path, path: &Path, output: &Path) -> bool {
    path == output
        || path
            .strip_prefix(root)
            .ok()
            .and_then(|rel| rel.components().next())
            .and_then(|component| match component.as_os_str() {
                value if value == OsStr::new(".sdkt") => Some(true),
                value if value == OsStr::new(".git") => Some(true),
                value if value == OsStr::new("target") => Some(true),
                _ => None,
            })
            .unwrap_or(false)
}

fn normalize_root(path: &Path) -> PathBuf {
    if path
        .file_name()
        .is_some_and(|name| name == OsStr::new("mod.toml"))
    {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn packages_root_into_zip() {
        let root = temp_root("package");
        fs::create_dir_all(&root).expect("root");
        fs::write(
            root.join("mod.toml"),
            r#"
            [mod]
            id = "example"
            name = "Example"

            [uses]
            plugins = ["sdk_runtime"]

            [entry]
            file = "main.js"
            "#,
        )
        .expect("manifest");
        fs::write(root.join("main.js"), "console.log('ok');").expect("entry");
        fs::create_dir_all(root.join(".sdkt")).expect("sdkt");
        fs::write(root.join(".sdkt").join("config.toml"), "ignored = true").expect("config");

        let output = package_project(&root, None, true).expect("package");
        let file = fs::File::open(&output).expect("zip");
        let mut archive = zip::ZipArchive::new(file).expect("archive");
        let mut names = (0..archive.len())
            .map(|index| archive.by_index(index).expect("entry").name().to_string())
            .collect::<Vec<_>>();
        names.sort();

        assert!(names.contains(&"main.js".to_string()));
        assert!(names.contains(&"mod.toml".to_string()));
        assert!(!names.iter().any(|name| name.starts_with(".sdkt/")));
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
