use super::types::{LargeInteger, Lpcwstr, Lpvoid};

pub(crate) unsafe fn read_overlapped_offset(overlapped: Lpvoid) -> LargeInteger {
    let bytes = overlapped as *const u8;
    *(bytes.add(16) as *const LargeInteger)
}

pub(crate) unsafe fn read_overlapped_event(overlapped: Lpvoid) -> usize {
    let bytes = overlapped as *const u8;
    *(bytes.add(24) as *const usize)
}

pub(crate) fn wide_path_to_string(path: Lpcwstr) -> Option<String> {
    if path.is_null() {
        return None;
    }
    let mut len = 0;
    unsafe {
        while *path.add(len) != 0 {
            len += 1;
        }
        Some(String::from_utf16_lossy(std::slice::from_raw_parts(
            path, len,
        )))
    }
}

pub(crate) fn hex_preview(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn extracts_file_name_from_wide_path() {
        let mut path: Vec<u16> = "D:\\Game\\OPPW4_PATCHER\\CharacterEditor\\MDLC038_Zoro_Wa.g1m"
            .encode_utf16()
            .collect();
        path.push(0);

        let file_name = wide_path_to_string(path.as_ptr()).and_then(|text| {
            Path::new(&text)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });

        assert_eq!(file_name.as_deref(), Some("MDLC038_Zoro_Wa.g1m"));
    }
}
