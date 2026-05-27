use std::{collections::HashSet, ffi::CStr, path::PathBuf};

use plugin_sdk::manifest::sanitize_plugin_id;

pub(crate) const CAP_FILES_VIRTUALIZE: &str = "files.virtualize";
pub(crate) const CAP_CONFIG_SCHEMA: &str = "config.schema";
pub(crate) const CAP_LINKDATA_PATCH: &str = "linkdata.patch";
pub(crate) const CAP_MEMORY_READ: &str = "memory.read";
pub(crate) const CAP_MEMORY_SCAN: &str = "memory.scan";
pub(crate) const CAP_MEMORY_WRITE: &str = "memory.write";
pub(crate) const CAP_RDB_PATCH: &str = "rdb.patch";
pub(crate) const CAP_SIGNALS_EMIT: &str = "signals.emit";
pub(crate) const CAP_SIGNALS_SUBSCRIBE: &str = "signals.subscribe";

pub(crate) struct ApiContext {
    pub(super) plugin_id: String,
    pub(super) mods_root: PathBuf,
    capabilities: HashSet<String>,
}

impl ApiContext {
    pub(crate) fn new(
        plugin_id: String,
        mods_root: PathBuf,
        capabilities: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            plugin_id,
            mods_root,
            capabilities: capabilities
                .into_iter()
                .map(|capability| capability.to_ascii_lowercase())
                .collect(),
        }
    }

    pub(crate) fn require_capability_for_cstr(
        &self,
        requested_plugin_id: Option<&CStr>,
        capability: &str,
    ) -> Result<(), i32> {
        let Some(requested_plugin_id) = requested_plugin_id else {
            return Err(-20);
        };
        self.require_capability_for_plugin_id(&requested_plugin_id.to_string_lossy(), capability)
    }

    pub(crate) fn require_capability_for_plugin_id(
        &self,
        requested_plugin_id: &str,
        capability: &str,
    ) -> Result<(), i32> {
        let requested_plugin_id = sanitize_plugin_id(requested_plugin_id);
        if !self.plugin_id.eq_ignore_ascii_case(&requested_plugin_id) {
            return Err(-21);
        }
        if !self.has_capability(capability) {
            return Err(-22);
        }
        Ok(())
    }

    pub(crate) fn require_capability(&self, capability: &str) -> Result<(), i32> {
        if !self.has_capability(capability) {
            return Err(-22);
        }
        Ok(())
    }

    pub(crate) fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_ascii_lowercase())
    }
}

pub(crate) unsafe fn context_from_raw<'a>(
    host_context: *mut std::ffi::c_void,
) -> Result<&'a ApiContext, i32> {
    host_context.cast::<ApiContext>().as_ref().ok_or(-19)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_check_rejects_plugin_id_spoofing() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            ["files.virtualize".to_string()],
        );

        assert_eq!(
            context.require_capability_for_plugin_id("fx_director", CAP_FILES_VIRTUALIZE),
            Err(-21)
        );
    }

    #[test]
    fn capability_check_rejects_missing_capability() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            Vec::<String>::new(),
        );

        assert_eq!(
            context.require_capability_for_plugin_id("skin_patcher", CAP_FILES_VIRTUALIZE),
            Err(-22)
        );
    }

    #[test]
    fn capability_check_accepts_declared_capability() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            ["files.virtualize".to_string()],
        );

        assert_eq!(
            context.require_capability_for_plugin_id("skin_patcher", CAP_FILES_VIRTUALIZE),
            Ok(())
        );
    }

    #[test]
    fn capability_check_normalizes_declared_capability() {
        let context = ApiContext::new(
            "fx_director".to_string(),
            "mods".into(),
            ["HOOKS.INSTALL".to_string()],
        );

        assert_eq!(
            context.require_capability_for_plugin_id("fx_director", "hooks.install"),
            Ok(())
        );
    }
}
