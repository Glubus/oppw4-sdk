#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DifficultyId {
    Easy,
    Normal,
    Hard,
    SuperHard,
}

impl DifficultyId {
    pub(crate) const fn as_u8(self) -> u8 {
        match self {
            Self::Easy => 0,
            Self::Normal => 1,
            Self::Hard => 2,
            Self::SuperHard => 3,
        }
    }
}

impl TryFrom<u8> for DifficultyId {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Easy),
            1 => Ok(Self::Normal),
            2 => Ok(Self::Hard),
            3 => Ok(Self::SuperHard),
            _ => Err(format!("unknown difficulty id {value}")),
        }
    }
}
