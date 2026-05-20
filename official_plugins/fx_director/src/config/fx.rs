#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetMode {
    All,
    LocalPlayer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FxConfig {
    pub(crate) enabled: bool,
    pub(crate) target: TargetMode,
    pub(crate) effect_id: u32,
    pub(crate) force_effect_id: bool,
    pub(crate) animation_speed: f32,
    pub(crate) loop_start: f32,
    pub(crate) loop_end: f32,
    pub(crate) required_character_ids: [u16; 4],
    pub(crate) required_character_id_count: u8,
}

impl Default for FxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target: TargetMode::All,
            effect_id: 2830,
            force_effect_id: true,
            animation_speed: 0.6,
            loop_start: 0.1,
            loop_end: 1.9,
            required_character_ids: [u16::MAX; 4],
            required_character_id_count: 0,
        }
    }
}

impl FxConfig {
    pub(crate) fn set_required_character_ids(&mut self, ids: impl IntoIterator<Item = u16>) {
        self.required_character_ids = [u16::MAX; 4];
        self.required_character_id_count = 0;
        for id in ids {
            if self.required_character_id_count as usize >= self.required_character_ids.len() {
                break;
            }
            if self.required_character_ids[..self.required_character_id_count as usize]
                .contains(&id)
            {
                continue;
            }
            self.required_character_ids[self.required_character_id_count as usize] = id;
            self.required_character_id_count += 1;
        }
    }

    pub(crate) fn accepts_active_character(&self, runtime_id: u16, alt_id: u16) -> bool {
        if self.required_character_id_count == 0 {
            return true;
        }
        self.required_character_ids[..self.required_character_id_count as usize]
            .iter()
            .any(|id| *id == runtime_id || *id == alt_id)
    }
}
