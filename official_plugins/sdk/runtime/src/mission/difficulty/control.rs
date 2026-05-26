use std::{
    ffi::{c_char, c_void},
    sync::OnceLock,
};

use plugin_sdk::{DifficultyRule, OwnedHostApi, DIFFICULTY_SET_RULE};

use crate::runtime::probe::PLUGIN_ID;

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();

pub(crate) fn install(host: OwnedHostApi) {
    let _ = HOST.set(host.clone());
    unsafe {
        subscribe(&host, DIFFICULTY_SET_RULE);
    }
}

unsafe fn subscribe(host: &OwnedHostApi, signal: &str) {
    if let Err(error) =
        host.signals()
            .subscribe_bytes(signal, std::ptr::null_mut(), difficulty_command_callback)
    {
        let _ = host.log().write(
            PLUGIN_ID,
            format!("difficulty_runtime subscribe failed signal={signal}: {error}"),
        );
    }
}

unsafe extern "system" fn difficulty_command_callback(
    _subscriber_context: *mut c_void,
    signal_utf8: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    let Some(host) = HOST.get() else {
        return -1;
    };
    let signal = unsafe { std::ffi::CStr::from_ptr(signal_utf8) }.to_string_lossy();
    let bytes = if payload_len == 0 {
        &[]
    } else if payload.is_null() {
        return -2;
    } else {
        unsafe { std::slice::from_raw_parts(payload, payload_len) }
    };
    match handle_command(host, &signal, bytes) {
        Ok(()) => 0,
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("difficulty_runtime command failed signal={signal}: {error}"),
            );
            -3
        }
    }
}

fn handle_command(host: &OwnedHostApi, signal: &str, payload: &[u8]) -> Result<(), String> {
    match signal {
        DIFFICULTY_SET_RULE => stage_rule(host, read_json(payload)?),
        _ => Ok(()),
    }
}

fn stage_rule(host: &OwnedHostApi, rule: DifficultyRule) -> Result<(), String> {
    let levels = rule
        .levels
        .iter()
        .map(|level| level.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let _ = host.log().write(
        PLUGIN_ID,
        format!(
            "difficulty_runtime staged rule levels=[{levels}] action={:?} condition={:?} enabled={}",
            rule.action, rule.condition, rule.enabled
        ),
    );
    Ok(())
}

fn read_json<'a, T>(payload: &'a [u8]) -> Result<T, String>
where
    T: serde::Deserialize<'a>,
{
    serde_json::from_slice(payload).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use plugin_sdk::{DifficultyAction, DifficultyRule, DifficultyValueOp, DIFFICULTY_SET_RULE};

    use super::*;

    #[test]
    fn difficulty_payload_accepts_open_actor_stats() {
        let payload =
            serde_json::to_vec(&DifficultyRule::scale_actor_stat("hp", 1.5).level("super-hard"))
                .expect("json");
        let rule: DifficultyRule = read_json(&payload).expect("rule");

        assert_eq!(rule.levels[0].as_str(), "super_hard");
        assert_eq!(
            rule.action,
            DifficultyAction::ActorStat {
                stat: "hp".into(),
                operation: DifficultyValueOp::ScaleF32(1.5),
            }
        );
    }

    #[test]
    fn unknown_signals_are_ignored() {
        let payload = serde_json::to_vec(&DifficultyRule::enable_levels(["hard"])).expect("json");
        assert!(matches_signal("other.signal", &payload).is_ok());
        assert!(matches_signal(DIFFICULTY_SET_RULE, &payload).is_ok());
    }

    fn matches_signal(signal: &str, payload: &[u8]) -> Result<(), String> {
        match signal {
            DIFFICULTY_SET_RULE => {
                let _: DifficultyRule = read_json(payload)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
