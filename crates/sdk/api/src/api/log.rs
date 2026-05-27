use plugin_abi::Oppw4PluginApi;

use crate::{api::r#unsafe, cstring_lossy, error::PluginError, PluginResult};

#[derive(Clone, Copy)]
pub struct LogService<'api> {
    abi: &'api Oppw4PluginApi,
}

impl<'api> LogService<'api> {
    pub(super) const fn new(abi: &'api Oppw4PluginApi) -> Self {
        Self { abi }
    }

    pub fn write(self, plugin_id: &str, message: impl AsRef<str>) -> PluginResult<()> {
        let log = self
            .abi
            .log
            .ok_or(PluginError::MissingHostFunction("log"))?;
        let plugin_id = cstring_lossy(plugin_id);
        let message = cstring_lossy(message.as_ref());
        r#unsafe::host_log(self.abi.host_context, log, &plugin_id, &message);
        Ok(())
    }
}
