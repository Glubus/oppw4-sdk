use std::fmt::Write;

use super::snapshot::{RankSlot, RankThresholdSnapshot};

impl RankThresholdSnapshot {
    pub(super) fn format_log(&self) -> String {
        format!(
            "rank_threshold_probe mission_id={} difficulty={} mode_type={} active_player={} global=0x{:x} fixed_rank_table=0x{:x} slots={}",
            self.mission_id,
            self.difficulty,
            self.mode_type,
            self.active_player,
            self.global,
            self.fixed_rank_table,
            self.slots
                .iter()
                .map(RankSlot::format_log)
                .collect::<Vec<_>>()
                .join(";"),
        )
    }
}

impl RankSlot {
    fn format_log(&self) -> String {
        format!(
            "p{}:rank_row={} raw=[{}] fixed=[{}] condition_row={} condition=[{}]",
            self.slot_index,
            self.rank_row_id,
            format_u16s(&self.raw_words),
            self.fixed_row_words
                .as_ref()
                .map_or_else(|| "unreadable".to_string(), |row| format_u16s(row)),
            self.condition_row_id
                .map_or_else(|| "none".to_string(), |value| value.to_string()),
            self.condition_row_words
                .as_ref()
                .map_or_else(|| "none".to_string(), |row| format_u16s(row)),
        )
    }
}

fn format_u16s(values: &[u16]) -> String {
    let mut text = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "+0x{:x}:{value}", index * 2);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_u16_offsets() {
        assert_eq!(format_u16s(&[7, 42, 65535]), "+0x0:7,+0x2:42,+0x4:65535");
    }
}
