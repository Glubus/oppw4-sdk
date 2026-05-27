use super::types::{LinkDataEntryId, LinkDataFile};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedDataLogicalId(pub u32);

impl FixedDataLogicalId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedDataStreamRef {
    pub logical_id: FixedDataLogicalId,
    pub runtime_stream_id: u32,
}

impl FixedDataStreamRef {
    pub const fn new(logical_id: FixedDataLogicalId, runtime_stream_id: u32) -> Self {
        Self {
            logical_id,
            runtime_stream_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedDataSourceCandidate {
    pub logical_id: FixedDataLogicalId,
    pub file: LinkDataFile,
    pub entry: LinkDataEntryId,
}

impl FixedDataSourceCandidate {
    pub const fn new(
        logical_id: FixedDataLogicalId,
        file: LinkDataFile,
        entry: LinkDataEntryId,
    ) -> Self {
        Self {
            logical_id,
            file,
            entry,
        }
    }
}
