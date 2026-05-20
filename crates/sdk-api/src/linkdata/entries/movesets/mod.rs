pub mod garp_entry_247;
pub mod rayleigh_entry_248;

use super::layout::LinkDataEntryLayout;

pub const KNOWN: &[LinkDataEntryLayout] = &[garp_entry_247::LAYOUT, rayleigh_entry_248::LAYOUT];
