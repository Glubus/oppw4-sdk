use plugin_sdk::OwnedHostApi;

use crate::runtime::reader::{read_u32, read_usize};

const FIXED_ROOT_RVA: usize = 0x1eba738;
const FIXED_ID_TABLE_RVA: usize = 0x1e24ee0;
const FIXED_OWNER_OFFSET: usize = 0x18;
const FIXED_ID_COUNT: usize = 32;
const FIXED_POINTER_OFFSETS: [usize; 10] =
    [0x0, 0x8, 0x10, 0x18, 0x20, 0x28, 0x58, 0x60, 0xa0, 0xd8];
const FIXED_POINTER_HEAD_WORDS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FixedDataSnapshot {
    module_base: usize,
    root: usize,
    owner: usize,
    logical_ids: [u32; FIXED_ID_COUNT],
    pointers: Vec<FixedPointer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FixedPointer {
    offset: usize,
    value: usize,
    head: Vec<u32>,
}

impl FixedDataSnapshot {
    pub(super) fn format_log(&self) -> String {
        format!(
            "fixed_data_probe root=0x{:x} owner=0x{:x} logical_ids=[{}] pointers=[{}] heads=[{}]",
            self.root,
            self.owner,
            format_logical_ids(&self.logical_ids),
            format_pointers(&self.pointers),
            format_pointer_heads(&self.pointers),
        )
    }
}

pub(super) fn read(host: &OwnedHostApi) -> Result<FixedDataSnapshot, String> {
    let module_base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if module_base == 0 {
        return Err("module base is null".to_string());
    }

    let root = read_usize(host, module_base + FIXED_ROOT_RVA, "fixed_root")?;
    if root == 0 {
        return Err("fixed_root is null".to_string());
    }

    let owner = read_usize(host, root + FIXED_OWNER_OFFSET, "fixed_root+0x18")?;
    if owner == 0 {
        return Err("fixed owner is null".to_string());
    }

    Ok(FixedDataSnapshot {
        module_base,
        root,
        owner,
        logical_ids: read_logical_ids(host, module_base)?,
        pointers: read_fixed_pointers(host, owner),
    })
}

fn read_logical_ids(
    host: &OwnedHostApi,
    module_base: usize,
) -> Result<[u32; FIXED_ID_COUNT], String> {
    let mut values = [0u32; FIXED_ID_COUNT];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_u32(
            host,
            module_base + FIXED_ID_TABLE_RVA + index * size_of::<u32>(),
            "fixed_data_id_table",
        )?;
    }
    Ok(values)
}

fn read_fixed_pointers(host: &OwnedHostApi, owner: usize) -> Vec<FixedPointer> {
    FIXED_POINTER_OFFSETS
        .iter()
        .filter_map(|offset| {
            let value = read_usize(host, owner + offset, "fixed_owner_pointer").ok()?;
            Some(FixedPointer {
                offset: *offset,
                value,
                head: read_pointer_head(host, value),
            })
        })
        .collect()
}

fn read_pointer_head(host: &OwnedHostApi, address: usize) -> Vec<u32> {
    if address == 0 {
        return Vec::new();
    }

    (0..FIXED_POINTER_HEAD_WORDS)
        .filter_map(|index| read_u32(host, address + index * size_of::<u32>(), "fixed_head").ok())
        .collect()
}

fn format_logical_ids(values: &[u32; FIXED_ID_COUNT]) -> String {
    values
        .iter()
        .enumerate()
        .filter(|(_, value)| **value != 0)
        .map(|(index, value)| format!("{index}:0x{value:08x}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_pointers(pointers: &[FixedPointer]) -> String {
    pointers
        .iter()
        .map(|pointer| format!("+0x{:x}=0x{:x}", pointer.offset, pointer.value))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_pointer_heads(pointers: &[FixedPointer]) -> String {
    pointers
        .iter()
        .filter(|pointer| !pointer.head.is_empty())
        .map(|pointer| {
            let head = pointer
                .head
                .iter()
                .map(|value| format!("0x{value:08x}"))
                .collect::<Vec<_>>()
                .join(",");
            format!("+0x{:x}:[{}]", pointer.offset, head)
        })
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_only_nonzero_logical_ids() {
        let mut values = [0u32; FIXED_ID_COUNT];
        values[8] = 0x1234;
        values[20] = 0xabc;

        assert_eq!(format_logical_ids(&values), "8:0x00001234,20:0x00000abc");
    }

    #[test]
    fn formats_fixed_pointers_with_offsets() {
        let pointers = [
            FixedPointer {
                offset: 0x8,
                value: 0x1000,
                head: Vec::new(),
            },
            FixedPointer {
                offset: 0x20,
                value: 0x2000,
                head: Vec::new(),
            },
        ];

        assert_eq!(format_pointers(&pointers), "+0x8=0x1000,+0x20=0x2000");
    }

    #[test]
    fn formats_fixed_pointer_heads() {
        let pointers = [
            FixedPointer {
                offset: 0x8,
                value: 0x1000,
                head: vec![1, 2],
            },
            FixedPointer {
                offset: 0x20,
                value: 0x2000,
                head: Vec::new(),
            },
        ];

        assert_eq!(format_pointer_heads(&pointers), "+0x8:[0x00000001,0x00000002]");
    }
}
