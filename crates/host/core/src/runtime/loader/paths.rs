use std::{
    env,
    path::{Path, PathBuf},
};

const MODS_ROOT_ENV: &str = "OPPW4_SDK_MODS_ROOT";

pub(super) fn mods_root(game_root: &Path) -> PathBuf {
    env::var_os(MODS_ROOT_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| game_root.join("mods"))
}

pub(super) fn path_to_wide(path: &Path) -> Vec<u16> {
    let mut wide = path.to_string_lossy().encode_utf16().collect::<Vec<_>>();
    wide.push(0);
    wide
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn mods_root_uses_env_override_when_present() {
        let previous = env::var_os(MODS_ROOT_ENV);
        env::set_var(MODS_ROOT_ENV, "C:/tmp/sdkt-mods");

        let root = mods_root(Path::new("C:/game"));

        assert_eq!(root, PathBuf::from("C:/tmp/sdkt-mods"));

        match previous {
            Some(value) => env::set_var(MODS_ROOT_ENV, value),
            None => env::remove_var(MODS_ROOT_ENV),
        }
    }
}
