use std::{
    fmt,
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::SystemTime,
};

use plugin_sdk::zip::{read_zip_entry, zip_entry_size};

pub trait ReadSeek: Read + Seek {}

impl<T> ReadSeek for T where T: Read + Seek {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReplacementSource {
    File(PathBuf),
    ZipEntry {
        zip_path: PathBuf,
        entry_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModAsset {
    pub file_name: String,
    pub source: ReplacementSource,
}

impl ReplacementSource {
    pub fn display_name(&self) -> String {
        match self {
            Self::File(path) => path.display().to_string(),
            Self::ZipEntry {
                zip_path,
                entry_name,
            } => format!("{}!{}", zip_path.display(), entry_name),
        }
    }

    pub fn backing_path(&self) -> &Path {
        match self {
            Self::File(path) => path,
            Self::ZipEntry { zip_path, .. } => zip_path,
        }
    }

    pub fn payload_size(&self) -> std::io::Result<u64> {
        match self {
            Self::File(path) => std::fs::metadata(path).map(|metadata| metadata.len()),
            Self::ZipEntry {
                zip_path,
                entry_name,
            } => zip_entry_size(zip_path, entry_name),
        }
    }

    pub fn modified_time(&self) -> Option<SystemTime> {
        std::fs::metadata(self.backing_path())
            .ok()
            .and_then(|metadata| metadata.modified().ok())
    }

    pub fn open_reader(&self) -> std::io::Result<Box<dyn ReadSeek + Send>> {
        match self {
            Self::File(path) => Ok(Box::new(File::open(path)?)),
            Self::ZipEntry {
                zip_path,
                entry_name,
            } => Ok(Box::new(Cursor::new(read_zip_entry(zip_path, entry_name)?))),
        }
    }

    pub fn read_range(&self, offset: u64, target: &mut [u8]) -> std::io::Result<()> {
        let mut reader = self.open_reader()?;
        reader.seek(SeekFrom::Start(offset))?;
        reader.read_exact(target)
    }
}

impl fmt::Display for ReplacementSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn zip_entry_source_reports_size_and_reads_ranges() {
        let root = temp_root("zip-source");
        let zip_path = root.join("mod.zip");
        write_zip(&zip_path, "ScreenLayout/portrait.g1t", b"abcdef");
        let source = ReplacementSource::ZipEntry {
            zip_path: zip_path.clone(),
            entry_name: "ScreenLayout/portrait.g1t".to_string(),
        };
        let mut range = [0u8; 3];

        assert_eq!(source.payload_size().unwrap(), 6);
        source.read_range(2, &mut range).unwrap();
        assert_eq!(&range, b"cde");
        assert_eq!(source.backing_path(), zip_path.as_path());

        let _ = std::fs::remove_dir_all(root);
    }

    fn temp_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "oppw4-replacement-source-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_zip(path: &Path, name: &str, bytes: &[u8]) {
        let file = File::create(path).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
        writer.finish().unwrap();
    }
}
