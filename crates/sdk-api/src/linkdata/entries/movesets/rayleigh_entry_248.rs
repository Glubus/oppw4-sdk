use crate::linkdata::{
    entries::layout::{
        EntryLayoutStatus, LinkDataEntryKind, LinkDataEntryLayout, LinkDataLayoutSource,
    },
    LinkDataEntryId, LinkDataFile,
};

pub const LAYOUT: LinkDataEntryLayout = LinkDataEntryLayout {
    file: LinkDataFile::A,
    entry: LinkDataEntryId::new(248),
    kind: LinkDataEntryKind::Moveset,
    name: "rayleigh_movesets",
    status: EntryLayoutStatus::Observed,
    source: Some(LinkDataLayoutSource {
        name: "moveset modpack diff",
        note: "entry 248 is currently treated as Rayleigh's moveset entry; section layout not mapped yet",
    }),
    sections: &[],
};
