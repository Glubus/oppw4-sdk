use crate::RdbError;

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, RdbError> {
    let Some(slice) = bytes.get(offset..offset + 4) else {
        return Err(RdbError::TooSmall);
    };
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

pub(crate) fn read_c_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

pub(crate) fn align4(value: usize) -> usize {
    (value + 3) & !3
}

pub(crate) fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
