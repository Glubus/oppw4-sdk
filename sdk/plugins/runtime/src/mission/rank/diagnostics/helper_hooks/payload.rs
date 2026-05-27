use hooks::module_base;
use serde::Serialize;

use super::{
    labels::{caller_rva, format_caller, result_label},
    memory::{rank_row_offset, read_fixed_score_table, read_process_i32},
    row::RankHelperRow,
    MERGE_GRADE_COUNT, MERGE_GRADE_TARGET_INDEX_RVA, MERGE_RANK_SCORE_INDEX_RVA,
    MERGE_SCORE_OFFSET, RANK_THRESHOLD_COUNT, SLOT_COUNT,
};

#[derive(Clone, Debug, Serialize)]
pub(super) struct RankHelperCallSignal {
    pub(super) kind: &'static str,
    pub(super) caller: String,
    pub(super) caller_rva: Option<usize>,
    pub(super) caller_label: &'static str,
    pub(super) row: usize,
    pub(super) row_offset: Option<usize>,
    pub(super) slot: usize,
    pub(super) selectors: [u16; SLOT_COUNT],
    pub(super) thresholds: [u32; RANK_THRESHOLD_COUNT],
    pub(super) all_thresholds: [[u32; RANK_THRESHOLD_COUNT]; SLOT_COUNT],
    pub(super) value_f32: Option<f32>,
    pub(super) value_u32: Option<u32>,
    pub(super) divisor: Option<f32>,
    pub(super) normalized: Option<u32>,
    pub(super) result: u8,
    pub(super) result_label: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct RankMergeCallSignal {
    pub(super) kind: &'static str,
    pub(super) caller: String,
    pub(super) caller_rva: Option<usize>,
    pub(super) caller_label: &'static str,
    pub(super) left_rank: i32,
    pub(super) left_label: &'static str,
    pub(super) right_rank: i32,
    pub(super) right_label: &'static str,
    pub(super) result: i32,
    pub(super) result_label: &'static str,
    pub(super) score: Option<MergeScoreSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct MergeScoreSnapshot {
    pub(super) left_score_index: i32,
    pub(super) right_score_index: i32,
    pub(super) left_score: u32,
    pub(super) right_score: u32,
    pub(super) combined_score: u32,
    pub(super) grade_score_indexes: [i32; MERGE_GRADE_COUNT],
    pub(super) grade_targets: [u32; MERGE_GRADE_COUNT],
}

impl RankHelperCallSignal {
    pub(super) fn time(
        caller: usize,
        caller_label: &'static str,
        row: usize,
        value: f32,
        result: u8,
        snapshot: RankHelperRow,
    ) -> Self {
        Self {
            kind: "time",
            caller: format_caller(caller),
            caller_rva: caller_rva(caller),
            caller_label,
            row,
            row_offset: rank_row_offset(row),
            slot: snapshot.slot,
            selectors: snapshot.selectors,
            thresholds: snapshot.thresholds,
            all_thresholds: snapshot.all_thresholds,
            value_f32: Some(value),
            value_u32: None,
            divisor: None,
            normalized: None,
            result,
            result_label: result_label(result),
        }
    }

    pub(super) fn count(
        caller: usize,
        caller_label: &'static str,
        row: usize,
        value: u32,
        divisor: f32,
        normalized: u32,
        result: u8,
        snapshot: RankHelperRow,
    ) -> Self {
        Self {
            kind: "count",
            caller: format_caller(caller),
            caller_rva: caller_rva(caller),
            caller_label,
            row,
            row_offset: rank_row_offset(row),
            slot: snapshot.slot,
            selectors: snapshot.selectors,
            thresholds: snapshot.thresholds,
            all_thresholds: snapshot.all_thresholds,
            value_f32: None,
            value_u32: Some(value),
            divisor: Some(divisor),
            normalized: Some(normalized),
            result,
            result_label: result_label(result),
        }
    }
}

impl MergeScoreSnapshot {
    pub(super) fn read(left_rank: i32, right_rank: i32) -> Option<Self> {
        let base = module_base();
        let fixed_score_table = read_fixed_score_table().ok()?;
        let left_score_index = read_rank_score_index(base, left_rank).ok()?;
        let right_score_index = read_rank_score_index(base, right_rank).ok()?;
        let left_score = read_scaled_score(fixed_score_table, left_score_index).ok()?;
        let right_score = read_scaled_score(fixed_score_table, right_score_index).ok()?;
        let grade_score_indexes =
            std::array::from_fn(|grade| read_grade_target_index(base, grade).unwrap_or_default());
        let grade_targets = std::array::from_fn(|grade| {
            read_scaled_score(fixed_score_table, grade_score_indexes[grade]).unwrap_or_default()
        });

        Some(Self {
            left_score_index,
            right_score_index,
            left_score,
            right_score,
            combined_score: left_score + right_score,
            grade_score_indexes,
            grade_targets,
        })
    }
}

fn read_rank_score_index(base: usize, rank: i32) -> Result<i32, String> {
    if !(0..=6).contains(&rank) {
        return Err(format!("rank out of score index range: {rank}"));
    }
    read_process_i32(base + MERGE_RANK_SCORE_INDEX_RVA + rank as usize * size_of::<i32>())
}

fn read_grade_target_index(base: usize, grade: usize) -> Result<i32, String> {
    read_process_i32(base + MERGE_GRADE_TARGET_INDEX_RVA + grade * size_of::<i32>())
}

fn read_scaled_score(fixed_score_table: usize, score_index: i32) -> Result<u32, String> {
    if score_index < 0 {
        return Err(format!("negative score index: {score_index}"));
    }
    let raw = read_process_i32(
        fixed_score_table + MERGE_SCORE_OFFSET + score_index as usize * size_of::<i32>(),
    )?;
    Ok((raw as f32 * 0.001) as u32)
}
