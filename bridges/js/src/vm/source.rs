use std::{fs, io, io::Read};

use sdk_bridge::{BridgeModContext, BridgeModSource};

pub(super) fn read_entry_script(context: &BridgeModContext) -> io::Result<String> {
    match &context.source {
        BridgeModSource::Directory(root) => fs::read_to_string(root.join(&context.entry_file)),
        BridgeModSource::Zip { path, root } => {
            read_zip_text(path, &zip_entry_path(root, &context.entry_file))
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

fn zip_entry_path(root: &str, entry_name: &str) -> String {
    if root.is_empty() {
        entry_name.to_string()
    } else {
        format!("{root}{entry_name}")
    }
}
