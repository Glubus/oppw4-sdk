use crate::{api::r#unsafe, cstring_lossy, error::PluginError, PluginResult};
use plugin_abi::Oppw4PluginApi;

pub const CAP_PLUGIN_HOST: &str = "plugin.host";
pub const CAP_CONFIG_SCHEMA: &str = "config.schema";
pub const CAP_FILES_VIRTUALIZE: &str = "files.virtualize";
pub const CAP_HOOKS_INSTALL: &str = "hooks.install";
pub const CAP_LINKDATA_PATCH: &str = "linkdata.patch";
pub const CAP_LUA_MODULE: &str = "lua.module";
pub const CAP_LUA_RUNTIME: &str = "lua.runtime";
pub const CAP_MEMORY_READ: &str = "memory.read";
pub const CAP_MEMORY_SCAN: &str = "memory.scan";
pub const CAP_MEMORY_WRITE: &str = "memory.write";
pub const CAP_MOD_DISCOVERY: &str = "mod.discovery";
pub const CAP_RDB_PATCH: &str = "rdb.patch";
pub const CAP_SIGNALS_EMIT: &str = "signals.emit";
pub const CAP_SIGNALS_SUBSCRIBE: &str = "signals.subscribe";
pub const CAP_STD_CHARACTER_EXTEND: &str = "std.character.extend";

#[derive(Clone, Copy)]
pub struct CapabilityService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> CapabilityService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn require(self, plugin_id: &str, capability: &str) -> PluginResult<()> {
        let require = self
            .abi
            .require_capability
            .ok_or(PluginError::MissingHostFunction("require_capability"))?;
        let plugin_id = cstring_lossy(plugin_id);
        let capability = cstring_lossy(capability);
        let code = r#unsafe::require_capability(
            self.abi.host_context,
            require,
            plugin_id.as_c_str(),
            capability.as_c_str(),
        );
        if code == 0 {
            Ok(())
        } else {
            Err(PluginError::HostCallFailed {
                operation: "require_capability",
                code,
            })
        }
    }
}
