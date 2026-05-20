pub mod costumes;
pub mod layout;
pub mod models;
pub mod movesets;
pub mod unknown;

pub use layout::{
    EntryLayoutStatus, LinkDataEntryKind, LinkDataEntryLayout, LinkDataLayoutSource,
    LinkDataSectionLayout,
};

pub const CURATED: &[LinkDataEntryLayout] = &[
    movesets::garp_entry_247::LAYOUT,
    movesets::rayleigh_entry_248::LAYOUT,
];
