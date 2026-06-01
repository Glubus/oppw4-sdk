use std::ffi::{c_char, c_void, CStr};

use crate::rewards;

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
    let Some(args_json) = bytes_to_str(args_json, args_json_len) else {
        return -41;
    };
    let result = match function_name {
        "set_reward_berry_total" => set_reward_berry_total(args_json),
        _ => Err(-42),
    };
    match result {
        Ok(json) => write_invoke_output(json.as_bytes(), out_json, out_json_len),
        Err(code) => code,
    }
}

fn set_reward_berry_total(args_json: &str) -> Result<String, i32> {
    let args = serde_json::from_str::<Vec<serde_json::Value>>(args_json).map_err(|_| -43)?;
    let total = args
        .first()
        .and_then(serde_json::Value::as_u64)
        .ok_or(-44)?;
    rewards::request_berry_total(total);
    Ok("null".to_string())
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
    #[test]
    fn set_reward_berry_total_accepts_u64_arg() {
        assert_eq!(
            super::set_reward_berry_total("[642]").expect("result"),
            "null"
        );
    }
}
