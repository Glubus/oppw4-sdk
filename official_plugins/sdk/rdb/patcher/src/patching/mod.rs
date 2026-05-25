#![allow(dead_code, unused_imports)]

mod source;
mod r#virtual;

pub use r#virtual::{
    assign_virtual_bin_offsets, attach_mod_file_sizes, build_virtualization_table,
    build_virtualization_table_from_assets, open_virtual_replacement, ReplacementMode, VirtualFile,
    VirtualHandle, VirtualHandleTable, VirtualManager, VirtualReplacement, VirtualReplacementFile,
};
pub use source::{ModAsset, ReadSeek, ReplacementSource};
