use plugin_sdk::OwnedHostApi;
use serde::Serialize;

use super::probe::PLUGIN_ID;

pub(crate) const DIFFICULTY_SNAPSHOT: &str = "sdk.runtime.difficulty.snapshot";
pub(crate) const RANK_SNAPSHOT: &str = "sdk.runtime.rank.snapshot";
pub(crate) const RANK_HELPER_CALL: &str = "sdk.runtime.rank.helper_call";
pub(crate) const RESULT_STATE_SNAPSHOT: &str = "sdk.runtime.result_state.snapshot";
pub(crate) const REWARD_STAGE_RULE: &str = "sdk.runtime.rewards.stage_rule";
pub(crate) const REWARD_COMMIT: &str = "sdk.runtime.rewards.commit";
pub(crate) const REWARD_ITEMS: &str = "sdk.runtime.rewards.items";

pub(crate) fn emit_json<T: Serialize>(host: &OwnedHostApi, signal: &str, payload: &T) {
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
