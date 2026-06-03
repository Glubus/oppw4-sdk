use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;

use crate::{
    config::{DamageFormulaProbeConfig, EnemyStatsProbeConfig},
    runtime::probe::PLUGIN_ID,
};

const ACTOR_STAT_INIT_SIGNATURE: Signature = Signature::new(
    "actor_stat_init_141231100",
    &[
        0x40, 0x53, 0x57, 0x48, 0x83, 0xec, 0x38, 0x48, 0x89, 0x74, 0x24, 0x58, 0x45, 0x33, 0xc9,
        0x4c, 0x89, 0x64, 0x24, 0x60, 0x41, 0x8b, 0xf0, 0x4c, 0x89, 0x6c, 0x24, 0x30, 0x48, 0x8b,
        0xfa, 0x40,
    ],
    &[1; 32],
);

const OVERWRITE_LEN: usize = 15;

type ActorStatInitFn = extern "system" fn(usize, usize, i32);

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static DAMAGE_FORMULA_ENABLED: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);
static ENEMY_STATS_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static ENEMY_STATS_MAX_LOGS: AtomicUsize = AtomicUsize::new(0);
static ENEMY_STATS_ENABLED: AtomicUsize = AtomicUsize::new(0);
static ENEMY_STATS_WRITE_REQUESTED: AtomicUsize = AtomicUsize::new(0);
static ENEMY_STATS_HP_MULTIPLIER: AtomicUsize = AtomicUsize::new(1);
static ENEMY_STATS_ATTACK_MULTIPLIER: AtomicUsize = AtomicUsize::new(1);
static ENEMY_STATS_SUMMARY: Mutex<EnemyStatsSummary> = Mutex::new(EnemyStatsSummary::new());
const ENEMY_STATS_SUMMARY_GROUPS: usize = 128;
const COMMANDER_CANDIDATE_HP_STAT: u32 = 390;

pub(crate) fn install(
    host: OwnedHostApi,
    config: DamageFormulaProbeConfig,
    enemy_stats: EnemyStatsProbeConfig,
) {
    if !config.enabled && !enemy_stats.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "actor_stat_init_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    DAMAGE_FORMULA_ENABLED.store(usize::from(config.enabled), Ordering::Relaxed);
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);
    configure_enemy_stats_probe(&host, enemy_stats);

    if HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "actor_stat_init_probe already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(ACTOR_STAT_INIT_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook =
                    builder.install_abs_jump(actor_stat_init_detour as *const () as usize)?;
                Ok((site, hook))
            })
    };

    match result {
        Ok((site, hook)) => {
            TRAMPOLINE.store(hook.trampoline, Ordering::SeqCst);
            let hook_trampoline = hook.trampoline;
            let _ = HOOK.set(hook);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "actor_stat_init_probe installed site=0x{site:x} trampoline=0x{hook_trampoline:x}",
                ),
            );
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("actor_stat_init_probe install failed: {error}"),
            );
        }
    }
}

extern "system" fn actor_stat_init_detour(actor: usize, source: usize, mode: i32) {
    let original = TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return;
    }

    let original: ActorStatInitFn = unsafe { mem::transmute(original) };
    original(actor, source, mode);

    let stats = ActorStats::read(actor);
    log_actor_stat_init(actor, source, mode, stats);
    log_enemy_stats_probe(actor, source, mode, stats);
}

fn log_actor_stat_init(actor: usize, source: usize, mode: i32, stats: ActorStats) {
    if DAMAGE_FORMULA_ENABLED.load(Ordering::Relaxed) == 0 {
        return;
    }
    let call = LOG_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if call > MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let source_snapshot = SourceStats::read(source);
    if let Some(host) = HOST.get() {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "actor_stat_init_probe call={call} mode={mode} actor=0x{actor:x} source=0x{source:x} actor_stats={} source_stats={}",
                stats.format(),
                source_snapshot.format(),
            ),
        );
    }
}

fn configure_enemy_stats_probe(host: &OwnedHostApi, config: EnemyStatsProbeConfig) {
    ENEMY_STATS_ENABLED.store(usize::from(config.enabled), Ordering::Relaxed);
    ENEMY_STATS_MAX_LOGS.store(config.max_logs, Ordering::Relaxed);
    ENEMY_STATS_WRITE_REQUESTED.store(usize::from(config.write_stats), Ordering::Relaxed);
    ENEMY_STATS_HP_MULTIPLIER.store(config.hp_multiplier, Ordering::Relaxed);
    ENEMY_STATS_ATTACK_MULTIPLIER.store(config.attack_multiplier, Ordering::Relaxed);

    if config.enabled {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "enemy_stats_probe armed max_logs={} write_stats={} hp_multiplier={} attack_multiplier={} write_filter=commander_candidate_390_byte00_1_5",
                config.max_logs,
                config.write_stats,
                config.hp_multiplier,
                config.attack_multiplier,
            ),
        );
    }
}

