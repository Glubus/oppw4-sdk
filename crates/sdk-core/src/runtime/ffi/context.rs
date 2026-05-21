use std::{collections::HashSet, ffi::CStr, path::PathBuf};

use plugin_sdk::manifest::sanitize_plugin_id;

pub(crate) const CAP_FILES_VIRTUALIZE: &str = "files.virtualize";
pub(crate) const CAP_CONFIG_SCHEMA: &str = "config.schema";
pub(crate) const CAP_LINKDATA_PATCH: &str = "linkdata.patch";
pub(crate) const CAP_LUA_MODULE: &str = "lua.module";
pub(crate) const CAP_MEMORY_READ: &str = "memory.read";
pub(crate) const CAP_MEMORY_SCAN: &str = "memory.scan";
pub(crate) const CAP_MEMORY_WRITE: &str = "memory.write";
pub(crate) const CAP_RDB_PATCH: &str = "rdb.patch";
pub(crate) const CAP_SIGNALS_EMIT: &str = "signals.emit";
pub(crate) const CAP_SIGNALS_SUBSCRIBE: &str = "signals.subscribe";
pub(crate) const CAP_STD_CHARACTER_EXTEND: &str = "std.character.extend";

pub(crate) struct ApiContext {
    pub(super) plugin_id: String,
    pub(super) mods_root: PathBuf,
    capabilities: HashSet<String>,
    lua_modules: HashSet<String>,
}

impl ApiContext {
    pub(crate) fn new(
        plugin_id: String,
        mods_root: PathBuf,
        capabilities: impl IntoIterator<Item = String>,
        lua_modules: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            plugin_id,
            mods_root,
            capabilities: capabilities
                .into_iter()
                .map(|capability| capability.to_ascii_lowercase())
                .collect(),
            lua_modules: lua_modules
                .into_iter()
                .map(|module_name| module_name.to_ascii_lowercase())
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

    pub(crate) fn require_lua_module_registration(
        &self,
        requested_plugin_id: Option<&CStr>,
        module_name: Option<&CStr>,
    ) -> Result<(), i32> {
        self.require_capability_for_cstr(requested_plugin_id, CAP_LUA_MODULE)?;

        let Some(module_name) = module_name else {
            return Err(-23);
        };
        let module_name = module_name.to_string_lossy().to_ascii_lowercase();
        if !self.lua_modules.contains(&module_name) {
            return Err(-24);
        }
        Ok(())
    }

    pub(crate) fn allows_character_extension(&self) -> bool {
        self.has_capability(CAP_STD_CHARACTER_EXTEND)
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
            ["lua.module".to_string()],
            ["skin_patcher".to_string()],
        );

        assert_eq!(
            context.require_capability_for_plugin_id("fx_director", CAP_LUA_MODULE),
            Err(-21)
        );
    }

    #[test]
    fn capability_check_rejects_missing_capability() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            Vec::<String>::new(),
            Vec::<String>::new(),
        );

        assert_eq!(
            context.require_capability_for_plugin_id("skin_patcher", CAP_LUA_MODULE),
            Err(-22)
        );
    }

    #[test]
    fn capability_check_accepts_declared_capability() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            ["lua.module".to_string()],
            ["skin_patcher".to_string()],
        );

        assert_eq!(
            context.require_capability_for_plugin_id("skin_patcher", CAP_LUA_MODULE),
            Ok(())
        );
    }

    #[test]
    fn capability_check_normalizes_declared_capability() {
        let context = ApiContext::new(
            "fx_director".to_string(),
            "mods".into(),
            ["HOOKS.INSTALL".to_string()],
            Vec::<String>::new(),
        );

        assert_eq!(
            context.require_capability_for_plugin_id("fx_director", "hooks.install"),
            Ok(())
        );
    }

    #[test]
    fn lua_module_registration_accepts_declared_module() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            ["lua.module".to_string()],
            ["skin_patcher".to_string()],
        );
        let plugin_id = c"skin_patcher";
        let module_name = c"skin_patcher";

        assert_eq!(
            context.require_lua_module_registration(Some(plugin_id), Some(module_name)),
            Ok(())
        );
    }

    #[test]
    fn lua_module_registration_rejects_undeclared_module() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            ["lua.module".to_string()],
            ["skin_patcher".to_string()],
        );
        let plugin_id = c"skin_patcher";
        let module_name = c"shared";

        assert_eq!(
            context.require_lua_module_registration(Some(plugin_id), Some(module_name)),
            Err(-24)
        );
    }

    #[test]
    fn lua_module_registration_rejects_missing_module_name() {
        let context = ApiContext::new(
            "skin_patcher".to_string(),
            "mods".into(),
            ["lua.module".to_string()],
            ["skin_patcher".to_string()],
        );
        let plugin_id = c"skin_patcher";

        assert_eq!(
            context.require_lua_module_registration(Some(plugin_id), None),
            Err(-23)
        );
    }
}
