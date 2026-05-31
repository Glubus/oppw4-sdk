use std::ffi::{c_char, c_void, CStr};

use crate::runtime::core::player;

pub(super) unsafe extern "system" fn invoke(
    _module_context: *mut c_void,
    function_name_utf8: *const c_char,
    args_json: *const u8,
    args_json_len: usize,
    out_json: *mut u8,
    out_json_len: *mut usize,
) -> i32 {
    let Some(function_name) = cstr_to_str(function_name_utf8) else {
        return -40;
    };
    if bytes_to_str(args_json, args_json_len).is_none() {
        return -41;
    }
    let result = match function_name {
        "active_characters" => active_characters(),
        _ => Err(-42),
    };
    match result {
        Ok(json) => write_invoke_output(json.as_bytes(), out_json, out_json_len),
        Err(code) => code,
    }
}

fn active_characters() -> Result<String, i32> {
    let snapshot = player::latest_snapshot();
    Ok(serde_json::json!(snapshot
        .active_character_ids
        .iter()
        .map(|character| character.as_str())
        .collect::<Vec<_>>())
    .to_string())
}

unsafe fn cstr_to_str<'a>(value: *const c_char) -> Option<&'a str> {
    (!value.is_null()).then(|| CStr::from_ptr(value).to_str().ok())?
}

unsafe fn bytes_to_str<'a>(bytes: *const u8, len: usize) -> Option<&'a str> {
    if bytes.is_null() && len != 0 {
        return None;
    }
    let bytes = std::slice::from_raw_parts(bytes, len);
    std::str::from_utf8(bytes).ok()
}

unsafe fn write_invoke_output(bytes: &[u8], out_json: *mut u8, out_json_len: *mut usize) -> i32 {
    let Some(out_len) = out_json_len.as_mut() else {
        return -45;
    };
    if out_json.is_null() {
        *out_len = bytes.len();
        return 0;
    }
    if *out_len < bytes.len() {
        *out_len = bytes.len();
        return -46;
    }
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_json, bytes.len());
    *out_len = bytes.len();
    0
}

#[cfg(test)]
mod tests {
    use crate::runtime::core::player::{update_snapshot, PlayerSnapshot};

    #[test]
    fn player_invoke_returns_active_characters() {
        update_snapshot(
            PlayerSnapshot::new()
                .with_active_character("luffy")
                .with_active_character("zoro"),
        );
        let function = std::ffi::CString::new("active_characters").expect("function");
        let args = b"[]";
        let mut required_len = 0usize;

        let first = unsafe {
            super::invoke(
                std::ptr::null_mut(),
                function.as_ptr(),
                args.as_ptr(),
                args.len(),
                std::ptr::null_mut(),
                &mut required_len,
            )
        };

        assert_eq!(first, 0);
        let mut out = vec![0u8; required_len];
        let mut written_len = out.len();
        let second = unsafe {
            super::invoke(
                std::ptr::null_mut(),
                function.as_ptr(),
                args.as_ptr(),
                args.len(),
                out.as_mut_ptr(),
                &mut written_len,
            )
        };

        assert_eq!(second, 0);
        out.truncate(written_len);
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(value, serde_json::json!(["luffy", "zoro"]));
    }
}
