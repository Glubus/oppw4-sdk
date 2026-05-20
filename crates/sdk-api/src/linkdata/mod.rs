pub mod archive;
pub mod entries;
pub mod sections;
pub mod types;

pub use archive::{rebuild_raw_with_entry_payloads, LinkDataArchive, LinkDataEntry, LinkDataError};
pub use sections::{LinkDataEntrySections, LinkDataSectionError};
pub use types::{
    LinkDataEntryId, LinkDataFile, LINKDATA_MAGIC, LINKDATA_OFFSET_GRANULARITY,
    LINKDATA_RECORD_SIZE, LINKDATA_TABLE_OFFSET,
};

#[cfg(test)]
mod tests;
