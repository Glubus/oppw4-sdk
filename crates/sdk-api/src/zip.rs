use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZipFileEntry {
    name: String,
    size: u64,
}

impl ZipFileEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn size(&self) -> u64 {
        self.size
    }

    pub fn file_name(&self) -> Option<&str> {
        self.name
            .rsplit('/')
            .find(|part| !part.is_empty() && *part != ".")
    }
}

pub fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

pub fn zip_file_entries(path: &Path) -> std::io::Result<Vec<ZipFileEntry>> {
    let file = File::open(path)?;
    zip_file_entries_from_reader(file)
}

pub fn zip_file_entries_from_reader(
    reader: impl Read + Seek,
) -> std::io::Result<Vec<ZipFileEntry>> {
    let mut archive = open_archive(reader)?;
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(zip_error)?;
        if entry.is_file() {
            entries.push(ZipFileEntry {
                name: normalize_zip_entry_name(entry.name()),
                size: entry.size(),
            });
        }
    }
    Ok(entries)
}

pub fn read_zip_entry(path: &Path, entry_name: &str) -> std::io::Result<Vec<u8>> {
    let file = File::open(path)?;
    read_zip_entry_from_reader(file, entry_name)
}

pub fn read_zip_entry_from_reader(
    reader: impl Read + Seek,
    entry_name: &str,
) -> std::io::Result<Vec<u8>> {
    let mut archive = open_archive(reader)?;
    let mut entry = archive.by_name(entry_name).map_err(zip_error)?;
    let mut bytes = Vec::with_capacity(entry.size().min(usize::MAX as u64) as usize);
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub fn zip_entry_size(path: &Path, entry_name: &str) -> std::io::Result<u64> {
    let file = File::open(path)?;
    let mut archive = open_archive(file)?;
    archive
        .by_name(entry_name)
        .map(|entry| entry.size())
        .map_err(zip_error)
}

fn open_archive<R: Read + Seek>(reader: R) -> std::io::Result<zip::ZipArchive<R>> {
    zip::ZipArchive::new(reader).map_err(zip_error)
}

fn normalize_zip_entry_name(name: &str) -> String {
    name.replace('\\', "/")
}

fn zip_error(error: zip::result::ZipError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use super::*;

    #[test]
    fn lists_only_files_with_normalized_entry_names() {
        let bytes = zip_bytes(&[
            ("ScreenLayout\\portrait.g1t", b"abc".as_slice()),
            ("ScreenLayout/", b""),
        ]);

        let entries = zip_file_entries_from_reader(Cursor::new(bytes)).expect("entries");

        assert_eq!(
            entries,
            [ZipFileEntry {
                name: "ScreenLayout/portrait.g1t".to_string(),
                size: 3,
            }]
        );
        assert_eq!(entries[0].file_name(), Some("portrait.g1t"));
    }

    #[test]
    fn reads_named_entry_payload() {
        let bytes = zip_bytes(&[("name_hash_catalog.txt", b"hello".as_slice())]);

        assert_eq!(
            read_zip_entry_from_reader(Cursor::new(bytes), "name_hash_catalog.txt").expect("entry"),
            b"hello"
        );
    }

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut output);
            let file_options = zip::write::SimpleFileOptions::default();
            for (name, bytes) in entries {
                if name.ends_with('/') {
                    writer.add_directory(*name, file_options).unwrap();
                } else {
                    writer.start_file(*name, file_options).unwrap();
                    writer.write_all(bytes).unwrap();
                }
            }
            writer.finish().unwrap();
        }
        output.into_inner()
    }
}
