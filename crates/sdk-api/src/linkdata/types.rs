pub const LINKDATA_MAGIC: u32 = 0x0007_7df9;
pub const LINKDATA_TABLE_OFFSET: usize = 0x10;
pub const LINKDATA_RECORD_SIZE: usize = 0x10;
pub const LINKDATA_OFFSET_GRANULARITY: usize = 0x100;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkDataFile {
    A = 0,
}

impl LinkDataFile {
    pub const fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::A),
            _ => None,
        }
    }

    pub const fn as_raw(self) -> u32 {
        self as u32
    }

    pub const fn file_name(self) -> &'static str {
        match self {
            Self::A => "LINKDATA_A.BIN",
        }
    }

    pub const fn relative_path(self) -> &'static str {
        match self {
            Self::A => "LINKDATA/CMN/LINKDATA_A.BIN",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkDataEntryId(pub u32);

impl LinkDataEntryId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

pub(crate) fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
