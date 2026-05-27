pub(super) fn difficulty_label(value: u8) -> &'static str {
    match value {
        0 => "easy",
        1 => "normal",
        2 => "hard",
        3 => "super_hard",
        _ => "unknown",
    }
}

pub(super) fn mode_type_label(value: u8) -> &'static str {
    match value {
        0 => "story",
        1 => "free_log",
        2 => "treasure_log",
        3 => "unknown_dlc_or_special_3",
        4 => "unknown_dlc_or_special_4",
        5 => "unknown_dlc_or_special_5",
        6 => "inactive_or_transition",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_known_difficulty_values() {
        assert_eq!(difficulty_label(0), "easy");
        assert_eq!(difficulty_label(1), "normal");
        assert_eq!(difficulty_label(2), "hard");
        assert_eq!(difficulty_label(3), "super_hard");
        assert_eq!(difficulty_label(4), "unknown");
    }

    #[test]
    fn labels_observed_mode_types() {
        assert_eq!(mode_type_label(0), "story");
        assert_eq!(mode_type_label(1), "free_log");
        assert_eq!(mode_type_label(2), "treasure_log");
        assert_eq!(mode_type_label(6), "inactive_or_transition");
        assert_eq!(mode_type_label(9), "unknown");
    }
}
