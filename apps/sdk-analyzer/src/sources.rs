use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

pub(crate) fn source_snapshot(
    roots: &[PathBuf],
) -> Result<BTreeMap<PathBuf, Option<SystemTime>>, String> {
    let mut snapshot = BTreeMap::new();
    for path in source_files(roots)? {
        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .ok();
        snapshot.insert(path, modified);
    }
    Ok(snapshot)
}

pub(crate) fn source_files(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
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

pub(crate) fn is_js_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("js"))
}

pub(crate) fn mod_root_for_source(roots: &[PathBuf], source_file: &Path) -> PathBuf {
    roots
        .iter()
        .map(|root| {
            if root.is_file() {
                root.parent().unwrap_or_else(|| Path::new("."))
            } else {
                root.as_path()
            }
        })
        .filter(|root| source_file.starts_with(root))
        .max_by_key(|root| root.components().count())
        .map(Path::to_path_buf)
        .or_else(|| source_file.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn read_string_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let quote = source.as_bytes().get(start).copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut value = String::new();
    let mut index = start + 1;
    let bytes = source.as_bytes();
    while let Some(byte) = bytes.get(index).copied() {
        if byte == quote {
            return Some((value, index + 1));
        }
        if byte == b'\\' {
            index += 1;
            value.push(*bytes.get(index)? as char);
        } else {
            value.push(byte as char);
        }
        index += 1;
    }
    None
}

pub(crate) fn skip_ws(source: &str, mut index: usize) -> usize {
    while source
        .as_bytes()
        .get(index)
        .is_some_and(u8::is_ascii_whitespace)
    {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use std::{env, time::SystemTime};

    use super::*;

    #[test]
    fn collects_js_sources_recursively() {
        let root = unique_temp_dir("sources");
        fs::create_dir_all(root.join("events")).expect("temp dir");
        fs::write(root.join("main.js"), "").expect("entry");
        fs::write(root.join("events/player.js"), "").expect("split source");
        fs::write(root.join("README.md"), "").expect("non source");

        let files = source_files(&[root.clone()]).expect("source files");

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("main.js")));
        assert!(files.iter().any(|path| path.ends_with("player.js")));
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
