use plugin_abi::{Oppw4ActiveCharacter, Oppw4GameStatus};

use crate::runtime::debug;

mod r#unsafe;

pub(crate) use r#unsafe::{host_active_character, host_debug_enabled, host_game_status};

fn write_game_status(out_status: &mut Oppw4GameStatus) {
    if crate::runtime::loader_services::read_game_status(out_status).is_some() {
        return;
    }
    *out_status = game_status_to_abi(hooks::game_status());
}

fn write_active_character(out: &mut Oppw4ActiveCharacter) {
    if crate::runtime::loader_services::read_active_character(out).is_some() {
        return;
    }
    *out = active_character_to_abi(hooks::active_character_snapshot());
}

fn debug_enabled() -> i32 {
    debug::enabled() as i32
}

fn game_status_to_abi(status: hooks::GameStatus) -> Oppw4GameStatus {
    Oppw4GameStatus {
        phase: status.phase,
        flags: status.flags,
        observed_file_opens: status.observed_file_opens,
        seconds_since_host_start: status.seconds_since_host_start,
    }
}

fn active_character_to_abi(snapshot: hooks::ActiveCharacter) -> Oppw4ActiveCharacter {
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
