use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use super::probe::PLUGIN_ID;

pub(crate) const DIFFICULTY_SNAPSHOT: &str = "sdk.runtime.difficulty.snapshot";
pub(crate) const DIFFICULTY_EVENT: &str = "sdk.runtime.difficulty.event";
pub(crate) const RANK_SNAPSHOT: &str = "sdk.runtime.rank.snapshot";
pub(crate) const RANK_EVENT: &str = "sdk.runtime.rank.event";
pub(crate) const RANK_HELPER_CALL: &str = "sdk.runtime.rank.helper_call";
pub(crate) const RESULT_STATE_SNAPSHOT: &str = "sdk.runtime.result_state.snapshot";
pub(crate) const REWARD_COMMIT: &str = "sdk.runtime.rewards.commit";
pub(crate) const REWARD_EVENT: &str = "sdk.runtime.rewards.event";
pub(crate) const REWARD_MEDALS: &str = "sdk.runtime.rewards.medals";

pub(crate) fn emit_json<T: Serialize>(host: &OwnedHostApi, signal: &str, payload: &T) {
    if !host.signals().has_listeners(signal) {
        return;
    }
    match serde_json::to_vec(payload) {
        Ok(bytes) => {
            let _ = host.signals().emit_bytes(signal, &bytes);
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("runtime signal serialize failed signal={signal} error={error}"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::*;

    #[derive(Serialize)]
    struct RewardEventTestPayload<'a> {
        rank: &'a str,
        berry: u64,
        mutations: Vec<&'a str>,
    }

    #[test]
    fn reward_event_signal_name_is_stable() {
        assert_eq!(REWARD_EVENT, "sdk.runtime.rewards.event");
    }

    #[test]
    fn reward_event_payload_shape_is_json_serializable() {
        let payload = RewardEventTestPayload {
            rank: "S+",
            berry: 321,
            mutations: vec!["multiply_berry"],
        };

        let json = serde_json::to_string(&payload).expect("json");

        assert_eq!(
            json,
            r#"{"rank":"S+","berry":321,"mutations":["multiply_berry"]}"#
        );
    }
}
