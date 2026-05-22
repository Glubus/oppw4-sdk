use std::{sync::OnceLock, thread, time::Duration};

use super::hook;

static STARTED: OnceLock<()> = OnceLock::new();

pub(super) fn start() {
    if STARTED.set(()).is_err() {
        return;
    }

    let _ = thread::Builder::new()
        .name("oppw4_sdk_runtime".to_string())
        .spawn(|| {
            hook::install_local_player_hook();
            poll_active_character();
        });
}

fn poll_active_character() {
    let mut last_local_player = 0usize;
    loop {
        thread::sleep(Duration::from_millis(100));
        let local_player = hook::raw_local_player();
        if local_player == 0 || local_player == last_local_player {
            continue;
        }
        last_local_player = local_player;
        hooks::publish_local_player(local_player);
    }
}
