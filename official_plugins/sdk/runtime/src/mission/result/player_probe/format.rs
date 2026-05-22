use super::snapshot::{
    PlayerResultSnapshot, PLAYER_RESULT_STAT_OFFSETS, SAVE_MODE_TOTAL_OFFSETS,
    SAVE_RESULT_TOTAL_OFFSETS, SOUL_STATE_OFFSET, SOUL_STATE_WORDS,
};

impl PlayerResultSnapshot {
    pub(super) fn format_log(&self) -> String {
        let active_index = usize::from(self.active_player).min(3);
        format!(
            "player_result_probe mission_id={} difficulty={} mode_type={} active_player={} global=0x{:x} save=0x{:x} active_stats={} all_players={} save_totals={} save_mode_totals={} soul_state={}",
            self.mission_id,
            self.difficulty,
            self.mode_type,
            self.active_player,
            self.global,
            self.save,
            format_named_values(&PLAYER_RESULT_STAT_OFFSETS, &self.player_stats[active_index]),
            format_players(&self.player_stats),
            format_named_values(&SAVE_RESULT_TOTAL_OFFSETS, &self.save_totals),
            format_named_values(&SAVE_MODE_TOTAL_OFFSETS, &self.save_mode_totals),
            format_soul_state(&self.soul_state),
        )
    }
}

fn format_players(players: &[[u32; PLAYER_RESULT_STAT_OFFSETS.len()]; 4]) -> String {
    players
        .iter()
        .enumerate()
        .map(|(index, stats)| {
            format!(
                "p{}:[{}]",
                index,
                stats
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn format_named_values<const N: usize>(offsets: &[usize; N], values: &[u32; N]) -> String {
    offsets
        .iter()
        .zip(values)
        .map(|(offset, value)| format!("+0x{offset:x}:{value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_soul_state(values: &[u32; SOUL_STATE_WORDS]) -> String {
    values
        .iter()
        .enumerate()
        .filter(|(_, value)| **value != 0)
        .map(|(index, value)| format!("+0x{:x}:{value}", SOUL_STATE_OFFSET + index * 4))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_empty_soul_state_as_empty_string() {
        assert_eq!(format_soul_state(&[0; SOUL_STATE_WORDS]), "");
    }

    #[test]
    fn formats_named_offsets() {
        assert_eq!(
            format_named_values(&[0x8fc, 0x900], &[30, 200]),
            "+0x8fc:30,+0x900:200"
        );
    }
}
