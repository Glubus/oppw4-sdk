use std::{fs, io, io::Read};

use sdk_bridge::{BridgeModContext, BridgeModSource};

pub(super) fn read_entry_script(context: &BridgeModContext) -> io::Result<String> {
    read_script(context, &context.entry_file)
}

pub(super) fn read_script(context: &BridgeModContext, entry_name: &str) -> io::Result<String> {
    match &context.source {
        BridgeModSource::Directory(root) => fs::read_to_string(root.join(entry_name)),
        BridgeModSource::Zip { path, root } => {
            read_zip_text(path, &zip_entry_path(root, entry_name))
        }
    }
}

pub(super) fn script_exists(context: &BridgeModContext, entry_name: &str) -> bool {
    match &context.source {
        BridgeModSource::Directory(root) => root.join(entry_name).is_file(),
        BridgeModSource::Zip { path, root } => {
            zip_entry_exists(path, &zip_entry_path(root, entry_name))
        }
    }
}

fn read_zip_text(path: &std::path::Path, entry_name: &str) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive.by_name(entry_name)?;
    let mut text = String::new();
    entry.read_to_string(&mut text)?;
    Ok(text)
}

fn zip_entry_exists(path: &std::path::Path, entry_name: &str) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return false;
    };
    let exists = archive.by_name(entry_name).is_ok();
    exists
}

fn zip_entry_path(root: &str, entry_name: &str) -> String {
    if root.is_empty() {
        entry_name.to_string()
    } else if root.ends_with('/') {
        format!("{root}{entry_name}")
    } else {
        format!("{root}/{entry_name}")
    }
}