fn log_enemy_stats_probe(actor: usize, source: usize, mode: i32, stats: ActorStats) {
    if ENEMY_STATS_ENABLED.load(Ordering::Relaxed) == 0 {
        return;
    }
    let call = ENEMY_STATS_LOG_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if call > ENEMY_STATS_MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let write_requested = ENEMY_STATS_WRITE_REQUESTED.load(Ordering::Relaxed) != 0;
    let source_snapshot = SourceStats::read(source);
    record_enemy_stats_summary(call, source_snapshot, stats);
    let write_status = if write_requested {
        apply_commander_candidate_stats(actor, source_snapshot, stats)
    } else {
        "read_only".to_string()
    };
    let stats_after = if write_requested {
        ActorStats::read(actor)
    } else {
        stats
    };
    if let Some(host) = HOST.get() {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "enemy_stats_probe call={call} mode={mode} actor=0x{actor:x} source=0x{source:x} actor_stats={} actor_stats_after={} source_stats={} hp_multiplier={} attack_multiplier={} write_status={}",
                stats.format(),
                stats_after.format(),
                source_snapshot.format(),
                ENEMY_STATS_HP_MULTIPLIER.load(Ordering::Relaxed),
                ENEMY_STATS_ATTACK_MULTIPLIER.load(Ordering::Relaxed),
                write_status,
            ),
        );
    }
}

fn apply_commander_candidate_stats(actor: usize, source: SourceStats, stats: ActorStats) -> String {
    if actor == 0 {
        return "refused:null_actor".to_string();
    }
    if !is_commander_candidate_stats(source, stats) {
        return "refused:filter".to_string();
    }

    let hp_multiplier = ENEMY_STATS_HP_MULTIPLIER.load(Ordering::Relaxed);
    let attack_multiplier = ENEMY_STATS_ATTACK_MULTIPLIER.load(Ordering::Relaxed);
    let hp_3c = scaled_stat(stats.stat_3c, hp_multiplier);
    let hp_40 = scaled_stat(stats.stat_40, hp_multiplier);
    let attack_34 = scalable_optional_stat(stats.field_34).map(|value| {
        let scaled = scaled_stat(value, attack_multiplier);
        (value, scaled)
    });
    let attack_38 = scalable_optional_stat(stats.field_38).map(|value| {
        let scaled = scaled_stat(value, attack_multiplier);
        (value, scaled)
    });

    unsafe {
        let ptr = actor as *mut u8;
        write_u32(ptr, 0x3c, hp_3c);
        write_u32(ptr, 0x40, hp_40);
        if let Some((_, value)) = attack_34 {
            write_u32(ptr, 0x34, value);
        }
        if let Some((_, value)) = attack_38 {
            write_u32(ptr, 0x38, value);
        }
    }

    format!(
        "applied:commander_candidate hp={}->{}:{}->{} attack34={} attack38={}",
        stats.stat_3c,
        hp_3c,
        stats.stat_40,
        hp_40,
        format_optional_scale(attack_34),
        format_optional_scale(attack_38)
    )
}

fn is_commander_candidate_stats(source: SourceStats, stats: ActorStats) -> bool {
    matches!(source.byte_00, 1 | 5)
        && stats.stat_3c == COMMANDER_CANDIDATE_HP_STAT
        && stats.stat_40 == COMMANDER_CANDIDATE_HP_STAT
}

fn scaled_stat(value: u32, multiplier: usize) -> u32 {
    if multiplier <= 1 {
        return value;
    }
    value.saturating_mul(multiplier.min(u32::MAX as usize) as u32)
}

fn scalable_optional_stat(value: u32) -> Option<u32> {
    if value == 0 || value == u32::MAX {
        None
    } else {
        Some(value)
    }
}

fn format_optional_scale(scale: Option<(u32, u32)>) -> String {
    match scale {
        Some((before, after)) => format!("{before}->{after}"),
        None => "skipped".to_string(),
    }
}

