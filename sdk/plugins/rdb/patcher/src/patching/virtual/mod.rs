mod file;
mod handles;
mod manager;
mod table;

pub use file::{open_virtual_replacement, VirtualFile};
pub use handles::{VirtualHandle, VirtualHandleTable};
pub use manager::VirtualManager;
pub use table::{
    assign_virtual_bin_offsets, attach_mod_file_sizes, build_virtualization_table_from_assets,
    VirtualReplacement,
};
