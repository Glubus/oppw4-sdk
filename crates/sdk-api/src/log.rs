#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogPolicy {
    pub host: bool,
}

impl LogPolicy {
    pub const HOST: Self = Self { host: true };
    pub const SILENT: Self = Self { host: false };
}

pub fn mirror_mod_log_to_host(level: &str) -> bool {
    matches!(level, "warn" | "error")
}

use crate::{HostApi, OwnedHostApi, PluginResult};

#[derive(Clone)]
pub struct PluginLogger {
    plugin_id: &'static str,
    host: OwnedHostApi,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_log_host_mirror_keeps_warnings_and_errors_only() {
        assert!(!mirror_mod_log_to_host("debug"));
        assert!(!mirror_mod_log_to_host("info"));
        assert!(mirror_mod_log_to_host("warn"));
        assert!(mirror_mod_log_to_host("error"));
    }
}

impl PluginLogger {
    pub fn new(plugin_id: &'static str, host: HostApi<'_>) -> Self {
        Self {
            plugin_id,
            host: host.owned(),
        }
    }

    pub const fn plugin_id(&self) -> &'static str {
        self.plugin_id
    }

    pub fn try_write_line(&self, message: impl AsRef<str>) -> PluginResult<()> {
        self.host.log().write(self.plugin_id, message)
    }

    pub fn write_line(&self, message: impl AsRef<str>) {
        let _ = self.try_write_line(message);
    }
}
