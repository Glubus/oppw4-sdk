use std::fmt::Write;

use super::snapshot::{
    ResultStateSnapshot, CREW_POINT_BLOCK_OFFSET, CREW_UNLOCK_WORDS, EVENT_WORDS,
};

impl ResultStateSnapshot {
    pub(super) fn format(&self, call: usize) -> String {
        format!(
            "result_state_probe call={call} ptr=0x{:x} state={} mission={} rank_fields=[{}] flags=0x{:x} result_copy=[{}] crew_points=[{}] crew_points_named={} source_rewards=[{}] soul_counter={} character_rewards=[{}] character_totals=[{}] crew_unlock_count={} crew_unlocks={} event_count={} events={}",
            self.address,
            self.state,
            self.mission_id,
            format_array(&self.difficulty_or_rank),
            self.unlock_flags,
            format_array(&self.result_copy),
            format_array(&self.crew_points),
            format_offset_block(&self.crew_points, CREW_POINT_BLOCK_OFFSET),
            format_array(&self.source_rewards),
            self.soul_counter,
            format_array(&self.character_rewards),
            format_array(&self.character_totals),
            self.crew_unlock_count,
            format_pairs(&self.crew_unlocks),
            self.event_count,
            format_events(&self.events),
        )
    }
}

fn format_array(values: &[u32]) -> String {
    values
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn format_offset_block(values: &[u32], start_offset: usize) -> String {
    let mut text = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "+0x{:x}:{}", start_offset + index * 4, value);
    }
    text
}

fn format_pairs(values: &[[u32; CREW_UNLOCK_WORDS]]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }

    let mut text = String::new();
    for (index, pair) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(text, "#{}:[{},{}]", index, pair[0], pair[1]);
    }
    text
}

fn format_events(values: &[[u32; EVENT_WORDS]]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }

    let mut text = String::new();
    for (index, event) in values.iter().enumerate() {
        if index > 0 {
            text.push(',');
        }
        let _ = write!(
            text,
            "#{}:[{},{},{},{},{}]",
            index, event[0], event[1], event[2], event[3], event[4]
        );
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_empty_collections() {
        assert_eq!(format_pairs(&[]), "none");
        assert_eq!(format_events(&[]), "none");
    }

    #[test]
    fn formats_pairs_and_events() {
        assert_eq!(format_pairs(&[[1, 2], [3, 4]]), "#0:[1,2],#1:[3,4]");
        assert_eq!(format_events(&[[1, 2, 3, 4, 5]]), "#0:[1,2,3,4,5]");
    }

    #[test]
    fn formats_offset_blocks() {
        assert_eq!(
            format_offset_block(&[12, 34, 56], 0x498),
            "+0x498:12,+0x49c:34,+0x4a0:56"
        );
    }
}
