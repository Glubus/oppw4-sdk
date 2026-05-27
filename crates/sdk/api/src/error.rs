pub type PluginResult<T> = Result<T, PluginError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginError {
    MissingHostFunction(&'static str),
    HostCallFailed { operation: &'static str, code: i32 },
    InvalidApiVersion { expected: u32, actual: u32 },
    ApiStructTooSmall { expected: u32, actual: u32 },
    InitFailed(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingHostFunction(name) => write!(formatter, "missing host function {name}"),
            Self::HostCallFailed { operation, code } => match host_call_code_reason(*code) {
                Some(reason) => write!(formatter, "{operation} failed with code {code} ({reason})"),
                None => write!(formatter, "{operation} failed with code {code}"),
            },
            Self::InvalidApiVersion { expected, actual } => {
                write!(
                    formatter,
                    "invalid plugin api version expected={expected} actual={actual}"
                )
            }
            Self::ApiStructTooSmall { expected, actual } => {
                write!(
                    formatter,
                    "plugin api struct too small expected_at_least={expected} actual={actual}"
                )
            }
            Self::InitFailed(message) => write!(formatter, "plugin init failed: {message}"),
        }
    }
}

fn host_call_code_reason(code: i32) -> Option<&'static str> {
    match code {
        -19 => Some("null SDK host context"),
        -20 => Some("missing plugin id"),
        -21 => Some("plugin id mismatch"),
        -22 => Some("missing manifest capability"),
        -23 => Some("missing registry module name"),
        -24 => Some("registry module not declared in manifest"),
        -25 => Some("missing capability name"),
        -26 => Some("missing config schema name"),
        -27 => Some("missing config schema body"),
        -28 => Some("duplicate config schema"),
        _ => None,
    }
}

impl std::error::Error for PluginError {}

impl From<String> for PluginError {
    fn from(message: String) -> Self {
        Self::InitFailed(message)
    }
}

impl From<crate::PluginInitError> for PluginError {
    fn from(error: crate::PluginInitError) -> Self {
        match error {
            crate::PluginInitError::NullApi => Self::InitFailed("null plugin api".to_string()),
            crate::PluginInitError::InvalidApiVersion { expected, actual } => {
                Self::InvalidApiVersion { expected, actual }
            }
            crate::PluginInitError::ApiStructTooSmall { expected, actual } => {
                Self::ApiStructTooSmall { expected, actual }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_call_display_includes_known_code_reason() {
        let error = PluginError::HostCallFailed {
            operation: "register_registry_module",
            code: -24,
        };

        assert_eq!(
            error.to_string(),
            "register_registry_module failed with code -24 (registry module not declared in manifest)"
        );
    }

    #[test]
    fn host_call_display_keeps_unknown_code_plain() {
        let error = PluginError::HostCallFailed {
            operation: "custom_operation",
            code: -999,
        };

        assert_eq!(error.to_string(), "custom_operation failed with code -999");
    }
}
