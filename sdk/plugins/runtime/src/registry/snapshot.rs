use std::ffi::{c_char, c_void, CStr};

use crate::runtime::core::{difficulty, player};

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
        "mission" => mission(),
        "difficulty" => difficulty(),
        "player" => player(),
        _ => Err(-42),
    };
    match result {
        Ok(json) => write_invoke_output(json.as_bytes(), out_json, out_json_len),
        Err(code) => code,
    }
}

fn mission() -> Result<String, i32> {
    let snapshot = difficulty::latest_snapshot();
    Ok(serde_json::json!({
        "id": snapshot.as_ref().and_then(|value| value.mission_id),
        "mode": snapshot.as_ref().map(|value| value.mode.key()),
    })
    .to_string())
}

fn difficulty() -> Result<String, i32> {
    let snapshot = difficulty::latest_snapshot();
    Ok(serde_json::json!({
        "key": snapshot.as_ref().map(|value| value.difficulty.key()),
    })
    .to_string())
}

fn player() -> Result<String, i32> {
    let snapshot = player::latest_snapshot();
    Ok(serde_json::json!({
        "active_character_ids": snapshot
            .active_character_ids
            .iter()
            .map(|character| character.as_str())
            .collect::<Vec<_>>(),
    })
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
    use crate::runtime::core::{
        difficulty::{
            update_snapshot as update_difficulty_snapshot, DifficultyId, DifficultyMode,
            DifficultySnapshot,
        },
        player::{test_snapshot_lock, update_snapshot as update_player_snapshot, PlayerSnapshot},
    };

    #[test]
    fn snapshot_invoke_returns_runtime_state() {
        let _lock = test_snapshot_lock();
        update_difficulty_snapshot(
            DifficultySnapshot::new(DifficultyMode::new("free_log"), DifficultyId::new("hard"))
                .with_mission_id(35),
        );
        update_player_snapshot(PlayerSnapshot::new().with_active_character("zoro"));

        assert_eq!(
            super::mission().expect("mission"),
            r#"{"id":35,"mode":"free_log"}"#
        );
        assert_eq!(
            super::difficulty().expect("difficulty"),
            r#"{"key":"hard"}"#
        );
        assert_eq!(
            super::player().expect("player"),
            r#"{"active_character_ids":["zoro"]}"#
        );
    }
}
