use plugin_sdk::OwnedHostApi;

use crate::runtime::reader::{read_f32, read_u16, read_u8, read_usize};

const GLOBAL_ROOT_RVA: usize = 0x1eba750;
const FIXED_ROOT_RVA: usize = 0x1eba738;

const GLOBAL_OWNER_OFFSET: usize = 0x18;
const GLOBAL_AUX_OFFSET: usize = 0x10;
const GLOBAL_STATE_OFFSET: usize = 0x28;
const FIXED_OWNER_OFFSET: usize = 0x18;
const FIXED_DATA20_OFFSET: usize = 0x20;
const FIXED_DATA28_OFFSET: usize = 0x28;

const MISSION_ID_OFFSET: usize = 0x1d750;
const MODE_TYPE_OFFSET: usize = 0x1d753;
const REWARD_MODE_OFFSET: usize = 0x1d754;
const DIFFICULTY_OFFSET: usize = 0x1d756;
const SPECIAL_ROW_FORCE_ZERO_OFFSET: usize = 0x1d762;
const SPECIAL_ROW_CONTEXT_OFFSET: usize = 0xff60;

const BASE_MISSION_LIMIT: u16 = 0x00f9;
const SPECIAL_ROW_CONTEXT_LIMIT: u32 = 0x3c;
const DIFFICULTY_COUNT: usize = 4;
const BASE_REWARD_INDEX_OFFSET: usize = 0x00a8;
const BASE_REWARD_INDEX_STRIDE: usize = 0x006e;
const SPECIAL_REWARD_INDEX_OFFSET: usize = 0x46cba;
const SPECIAL_REWARD_INDEX_STRIDE: usize = 0x002c;

const B3_ROW_STRIDE: usize = 0x0c;
const B3_TABLES: [usize; 3] = [0xb3d8, 0xb3dc, 0xb3e0];
const B4_CHANCE_BASE: usize = 0xb4c8;
const B518_WEIGHTS_BASE: usize = 0xb518;
const B518_CANDIDATE_STRIDE: usize = 0x50;
const B518_DIFFICULTY_STRIDE: usize = 0x14;

const CANDIDATE_TABLE_A: usize = 0x6608;
const CANDIDATE_TABLE_B: usize = 0x1b08;
const CANDIDATE_STRIDE: usize = 0x1e0;
const CANDIDATE_DIFF_STRIDE: usize = 0x78;
const CANDIDATE_CATEGORY_STRIDE: usize = 0x28;

const CATEGORY_ROW_BASES: [usize; 3] = [0x1928, 0x1930, 0x1938];
const CATEGORY_ROW_STRIDE: usize = 0x18;

const PHASE_WEIGHT_BASES: [[usize; 3]; 3] = [
    [0xb114, 0xb116, 0xb118],
    [0xb10e, 0xb110, 0xb112],
    [0xb108, 0xb10a, 0xb10c],
];
const PHASE_WEIGHT_STRIDE: usize = 0x12;

const COOLDOWN_FLOAT_OFFSETS: [usize; 6] = [0xc644, 0xc648, 0xc64c, 0xc650, 0xc654, 0xc65c];
const BEHAVIOR_FLOAT_OFFSETS: [usize; 6] = [0xc57c, 0xc580, 0xc584, 0xc588, 0xc58c, 0xc590];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SpawnScalingSnapshot {
    global: usize,
    fixed20: usize,
    fixed28: usize,
    mission_id: u16,
    active_difficulty: u8,
    mode_type: u8,
    reward_mode: u8,
    special_context: SpecialRowContext,
    cooldowns: Vec<(usize, f32)>,
    behavior: Vec<(usize, f32)>,
    difficulties: Vec<DifficultyTables>,
    phase_weights: Vec<PhaseWeights>,
}