fn record_enemy_stats_summary(call: usize, source: SourceStats, stats: ActorStats) {
    let Ok(mut summary) = ENEMY_STATS_SUMMARY.lock() else {
        return;
    };
    summary.record(source, stats);
    if let Some(text) = summary.format_checkpoint(call) {
        if let Some(host) = HOST.get() {
            let _ = host.log().write(PLUGIN_ID, text);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct EnemyStatsSummaryEntry {
    byte_00: u8,
    word_08: u16,
    stat_3c: u32,
    stat_40: u32,
    count: usize,
}

impl EnemyStatsSummaryEntry {
    const fn empty() -> Self {
        Self {
            byte_00: 0,
            word_08: 0,
            stat_3c: 0,
            stat_40: 0,
            count: 0,
        }
    }

    fn matches(self, source: SourceStats, stats: ActorStats) -> bool {
        self.byte_00 == source.byte_00
            && self.word_08 == source.word_08
            && self.stat_3c == stats.stat_3c
            && self.stat_40 == stats.stat_40
    }
}

#[derive(Debug)]
struct EnemyStatsSummary {
    entries: [EnemyStatsSummaryEntry; ENEMY_STATS_SUMMARY_GROUPS],
    overflow: usize,
}

impl EnemyStatsSummary {
    const fn new() -> Self {
        Self {
            entries: [EnemyStatsSummaryEntry::empty(); ENEMY_STATS_SUMMARY_GROUPS],
            overflow: 0,
        }
    }

    fn record(&mut self, source: SourceStats, stats: ActorStats) {
        for entry in &mut self.entries {
            if entry.count != 0 && entry.matches(source, stats) {
                entry.count += 1;
                return;
            }
        }

        for entry in &mut self.entries {
            if entry.count == 0 {
                *entry = EnemyStatsSummaryEntry {
                    byte_00: source.byte_00,
                    word_08: source.word_08,
                    stat_3c: stats.stat_3c,
                    stat_40: stats.stat_40,
                    count: 1,
                };
                return;
            }
        }

        self.overflow += 1;
    }

    fn format_checkpoint(&self, call: usize) -> Option<String> {
        if !matches!(call, 64 | 128 | 256 | 512) {
            return None;
        }

        let mut entries = self.entries;
        entries.sort_by(|left, right| right.count.cmp(&left.count));
        let groups = entries
            .iter()
            .filter(|entry| entry.count != 0)
            .take(12)
            .map(|entry| {
                format!(
                    "byte00={} word08={} stat3c={} stat40={} count={}",
                    entry.byte_00, entry.word_08, entry.stat_3c, entry.stat_40, entry.count
                )
            })
            .collect::<Vec<_>>()
            .join("|");

        Some(format!(
            "enemy_stats_probe summary call={call} top=[{groups}] overflow={}",
            self.overflow
        ))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ActorStats {
    head_words: [u16; 16],
    field_34: u32,
    field_38: u32,
    stat_3c: u32,
    stat_40: u32,
    field_48: u8,
    field_d7: u8,
}

impl ActorStats {
    fn read(actor: usize) -> Self {
        if actor == 0 {
            return Self::default();
        }

        unsafe {
            let ptr = actor as *const u8;
            Self {
                head_words: read_u16_head(ptr),
                field_34: read_u32(ptr, 0x34),
                field_38: read_u32(ptr, 0x38),
                stat_3c: read_u32(ptr, 0x3c),
                stat_40: read_u32(ptr, 0x40),
                field_48: read_u8(ptr, 0x48),
                field_d7: read_u8(ptr, 0xd7),
            }
        }
    }

    fn format(self) -> String {
        format!(
            "head_u16={} field34={} field38={} stat3c={} stat40={} byte48={} byte_d7={}",
            format_u16_head(self.head_words),
            self.field_34,
            self.field_38,
            self.stat_3c,
            self.stat_40,
            self.field_48,
            self.field_d7
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SourceStats {
    byte_00: u8,
    head_words: [u16; 16],
    word_06: u16,
    word_08: u16,
    word_0a: u16,
    field_34: u32,
    field_38: u32,
    field_44: u32,
    field_238: u32,
}

impl SourceStats {
    fn read(source: usize) -> Self {
        if source == 0 {
            return Self::default();
        }

        unsafe {
            let ptr = source as *const u8;
            Self {
                byte_00: read_u8(ptr, 0x00),
                head_words: read_u16_head(ptr),
                word_06: read_u16(ptr, 0x06),
                word_08: read_u16(ptr, 0x08),
                word_0a: read_u16(ptr, 0x0a),
                field_34: read_u32(ptr, 0x34),
                field_38: read_u32(ptr, 0x38),
                field_44: read_u32(ptr, 0x44),
                field_238: read_u32(ptr, 0x238),
            }
        }
    }

    fn format(self) -> String {
        format!(
            "byte00={} word06={} word08={} word0a={} head_u16={} field34={} field38={} field44={} field238={}",
            self.byte_00,
            self.word_06,
            self.word_08,
            self.word_0a,
            format_u16_head(self.head_words),
            self.field_34,
            self.field_38,
            self.field_44,
            self.field_238
        )
    }
}

unsafe fn read_u8(ptr: *const u8, offset: usize) -> u8 {
    ptr.add(offset).read()
}

unsafe fn read_u16(ptr: *const u8, offset: usize) -> u16 {
    (ptr.add(offset) as *const u16).read_unaligned()
}

unsafe fn read_u16_head(ptr: *const u8) -> [u16; 16] {
    let mut words = [0; 16];
    for (index, word) in words.iter_mut().enumerate() {
        *word = read_u16(ptr, index * 2);
    }
    words
}

unsafe fn read_u32(ptr: *const u8, offset: usize) -> u32 {
    (ptr.add(offset) as *const u32).read_unaligned()
}

unsafe fn write_u32(ptr: *mut u8, offset: usize, value: u32) {
    (ptr.add(offset) as *mut u32).write_unaligned(value);
}

fn format_u16_head(words: [u16; 16]) -> String {
    words
        .iter()
        .enumerate()
        .map(|(index, word)| format!("{:02x}:{word}", index * 2))
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::{
        format_u16_head, is_commander_candidate_stats, scalable_optional_stat, scaled_stat,
        ActorStats, EnemyStatsSummary, SourceStats,
    };

    #[test]
    fn enemy_stats_summary_groups_by_source_and_stats() {
        let mut summary = EnemyStatsSummary::new();
        let source = SourceStats {
            byte_00: 5,
            word_08: 13,
            ..SourceStats::default()
        };
        let stats = ActorStats {
            stat_3c: 390,
            stat_40: 390,
            ..ActorStats::default()
        };

        summary.record(source, stats);
        summary.record(source, stats);

        let formatted = summary.format_checkpoint(64).unwrap();
        assert!(formatted.contains("byte00=5 word08=13 stat3c=390 stat40=390 count=2"));
    }

    #[test]
    fn enemy_stats_summary_only_logs_checkpoints() {
        let summary = EnemyStatsSummary::new();

        assert!(summary.format_checkpoint(63).is_none());
        assert!(summary.format_checkpoint(64).is_some());
        assert!(summary.format_checkpoint(512).is_some());
    }

    #[test]
    fn commander_candidate_filter_accepts_observed_officer_family_only() {
        let stats = ActorStats {
            stat_3c: 390,
            stat_40: 390,
            ..ActorStats::default()
        };

        assert!(is_commander_candidate_stats(
            SourceStats {
                byte_00: 1,
                ..SourceStats::default()
            },
            stats
        ));
        assert!(is_commander_candidate_stats(
            SourceStats {
                byte_00: 5,
                ..SourceStats::default()
            },
            stats
        ));
        assert!(!is_commander_candidate_stats(
            SourceStats {
                byte_00: 65,
                ..SourceStats::default()
            },
            stats
        ));
        assert!(!is_commander_candidate_stats(
            SourceStats {
                byte_00: 5,
                ..SourceStats::default()
            },
            ActorStats {
                stat_3c: 585,
                stat_40: 585,
                ..ActorStats::default()
            }
        ));
    }

    #[test]
    fn enemy_stat_scaling_saturates_and_skips_sentinels() {
        assert_eq!(scaled_stat(390, 2), 780);
        assert_eq!(scaled_stat(u32::MAX - 1, 2), u32::MAX);
        assert_eq!(scalable_optional_stat(0), None);
        assert_eq!(scalable_optional_stat(u32::MAX), None);
        assert_eq!(scalable_optional_stat(12), Some(12));
    }

    #[test]
    fn source_head_words_format_preserves_offsets() {
        let mut words = [0; 16];
        words[4] = 47;
        words[5] = 398;

        let formatted = format_u16_head(words);

        assert!(formatted.contains("08:47"));
        assert!(formatted.contains("0a:398"));
    }
}
