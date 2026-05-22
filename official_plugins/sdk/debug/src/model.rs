#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DebugConfig {
    pub(crate) enabled: bool,
    pub(crate) interval_ms: u64,
    pub(crate) watches: Vec<Watch>,
    pub(crate) scans: Vec<Scan>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_ms: 500,
            watches: Vec::new(),
            scans: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Watch {
    pub(crate) id: String,
    pub(crate) value_type: ValueType,
    pub(crate) address: AddressSpec,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Scan {
    pub(crate) id: String,
    pub(crate) value_type: ValueType,
    pub(crate) start: AddressSpec,
    pub(crate) bytes: usize,
    pub(crate) values: Vec<TargetValue>,
    pub(crate) max_hits: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AddressSpec {
    Absolute(usize),
    ModuleRva(usize),
    PointerChain {
        base: Box<AddressSpec>,
        offsets: Vec<usize>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValueType {
    U8,
    U16,
    U32,
    I32,
    F32,
}

impl ValueType {
    pub(crate) fn width(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::U32 | Self::I32 | Self::F32 => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TargetValue {
    Integer(i64),
    Float(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WatchValue {
    pub(crate) address: usize,
    pub(crate) value_type: ValueType,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ScanHit {
    pub(crate) address: usize,
    pub(crate) offset: usize,
    pub(crate) value: TargetValue,
}
