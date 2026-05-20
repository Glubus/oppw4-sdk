#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkDataSectionError {
    InvalidHeader,
    InvalidSectionOffset { section: usize, offset: usize },
    SectionOutOfBounds { section: usize },
    RowOutOfBounds { section: usize, row: usize },
    RowSizeMismatch { expected: usize, actual: usize },
    InvalidRecordSize,
}

impl std::fmt::Display for LinkDataSectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeader => write!(formatter, "invalid LINKDATA entry section header"),
            Self::InvalidSectionOffset { section, offset } => write!(
                formatter,
                "invalid LINKDATA entry section offset section={section} offset=0x{offset:x}"
            ),
            Self::SectionOutOfBounds { section } => {
                write!(formatter, "LINKDATA entry section {section} out of bounds")
            }
            Self::RowOutOfBounds { section, row } => {
                write!(
                    formatter,
                    "LINKDATA entry row out of bounds section={section} row={row}"
                )
            }
            Self::RowSizeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "LINKDATA row size mismatch expected={expected} actual={actual}"
                )
            }
            Self::InvalidRecordSize => write!(formatter, "invalid LINKDATA row record size"),
        }
    }
}

impl std::error::Error for LinkDataSectionError {}
