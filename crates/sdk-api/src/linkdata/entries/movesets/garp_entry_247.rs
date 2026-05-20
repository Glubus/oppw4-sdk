use crate::linkdata::{
    entries::layout::{
        EntryLayoutStatus, LinkDataEntryKind, LinkDataEntryLayout, LinkDataLayoutSource,
    },
    LinkDataEntryId, LinkDataFile,
};

pub const LAYOUT: LinkDataEntryLayout = LinkDataEntryLayout {
    file: LinkDataFile::A,
    entry: LinkDataEntryId::new(247),
    kind: LinkDataEntryKind::Moveset,
    name: "garp_movesets",
    status: EntryLayoutStatus::Observed,
    source: Some(LinkDataLayoutSource {
        name: "moveset modpack diff",
        note:
            "entry 247 is currently treated as Garp's moveset entry; section layout not mapped yet",
    }),
    sections: &[],
};