#[derive(Clone, Debug, PartialEq)]
struct DifficultyTables {
    difficulty: u8,
    base_row: Option<u16>,
    special_row: Option<u16>,
    row_source: RowSource,
    normalized_row: u16,
    b3_chances: [u8; 3],
    b4_chance: u8,
    b518_weights: WeightSummary,
    candidate_a: [WeightSummary; 3],
    candidate_b: [WeightSummary; 3],
    category_rows: [u16; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PhaseWeights {
    phase: u8,
    category_weights: [WeightSummary; 3],
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct WeightSummary {
    sum: u32,
    nonzero: usize,
    first: Vec<(usize, u16)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SpecialRowContext {
    index: Option<u32>,
    forced_zero: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowSource {
    Base,
    Special,
    None,
}

impl SpawnScalingSnapshot {
    pub(super) fn format_log(&self) -> String {
        format!(
            "spawn_scaling_probe mission_id={} active_difficulty={} mode_type={} reward_mode={} special_context={} global=0x{:x} fixed20=0x{:x} fixed28=0x{:x} cooldowns=[{}] behavior=[{}] difficulties=[{}] phase_weights=[{}]",
            self.mission_id,
            self.active_difficulty,
            self.mode_type,
            self.reward_mode,
            self.special_context.format_log(),
            self.global,
            self.fixed20,
            self.fixed28,
            format_f32_pairs(&self.cooldowns),
            format_f32_pairs(&self.behavior),
            self.difficulties
                .iter()
                .map(DifficultyTables::format_log)
                .collect::<Vec<_>>()
                .join(";"),
            self.phase_weights
                .iter()
                .map(PhaseWeights::format_log)
                .collect::<Vec<_>>()
                .join(";"),
        )
    }
}

impl DifficultyTables {
    fn format_log(&self) -> String {
        format!(
            "diff{}:source={} base_row={} special_row={} norm={} b3={}/{}/{} b4={} b518={} cand_a=[{}] cand_b=[{}] cat_rows={}/{}/{}",
            self.difficulty,
            self.row_source.format_log(),
            self.base_row
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.special_row
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.normalized_row,
            self.b3_chances[0],
            self.b3_chances[1],
            self.b3_chances[2],
            self.b4_chance,
            self.b518_weights.format_log(),
            format_category_summaries(&self.candidate_a),
            format_category_summaries(&self.candidate_b),
            self.category_rows[0],
            self.category_rows[1],
            self.category_rows[2],
        )
    }
}

impl PhaseWeights {
    fn format_log(&self) -> String {
        format!(
            "phase{}:[{}]",
            self.phase,
            format_category_summaries(&self.category_weights)
        )
    }
}

impl SpecialRowContext {
    fn format_log(self) -> String {
        let index = self
            .index
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string());
        format!("index={index},forced_zero={}", self.forced_zero)
    }
}

impl RowSource {
    fn format_log(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Special => "special",
            Self::None => "none",
        }
    }
}

impl WeightSummary {
    fn format_log(&self) -> String {
        let first = self
            .first
            .iter()
            .map(|(index, value)| format!("{index}:{value}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "sum={} nz={} first={}",
            self.sum,
            self.nonzero,
            if first.is_empty() { "-" } else { &first }
        )
    }
}

pub(super) fn read(
    host: &OwnedHostApi,
    max_candidates: usize,
) -> Result<SpawnScalingSnapshot, String> {
    let module_base = host
        .memory()
        .module_base()
        .map_err(|error| format!("module_base failed: {error}"))?;
    if module_base == 0 {
        return Err("module base is null".to_string());
    }

    let global_refs = read_global_refs(host, module_base)?;
    let fixed_owner = read_fixed_owner(host, module_base)?;
    let fixed20 = read_usize(host, fixed_owner + FIXED_DATA20_OFFSET, "fixed20")?;
    let fixed28 = read_usize(host, fixed_owner + FIXED_DATA28_OFFSET, "fixed28")?;
    if fixed20 == 0 || fixed28 == 0 {
        return Err("fixed spawn table pointer is null".to_string());
    }

    let mission_id = read_u16(host, global_refs.state + MISSION_ID_OFFSET, "mission_id")?;
    let active_difficulty = read_u8(host, global_refs.state + DIFFICULTY_OFFSET, "difficulty")?;
    let mode_type = read_u8(host, global_refs.state + MODE_TYPE_OFFSET, "mode_type")?;
    let special_context = read_special_context(host, global_refs)?;
    let candidate_count = max_candidates.min(40);

    Ok(SpawnScalingSnapshot {
        global: global_refs.state,
        fixed20,
        fixed28,
        mission_id,
        active_difficulty,
        mode_type,
        reward_mode: read_u8(host, global_refs.state + REWARD_MODE_OFFSET, "reward_mode")?,
        special_context,
        cooldowns: read_floats(host, fixed20, &COOLDOWN_FLOAT_OFFSETS),
        behavior: read_floats(host, fixed20, &BEHAVIOR_FLOAT_OFFSETS),
        difficulties: read_difficulty_tables(
            host,
            fixed20,
            fixed28,
            mission_id,
            mode_type,
            special_context,
            candidate_count,
        )?,
        phase_weights: read_phase_weights(host, fixed20, candidate_count)?,
    })
}

#[derive(Clone, Copy)]
struct GlobalRefs {
    aux: usize,
    state: usize,
}

fn read_global_refs(host: &OwnedHostApi, module_base: usize) -> Result<GlobalRefs, String> {
    let root = read_usize(host, module_base + GLOBAL_ROOT_RVA, "global_root")?;
    let owner = read_usize(host, root + GLOBAL_OWNER_OFFSET, "global_root+0x18")?;
    let aux = read_usize(host, owner + GLOBAL_AUX_OFFSET, "global_aux")?;
    let state = read_usize(host, owner + GLOBAL_STATE_OFFSET, "global_state")?;
    if state == 0 {
        Err("global_state is null".to_string())
    } else {
        Ok(GlobalRefs { aux, state })
    }
}

fn read_special_context(
    host: &OwnedHostApi,
    global: GlobalRefs,
) -> Result<SpecialRowContext, String> {
    let forced_zero = read_u8(
        host,
        global.state + SPECIAL_ROW_FORCE_ZERO_OFFSET,
        "special_row_force_zero",
    )? != 0;
    if forced_zero {
        return Ok(SpecialRowContext {
            index: Some(0),
            forced_zero,
        });
    }

    if global.aux == 0 {
        return Ok(SpecialRowContext {
            index: None,
            forced_zero,
        });
    }

    let index = crate::runtime::reader::read_u32(
        host,
        global.aux + SPECIAL_ROW_CONTEXT_OFFSET,
        "special_row_context",
    )
    .ok()
    .filter(|value| *value <= SPECIAL_ROW_CONTEXT_LIMIT);
    Ok(SpecialRowContext { index, forced_zero })
}

fn read_fixed_owner(host: &OwnedHostApi, module_base: usize) -> Result<usize, String> {
    let root = read_usize(host, module_base + FIXED_ROOT_RVA, "fixed_root")?;
    let owner = read_usize(host, root + FIXED_OWNER_OFFSET, "fixed_root+0x18")?;
    if owner == 0 {
        Err("fixed owner is null".to_string())
    } else {
        Ok(owner)
    }
}

fn read_floats(host: &OwnedHostApi, base: usize, offsets: &[usize]) -> Vec<(usize, f32)> {
    offsets
        .iter()
        .filter_map(|offset| {
            read_f32(host, base + offset, "spawn_scaling_f32")
                .ok()
                .map(|value| (*offset, value))
        })
        .collect()
}

fn read_difficulty_tables(
    host: &OwnedHostApi,
    fixed20: usize,
    fixed28: usize,
    mission_id: u16,
    mode_type: u8,
    special_context: SpecialRowContext,
    candidate_count: usize,
) -> Result<Vec<DifficultyTables>, String> {
    (0..DIFFICULTY_COUNT)
        .map(|difficulty| {
            let difficulty = difficulty as u8;
            let base_row = read_base_reward_row_index(host, fixed28, mission_id, difficulty)?;
            let special_row =
                read_special_reward_row_index(host, fixed28, special_context.index, difficulty)?;
            let row_source = selected_row_source(mode_type, base_row, special_row);
            let normalized_row = row_source
                .select(base_row, special_row)
                .map_or(0, normalize_reward_row);

            Ok(DifficultyTables {
                difficulty,
                base_row,
                special_row,
                row_source,
                normalized_row,
                b3_chances: read_b3_chances(
                    host,
                    fixed20,
                    normalized_row,
                    usize::from(difficulty),
                )?,
                b4_chance: read_u8(
                    host,
                    fixed20
                        + B4_CHANCE_BASE
                        + usize::from(normalized_row) * 4
                        + usize::from(difficulty),
                    "spawn_b4_chance",
                )?,
                b518_weights: read_b518_weights(
                    host,
                    fixed20,
                    normalized_row,
                    usize::from(difficulty),
                    candidate_count,
                )?,
                candidate_a: read_candidate_table(
                    host,
                    fixed20,
                    CANDIDATE_TABLE_A,
                    usize::from(difficulty),
                    normalized_row,
                    candidate_count,
                )?,
                candidate_b: read_candidate_table(
                    host,
                    fixed20,
                    CANDIDATE_TABLE_B,
                    usize::from(difficulty),
                    normalized_row,
                    candidate_count,
                )?,
                category_rows: read_category_rows(
                    host,
                    fixed20,
                    usize::from(difficulty),
                    normalized_row,
                )?,
            })
        })
        .collect()
}

fn read_base_reward_row_index(
    host: &OwnedHostApi,
    fixed28: usize,
    mission_id: u16,
    difficulty: u8,
) -> Result<Option<u16>, String> {
    if mission_id > BASE_MISSION_LIMIT {
        return Ok(None);
    }
    let offset = BASE_REWARD_INDEX_OFFSET
        + (usize::from(mission_id) * BASE_REWARD_INDEX_STRIDE + usize::from(difficulty)) * 2;
    read_u16(host, fixed28 + offset, "spawn_reward_row_index").map(Some)
}

fn read_special_reward_row_index(
    host: &OwnedHostApi,
    fixed28: usize,
    context_index: Option<u32>,
    difficulty: u8,
) -> Result<Option<u16>, String> {
    let Some(context_index) = context_index else {
        return Ok(None);
    };
    let offset = SPECIAL_REWARD_INDEX_OFFSET
        + (context_index as usize * SPECIAL_REWARD_INDEX_STRIDE + usize::from(difficulty)) * 2;
    read_u16(host, fixed28 + offset, "spawn_special_reward_row_index").map(Some)
}

fn selected_row_source(
    mode_type: u8,
    base_row: Option<u16>,
    special_row: Option<u16>,
) -> RowSource {
    match mode_type {
        0 | 1 => {
            if base_row.is_some() {
                RowSource::Base
            } else {
                RowSource::None
            }
        }
        2..=5 => {
            if special_row.is_some() {
                RowSource::Special
            } else if base_row.is_some() {
                RowSource::Base
            } else {
                RowSource::None
            }
        }
        _ => RowSource::None,
    }
}

impl RowSource {
    fn select(self, base_row: Option<u16>, special_row: Option<u16>) -> Option<u16> {
        match self {
            Self::Base => base_row,
            Self::Special => special_row,
            Self::None => None,
        }
    }
}

fn normalize_reward_row(row: u16) -> u16 {
    if (0x14..=0x1d).contains(&row) {
        0x13
    } else if row > 0x1d {
        0
    } else {
        row
    }
}

fn read_b3_chances(
    host: &OwnedHostApi,
    fixed20: usize,
    row: u16,
    difficulty: usize,
) -> Result<[u8; 3], String> {
    let mut chances = [0u8; 3];
    for (index, offset) in B3_TABLES.iter().enumerate() {
        chances[index] = read_u8(
            host,
            fixed20 + offset + usize::from(row) * B3_ROW_STRIDE + difficulty,
            "spawn_b3_chance",
        )?;
    }
    Ok(chances)
}

fn read_b518_weights(
    host: &OwnedHostApi,
    fixed20: usize,
    row: u16,
    difficulty: usize,
    candidate_count: usize,
) -> Result<WeightSummary, String> {
    summarize_weights(candidate_count, |candidate| {
        read_u8(
            host,
            fixed20
                + B518_WEIGHTS_BASE
                + candidate * B518_CANDIDATE_STRIDE
                + difficulty * B518_DIFFICULTY_STRIDE
                + usize::from(row),
            "spawn_b518_weight",
        )
        .map(u16::from)
    })
}

fn read_candidate_table(
    host: &OwnedHostApi,
    fixed20: usize,
    table_base: usize,
    difficulty: usize,
    row: u16,
    candidate_count: usize,
) -> Result<[WeightSummary; 3], String> {
    [
        read_candidate_category(
            host,
            fixed20,
            table_base,
            difficulty,
            0,
            row,
            candidate_count,
        )?,
        read_candidate_category(
            host,
            fixed20,
            table_base,
            difficulty,
            1,
            row,
            candidate_count,
        )?,
        read_candidate_category(
            host,
            fixed20,
            table_base,
            difficulty,
            2,
            row,
            candidate_count,
        )?,
    ]
    .pipe(Ok)
}

fn read_candidate_category(
    host: &OwnedHostApi,
    fixed20: usize,
    table_base: usize,
    difficulty: usize,
    category: usize,
    row: u16,
    candidate_count: usize,
) -> Result<WeightSummary, String> {
    summarize_weights(candidate_count, |candidate| {
        read_u16(
            host,
            fixed20
                + table_base
                + candidate * CANDIDATE_STRIDE
                + difficulty * CANDIDATE_DIFF_STRIDE
                + category * CANDIDATE_CATEGORY_STRIDE
                + usize::from(row) * 2,
            "spawn_candidate_weight",
        )
    })
}

fn read_category_rows(
    host: &OwnedHostApi,
    fixed20: usize,
    difficulty: usize,
    row: u16,
) -> Result<[u16; 3], String> {
    let row_base = fixed20 + usize::from(row) * CATEGORY_ROW_STRIDE;
    Ok([
        read_u16(
            host,
            row_base + CATEGORY_ROW_BASES[0] + difficulty * 2,
            "spawn_category_row_0",
        )?,
        read_u16(
            host,
            row_base + CATEGORY_ROW_BASES[1] + difficulty * 2,
            "spawn_category_row_1",
        )?,
        read_u16(
            host,
            row_base + CATEGORY_ROW_BASES[2] + difficulty * 2,
            "spawn_category_row_2",
        )?,
    ])
}

fn read_phase_weights(
    host: &OwnedHostApi,
    fixed20: usize,
    candidate_count: usize,
) -> Result<Vec<PhaseWeights>, String> {
    PHASE_WEIGHT_BASES
        .iter()
        .enumerate()
        .map(|(phase, offsets)| {
            Ok(PhaseWeights {
                phase: phase as u8,
                category_weights: [
                    read_phase_category(host, fixed20, offsets[0], candidate_count)?,
                    read_phase_category(host, fixed20, offsets[1], candidate_count)?,
                    read_phase_category(host, fixed20, offsets[2], candidate_count)?,
                ],
            })
        })
        .collect()
}

fn read_phase_category(
    host: &OwnedHostApi,
    fixed20: usize,
    offset: usize,
    candidate_count: usize,
) -> Result<WeightSummary, String> {
    summarize_weights(candidate_count, |candidate| {
        read_u16(
            host,
            fixed20 + offset + candidate * PHASE_WEIGHT_STRIDE,
            "spawn_phase_weight",
        )
    })
}

fn summarize_weights<F>(candidate_count: usize, mut read: F) -> Result<WeightSummary, String>
where
    F: FnMut(usize) -> Result<u16, String>,
{
    let mut summary = WeightSummary::default();
    for candidate in 0..candidate_count {
        let value = read(candidate)?;
        summary.sum += u32::from(value);
        if value != 0 {
            summary.nonzero += 1;
            if summary.first.len() < 8 {
                summary.first.push((candidate, value));
            }
        }
    }
    Ok(summary)
}

fn format_f32_pairs(values: &[(usize, f32)]) -> String {
    values
        .iter()
        .map(|(offset, value)| format!("0x{offset:x}:{value:.3}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_category_summaries(values: &[WeightSummary; 3]) -> String {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| format!("cat{index}:{}", value.format_log()))
        .collect::<Vec<_>>()
        .join("|")
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_reward_rows_like_spawn_readers() {
        assert_eq!(normalize_reward_row(0), 0);
        assert_eq!(normalize_reward_row(0x13), 0x13);
        assert_eq!(normalize_reward_row(0x14), 0x13);
        assert_eq!(normalize_reward_row(0x1d), 0x13);
        assert_eq!(normalize_reward_row(0x1e), 0);
    }

    #[test]
    fn selects_special_row_when_context_is_known() {
        assert_eq!(
            selected_row_source(2, Some(8), Some(12)),
            RowSource::Special
        );
        assert_eq!(RowSource::Special.select(Some(8), Some(12)), Some(12));
    }

    #[test]
    fn falls_back_to_base_row_without_special_context() {
        assert_eq!(selected_row_source(1, Some(8), Some(12)), RowSource::Base);
        assert_eq!(RowSource::Base.select(Some(8), None), Some(8));
    }

    #[test]
    fn ignores_rows_during_inactive_mode() {
        assert_eq!(selected_row_source(6, Some(8), Some(12)), RowSource::None);
    }

    #[test]
    fn formats_weight_summary_compactly() {
        let summary = WeightSummary {
            sum: 12,
            nonzero: 2,
            first: vec![(1, 5), (3, 7)],
        };

        assert_eq!(summary.format_log(), "sum=12 nz=2 first=1:5,3:7");
    }
}
