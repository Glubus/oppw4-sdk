use std::path::Path;

use plugin_sdk::linkdata::LinkDataFile;

use super::super::state::LinkDataState;
use super::logs;

pub(super) unsafe fn open_virtual_file(
    state: &mut LinkDataState,
    game_root: &Path,
    file: LinkDataFile,
    out_handle: *mut u64,
) -> i32 {
    let base_path = game_root.join(file.relative_path());
    match state.open(&base_path) {
        Ok(handle) => {
            *out_handle = handle;
            logs::virtual_opened(file, handle);
            1
        }
        Err(error) => {
            logs::virtual_open_failed(file, &error);
            0
        }
    }
}
