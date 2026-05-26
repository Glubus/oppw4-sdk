use std::ffi::c_void;

use plugin_sdk::Oppw4ActiveCharacter;

use super::state::{self, ActiveCharacter};

pub(crate) unsafe extern "system" fn read_active_character(
    _provider_context: *mut c_void,
    out: *mut Oppw4ActiveCharacter,
) -> i32 {
    let Some(out) = out.as_mut() else {
        return -1;
    };
    *out = active_character_to_abi(state::snapshot());
    0
}

fn active_character_to_abi(snapshot: ActiveCharacter) -> Oppw4ActiveCharacter {
    Oppw4ActiveCharacter {
        runtime_id: snapshot.runtime_id,
        alt_id: snapshot.alt_id,
        flags: snapshot.flags,
        local_player: snapshot.local_player,
        fx_owner: snapshot.fx_owner,
        source: snapshot.source,
        sequence: snapshot.sequence,
    }
}
