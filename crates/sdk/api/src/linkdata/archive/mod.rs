mod entry;
mod error;
mod inflate;
mod rebuild;
mod table;

use std::collections::{BTreeMap, HashMap};

use super::types::LinkDataEntryId;

pub use entry::LinkDataEntry;
pub use error::LinkDataError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkDataArchive {
    bytes: Vec<u8>,
    entries: Vec<LinkDataEntry>,
    entry_indexes: HashMap<LinkDataEntryId, usize>,
}

impl LinkDataArchive {
    pub fn parse(bytes: impl Into<Vec<u8>>) -> Result<Self, LinkDataError> {
        let bytes = bytes.into();
        let entries = table::parse_entries(&bytes)?;
        let entry_indexes = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.id, index))
            .collect();
        Ok(Self {
            bytes,
            entries,
            entry_indexes,
        })
    }

    pub fn entries(&self) -> &[LinkDataEntry] {
        &self.entries
    }

    pub fn entry_payload(&self, id: LinkDataEntryId) -> Result<Vec<u8>, LinkDataError> {
        let Some(entry) = self
            .entry_indexes
            .get(&id)
            .and_then(|index| self.entries.get(*index))
        else {
            return Err(LinkDataError::OutOfBounds { entry: id.get() });
        };
        inflate::inflate_entry(&self.bytes, entry)
    }

    pub fn rebuild_with_entry_payloads(
        &self,
        edits: &BTreeMap<LinkDataEntryId, Vec<u8>>,
    ) -> Result<Vec<u8>, LinkDataError> {
        rebuild::rebuild_with_entry_payloads(&self.bytes, &self.entries, edits)
    }
}

pub fn rebuild_raw_with_entry_payloads(
    bytes: &[u8],
    edits: &BTreeMap<LinkDataEntryId, Vec<u8>>,
) -> Result<Vec<u8>, LinkDataError> {
    LinkDataArchive::parse(bytes.to_vec())?.rebuild_with_entry_payloads(edits)
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, LinkDataError> {
    let Some(raw) = bytes.get(offset..offset + 4) else {
        return Err(LinkDataError::ReadOutOfBounds { offset });
    };
    Ok(u32::from_le_bytes(
        raw.try_into()
            .map_err(|_| LinkDataError::ReadOutOfBounds { offset })?,
    ))
}
