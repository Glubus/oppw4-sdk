use hooks::module_base;

use super::{
    GLOBAL_MERGE_DEFAULT_CALLER_RVA, GLOBAL_MERGE_MODE4_DIRECT_CALLER_RVA, RESULT_COUNT_CALLER_RVA,
    RESULT_TIME_CALLER_A_RVA, RESULT_TIME_CALLER_B_RVA,
};

pub(super) fn format_caller(caller: usize) -> String {
    let base = module_base();
    if caller >= base {
        format!("game+0x{:x}", caller - base)
    } else {
        format!("0x{caller:x}")
    }
}

pub(super) fn caller_rva(caller: usize) -> Option<usize> {
    let base = module_base();
    (caller >= base).then_some(caller - base)
}

pub(super) fn caller_label(caller: usize) -> &'static str {
    match caller_rva(caller) {
        Some(RESULT_TIME_CALLER_A_RVA) => "result_time_candidate_a",
        Some(RESULT_TIME_CALLER_B_RVA) => "result_time_candidate_b",
        Some(RESULT_COUNT_CALLER_RVA) => "result_defeated_count_candidate",
        Some(GLOBAL_MERGE_MODE4_DIRECT_CALLER_RVA) => "global_mode4_direct_merge",
        Some(GLOBAL_MERGE_DEFAULT_CALLER_RVA) => "global_default_merge",
        _ => "unknown",
    }
}

pub(super) fn result_label(result: u8) -> &'static str {
    result_label_i32(result.into())
}

pub(super) fn result_label_i32(result: i32) -> &'static str {
    match result {
        0 => "D",
        1 => "C",
        2 => "B",
        3 => "A",
        4 => "S",
        5 => "S+",
        _ => "unknown",
    }
}

pub(super) fn format_optional_offset(offset: Option<usize>) -> String {
    offset
        .map(|offset| format!("0x{offset:x}"))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_known_rank_results() {
        assert_eq!(result_label(0), "D");
        assert_eq!(result_label(3), "A");
        assert_eq!(result_label(5), "S+");
        assert_eq!(result_label(9), "unknown");
    }
}
