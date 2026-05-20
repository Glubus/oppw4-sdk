#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    Unknown,
    Booting,
    RdbLoading,
    RdbBinLoading,
    DlcCharacterLoading,
    VirtualResourceLoading,
    Other(u32),
}

impl GamePhase {
    pub fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::Unknown,
            1 => Self::Booting,
            2 => Self::RdbLoading,
            3 => Self::RdbBinLoading,
            4 => Self::DlcCharacterLoading,
            5 => Self::VirtualResourceLoading,
            other => Self::Other(other),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Booting => "booting",
            Self::RdbLoading => "rdb_loading",
            Self::RdbBinLoading => "rdb_bin_loading",
            Self::DlcCharacterLoading => "dlc_character_loading",
            Self::VirtualResourceLoading => "virtual_resource_loading",
            Self::Other(_) => "other",
        }
    }
}

pub fn phase_name(raw: u32) -> &'static str {
    GamePhase::from_raw(raw).as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_known_phases() {
        assert_eq!(phase_name(1), "booting");
        assert_eq!(phase_name(5), "virtual_resource_loading");
        assert_eq!(phase_name(42), "other");
    }
}
