mod source;
mod r#virtual;

pub use r#virtual::{
    assign_virtual_bin_offsets, attach_mod_file_sizes, build_virtualization_table_from_assets,
    open_virtual_replacement, VirtualFile, VirtualHandle, VirtualHandleTable, VirtualManager,
    VirtualReplacement,
};
pub use source::{ModAsset, ReadSeek, ReplacementSource};
