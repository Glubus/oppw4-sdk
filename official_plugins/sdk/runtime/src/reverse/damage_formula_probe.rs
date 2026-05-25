use std::{
    mem,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;

use crate::{config::DamageFormulaProbeConfig, runtime::probe::PLUGIN_ID};

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
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn install(host: OwnedHostApi, config: DamageFormulaProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "actor_stat_init_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);

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
}

fn log_actor_stat_init(actor: usize, source: usize, mode: i32, stats: ActorStats) {
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ActorStats {
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
            "field34={} field38={} stat3c={} stat40={} byte48={} byte_d7={}",
            self.field_34, self.field_38, self.stat_3c, self.stat_40, self.field_48, self.field_d7
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SourceStats {
    byte_00: u8,
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
            "byte00={} word06={} word08={} word0a={} field34={} field38={} field44={} field238={}",
            self.byte_00,
            self.word_06,
            self.word_08,
            self.word_0a,
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

unsafe fn read_u32(ptr: *const u8, offset: usize) -> u32 {
    (ptr.add(offset) as *const u32).read_unaligned()
}
