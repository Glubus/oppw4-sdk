#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkDataError {
    InvalidHeader,
    TruncatedTable { entry: usize },
    OutOfBounds { entry: u32 },
    TruncatedChunk { entry: u32 },
    InflateFailed { entry: u32, message: String },
    ReadOutOfBounds { offset: usize },
}

impl std::fmt::Display for LinkDataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeader => write!(formatter, "invalid LINKDATA header"),
            Self::TruncatedTable { entry } => {
                write!(formatter, "truncated LINKDATA table at entry {entry}")
            }
            Self::OutOfBounds { entry } => {
                write!(formatter, "LINKDATA entry {entry} out of bounds")
            }
            Self::TruncatedChunk { entry } => {
                write!(formatter, "LINKDATA entry {entry} has a truncated chunk")
            }
            Self::InflateFailed { entry, message } => {
                write!(
                    formatter,
                    "LINKDATA entry {entry} inflate failed: {message}"
                )
            }
            Self::ReadOutOfBounds { offset } => {
                write!(formatter, "LINKDATA read out of bounds at 0x{offset:x}")
            }
        }
    }
}

impl std::error::Error for LinkDataError {}
