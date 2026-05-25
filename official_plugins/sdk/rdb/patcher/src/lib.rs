use plugin_sdk::{HostApi, PluginError, PluginResult};

mod log;
mod lua;
mod mods;
mod patching;
mod provider;
mod rdb_tracker;
mod runtime;
mod state;

pub(crate) const LEGACY_NAME_HASH_CATALOG_ZIP: &[u8] =
    include_bytes!("../../../../../resources/name_hash_catalog.zip");

pub fn initialize(host: HostApi<'_>) -> PluginResult<()> {
    log::initialize(host);
    log::write_line(format!(
        "skin_patcher sdk service initialized legacy_hash_catalog_zip_bytes={}",
        LEGACY_NAME_HASH_CATALOG_ZIP.len()
    ));
    lua::register(host);
    let code = runtime::initialize(host);
    if code == 0 {
        Ok(())
    } else {
        Err(PluginError::HostCallFailed {
            operation: "skin_patcher_initialize",
            code,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_legacy_name_hash_catalog() {
        assert!(LEGACY_NAME_HASH_CATALOG_ZIP.len() > 0x1000);
        assert_eq!(&LEGACY_NAME_HASH_CATALOG_ZIP[..2], b"PK");
    }
}
