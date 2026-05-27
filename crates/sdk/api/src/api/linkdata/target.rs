use crate::linkdata::{LinkDataEntryId, LinkDataFile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkDataRowTarget {
    pub file: LinkDataFile,
    pub entry: LinkDataEntryId,
    pub section: u32,
    pub record_size: u32,
    pub row: u32,
}

impl LinkDataRowTarget {
    pub const fn new(
        file: LinkDataFile,
        entry: LinkDataEntryId,
        section: u32,
        record_size: u32,
        row: u32,
    ) -> Self {
        Self {
            file,
            entry,
            section,
            record_size,
            row,
        }
    }
}
