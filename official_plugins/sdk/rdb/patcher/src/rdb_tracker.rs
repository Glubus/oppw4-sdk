#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackedFileKind {
    Index,
    Data,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedRead {
    pub archive_name: String,
    pub kind: TrackedFileKind,
}

impl TrackedFileKind {
    pub fn label(self) -> &'static str {
        match self {
            TrackedFileKind::Index => "RDB",
            TrackedFileKind::Data => "RDB BIN",
        }
    }
}

pub fn tracked_read_from_path(path: &str) -> Option<TrackedRead> {
    let file_name = path.rsplit(['\\', '/']).find(|part| !part.is_empty())?;
    let (archive_name, kind) = tracked_archive_name(file_name)?;
    Some(TrackedRead { archive_name, kind })
}

fn tracked_archive_name(file_name: &str) -> Option<(String, TrackedFileKind)> {
    let lower = file_name.to_ascii_lowercase();
    if let Some(index) = lower.find(".rdb.bin") {
        if lower[index + ".rdb.bin".len()..]
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            return Some((file_name[..index].to_string(), TrackedFileKind::Data));
        }
    }
    if lower.ends_with(".rdb.bin") {
        return Some((
            file_name[..file_name.len() - ".rdb.bin".len()].to_string(),
            TrackedFileKind::Data,
        ));
    }
    if lower.ends_with(".rdb") {
        return Some((
            file_name[..file_name.len() - ".rdb".len()].to_string(),
            TrackedFileKind::Index,
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_rdb_and_rdb_bin_file_names() {
        assert_eq!(
            tracked_read_from_path(r"D:\Game\CharacterEditor.rdb"),
            Some(TrackedRead {
                archive_name: "CharacterEditor".to_string(),
                kind: TrackedFileKind::Index,
            })
        );
        assert_eq!(
            tracked_read_from_path(r"D:\Game\MaterialEditor.rdb.bin"),
            Some(TrackedRead {
                archive_name: "MaterialEditor".to_string(),
                kind: TrackedFileKind::Data,
            })
        );
        assert_eq!(
            tracked_read_from_path(r"D:\Game\ScreenLayout.rdb.bin10"),
            Some(TrackedRead {
                archive_name: "ScreenLayout".to_string(),
                kind: TrackedFileKind::Data,
            })
        );
        assert_eq!(tracked_read_from_path("not-an-archive.bin"), None);
    }
}
