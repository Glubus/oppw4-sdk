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
            Self::HostCallFailed { operation, code } => {
                write!(formatter, "{operation} failed with code {code}")
            }
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
