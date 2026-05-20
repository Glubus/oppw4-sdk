use crate::linkdata::{LinkDataEntryId, LinkDataFile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkDataEntryKind {
    Moveset,
    Model,
    Costume,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryLayoutStatus {
    Generated,
    Observed,
    Partial,
    Confirmed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkDataLayoutSource {
    pub name: &'static str,
    pub note: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkDataEntryLayout {
    pub file: LinkDataFile,
    pub entry: LinkDataEntryId,
    pub kind: LinkDataEntryKind,
    pub name: &'static str,
    pub status: EntryLayoutStatus,
    pub source: Option<LinkDataLayoutSource>,
    pub sections: &'static [LinkDataSectionLayout],
}

impl LinkDataEntryLayout {
    pub const fn unknown(file: LinkDataFile, entry: LinkDataEntryId, name: &'static str) -> Self {
        Self {
            file,
            entry,
            kind: LinkDataEntryKind::Unknown,
            name,
            status: EntryLayoutStatus::Generated,
            source: None,
            sections: &[],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkDataSectionLayout {
    pub index: usize,
    pub name: &'static str,
    pub status: EntryLayoutStatus,
    pub byte_len: Option<usize>,
    pub record_size: Option<usize>,
}

impl LinkDataSectionLayout {
    pub const fn unknown(index: usize) -> Self {
        Self {
            index,
            name: "unknown",
            status: EntryLayoutStatus::Generated,
            byte_len: None,
            record_size: None,
        }
    }
}
