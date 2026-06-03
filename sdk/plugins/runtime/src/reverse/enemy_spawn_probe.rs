use std::{
    mem,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        OnceLock,
    },
};

use hooks::module_base;
use hooks::{HookBuilder, InlineHook, Signature};
use plugin_sdk::OwnedHostApi;

use crate::{
    config::EnemySpawnProbeConfig,
    runtime::{memory::CaveArena, probe::PLUGIN_ID},
};

const SPAWN_REQUEST_SIGNATURE: Signature = Signature::new(
    "spawn_request_1415d1320",
    &[
        0x48, 0x89, 0x5c, 0x24, 0x08, 0x48, 0x89, 0x6c, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18,
        0x48, 0x89, 0x7c, 0x24, 0x20, 0x41, 0x56, 0x48, 0x83, 0xec, 0x70, 0x8d, 0x42, 0xfa, 0x4d,
        0x8b, 0xf1,
    ],
    &[1; 32],
);

const OVERWRITE_LEN: usize = 15;
const SPAWN_REQUEST_RVA: usize = 0x15d1320;

type SpawnRequestFn = extern "system" fn(usize, i32, *const f32, u64, u32) -> u64;

#[derive(Clone, Copy, Debug)]
struct SpawnCallsiteProbe {
    name: &'static str,
    kind: u32,
    rva: usize,
    original: [u8; 5],
    installed: &'static AtomicBool,
}

static DIRECT_PATH_INSTALLED: AtomicBool = AtomicBool::new(false);
static EXTRA_PATH_INSTALLED: AtomicBool = AtomicBool::new(false);
static WEIGHTED_PATH_INSTALLED: AtomicBool = AtomicBool::new(false);

const DIRECT_PATH_KIND: u32 = 1;
const EXTRA_PATH_KIND: u32 = 2;
const WEIGHTED_PATH_KIND: u32 = 3;

const CALLSITE_PROBES: &[SpawnCallsiteProbe] = &[
    SpawnCallsiteProbe {
        name: "direct_141254a70",
        kind: DIRECT_PATH_KIND,
        rva: 0x12550e2,
        original: [0xe8, 0x39, 0xc2, 0x37, 0x00],
        installed: &DIRECT_PATH_INSTALLED,
    },
    SpawnCallsiteProbe {
        name: "extra_1412505b0",
        kind: EXTRA_PATH_KIND,
        rva: 0x12507c7,
        original: [0xe8, 0x54, 0x0b, 0x38, 0x00],
        installed: &EXTRA_PATH_INSTALLED,
    },
    SpawnCallsiteProbe {
        name: "weighted_141250830",
        kind: WEIGHTED_PATH_KIND,
        rva: 0x1250e66,
        original: [0xe8, 0xb5, 0x04, 0x38, 0x00],
        installed: &WEIGHTED_PATH_INSTALLED,
    },
];

static HOST: OnceLock<OwnedHostApi> = OnceLock::new();
static HOOK: OnceLock<InlineHook> = OnceLock::new();
static TRAMPOLINE: AtomicUsize = AtomicUsize::new(0);
static LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static CALLSITE_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);
static MAX_LOGS: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn install(host: OwnedHostApi, config: EnemySpawnProbeConfig) {
    if !config.enabled {
        let _ = host
            .log()
            .write(PLUGIN_ID, "enemy_spawn_probe disabled by config");
        return;
    }

    let _ = HOST.set(host.clone());
    MAX_LOGS.store(config.max_logs, Ordering::Relaxed);

    if HOOK.get().is_some() {
        let _ = host
            .log()
            .write(PLUGIN_ID, "enemy_spawn_probe already installed");
        return;
    }

    let result = unsafe {
        HookBuilder::new(SPAWN_REQUEST_SIGNATURE)
            .overwrite_len(OVERWRITE_LEN)
            .scan()
            .and_then(|builder| {
                let site = builder.site();
                let hook = builder.install_abs_jump(spawn_request_detour as *const () as usize)?;
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
                    "enemy_spawn_probe installed site=0x{site:x} trampoline=0x{hook_trampoline:x} max_logs={}",
                    config.max_logs,
                ),
            );
            install_callsite_probes(&host);
        }
        Err(error) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!("enemy_spawn_probe install failed: {error}"),
            );
        }
    }
}

fn install_callsite_probes(host: &OwnedHostApi) {
    let base = module_base();
    if base == 0 {
        let _ = host.log().write(
            PLUGIN_ID,
            "enemy_spawn_probe callsite install failed: module base is null",
        );
        return;
    }

    for probe in CALLSITE_PROBES {
        install_callsite_probe(host, base, *probe);
    }
}

fn install_callsite_probe(host: &OwnedHostApi, base: usize, probe: SpawnCallsiteProbe) {
    if probe.installed.swap(true, Ordering::SeqCst) {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "enemy_spawn_probe callsite {} already installed",
                probe.name
            ),
        );
        return;
    }

    let site = base + probe.rva;
    let mut current = [0u8; 5];
    let read = unsafe { hooks::read_memory(site, current.as_mut_ptr(), current.len()) };
    if read != 0 {
        probe.installed.store(false, Ordering::SeqCst);
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "enemy_spawn_probe callsite {} read failed site=0x{site:x} result={read}",
                probe.name
            ),
        );
        return;
    }
    if current != probe.original {
        probe.installed.store(false, Ordering::SeqCst);
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "enemy_spawn_probe callsite {} unexpected bytes site=0x{site:x} expected={} got={}",
                probe.name,
                format_hex(&probe.original),
                format_hex(&current),
            ),
        );
        return;
    }

    let result = unsafe {
        (|| -> Result<usize, String> {
            let Some(mut arena) = CaveArena::new(site, 0x400) else {
                return Err("cave allocation failed".to_string());
            };
            let cave = build_callsite_cave(base + SPAWN_REQUEST_RVA, site + 5, probe.kind);
            let cave_address = arena.alloc(&cave, 16)?;
            let mut patch = [0x90; 5];
            asm::write_rel32_jump(&mut patch, site, 5, cave_address)?;
            let write = hooks::write_memory(site, patch.as_ptr(), patch.len());
            if write == 0 {
                Ok(cave_address)
            } else {
                Err(format!("write failed result={write}"))
            }
        })()
    };

    match result {
        Ok(cave_address) => {
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "enemy_spawn_probe callsite {} installed site=0x{site:x} cave=0x{cave_address:x}",
                    probe.name
                ),
            );
        }
        Err(error) => {
            probe.installed.store(false, Ordering::SeqCst);
            let _ = host.log().write(
                PLUGIN_ID,
                format!(
                    "enemy_spawn_probe callsite {} install failed site=0x{site:x}: {error}",
                    probe.name
                ),
            );
        }
    }
}

fn build_callsite_cave(spawn_request: usize, return_address: usize, kind: u32) -> Vec<u8> {
    let mut code = Vec::new();
    push_volatile_registers(&mut code);
    code.extend_from_slice(&[0x48, 0x83, 0xec, 0x28]);
    code.extend_from_slice(&[0x4d, 0x8b, 0xc8]);
    code.extend_from_slice(&[0x44, 0x8b, 0xc2]);
    code.extend_from_slice(&[0x48, 0x8b, 0xd1]);
    code.push(0xb9);
    code.extend_from_slice(&kind.to_le_bytes());
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(spawn_callsite_log as *const () as usize as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0]);
    code.extend_from_slice(&[0x48, 0x83, 0xc4, 0x28]);
    pop_volatile_registers(&mut code);
    code.extend_from_slice(&[0x48, 0xb8]);
    code.extend_from_slice(&(spawn_request as u64).to_le_bytes());
    code.extend_from_slice(&[0xff, 0xd0]);
    asm::emit_absolute_jump(&mut code, return_address, asm::AbsoluteJumpMode::UseR11);
    code
}

fn push_volatile_registers(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x50, 0x51, 0x52]);
    code.extend_from_slice(&[0x41, 0x50]);
    code.extend_from_slice(&[0x41, 0x51]);
    code.extend_from_slice(&[0x41, 0x52]);
    code.extend_from_slice(&[0x41, 0x53]);
}

fn pop_volatile_registers(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x41, 0x5b]);
    code.extend_from_slice(&[0x41, 0x5a]);
    code.extend_from_slice(&[0x41, 0x59]);
    code.extend_from_slice(&[0x41, 0x58]);
    code.extend_from_slice(&[0x5a, 0x59, 0x58]);
}

extern "system" fn spawn_request_detour(
    owner: usize,
    request_type: i32,
    position: *const f32,
    arg4: u64,
    arg5: u32,
) -> u64 {
    let original = TRAMPOLINE.load(Ordering::SeqCst);
    if original == 0 {
        return 0;
    }

    let original: SpawnRequestFn = unsafe { mem::transmute(original) };
    let result = original(owner, request_type, position, arg4, arg5);
    log_spawn_request(owner, request_type, position, arg4, arg5, result);
    result
}

fn log_spawn_request(
    owner: usize,
    request_type: i32,
    position: *const f32,
    arg4: u64,
    arg5: u32,
    result: u64,
) {
    let call = LOG_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if call > MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let position = SpawnRequestPosition::read(position);
    if let Some(host) = HOST.get() {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "enemy_spawn_probe call={call} owner=0x{owner:x} type={request_type} position={} arg4=0x{arg4:x} arg5=0x{arg5:x} result={result}",
                position.format(),
            ),
        );
    }
}

extern "system" fn spawn_callsite_log(
    kind: u32,
    owner: usize,
    request_type: i32,
    position: *const f32,
) {
    let call = CALLSITE_LOG_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if call > MAX_LOGS.load(Ordering::Relaxed) {
        return;
    }

    let position = SpawnRequestPosition::read(position);
    if let Some(host) = HOST.get() {
        let _ = host.log().write(
            PLUGIN_ID,
            format!(
                "enemy_spawn_probe callsite={call} source={} owner=0x{owner:x} type={request_type} position={}",
                callsite_label(kind),
                position.format(),
            ),
        );
    }
}

fn callsite_label(kind: u32) -> &'static str {
    match kind {
        DIRECT_PATH_KIND => "direct_141254a70",
        EXTRA_PATH_KIND => "extra_1412505b0",
        WEIGHTED_PATH_KIND => "weighted_141250830",
        _ => "unknown",
    }
}

fn format_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SpawnRequestPosition {
    values: Option<[f32; 4]>,
}

impl SpawnRequestPosition {
    fn read(position: *const f32) -> Self {
        if position.is_null() {
            return Self { values: None };
        }

        let values = unsafe {
            [
                position.read(),
                position.add(1).read(),
                position.add(2).read(),
                position.add(3).read(),
            ]
        };
        Self {
            values: Some(values),
        }
    }

    fn format(self) -> String {
        match self.values {
            Some([x, y, z, w]) => format!("{x:.3},{y:.3},{z:.3},{w:.3}"),
            None => "null".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_callsite_cave, callsite_label, SpawnRequestPosition, CALLSITE_PROBES,
        DIRECT_PATH_KIND, SPAWN_REQUEST_RVA,
    };

    const GAME_IMAGE_BASE: usize = 0x140000000;

    #[test]
    fn spawn_position_formats_null_pointer() {
        assert_eq!(
            SpawnRequestPosition::read(std::ptr::null()).format(),
            "null"
        );
    }

    #[test]
    fn spawn_position_formats_vec4() {
        let position = [1.25_f32, 2.5, 3.75, 4.0];

        assert_eq!(
            SpawnRequestPosition::read(position.as_ptr()).format(),
            "1.250,2.500,3.750,4.000"
        );
    }

    #[test]
    fn callsite_labels_known_paths() {
        assert_eq!(callsite_label(DIRECT_PATH_KIND), "direct_141254a70");
        assert_eq!(callsite_label(999), "unknown");
    }

    #[test]
    fn callsite_original_bytes_target_spawn_request() {
        for probe in CALLSITE_PROBES {
            assert_eq!(probe.original[0], 0xe8);
            let rel = i32::from_le_bytes([
                probe.original[1],
                probe.original[2],
                probe.original[3],
                probe.original[4],
            ]);
            let target = GAME_IMAGE_BASE + probe.rva + 5 + rel as usize;
            assert_eq!(target, GAME_IMAGE_BASE + SPAWN_REQUEST_RVA);
        }
    }

    #[test]
    fn callsite_cave_calls_logger_then_spawn_request() {
        let cave = build_callsite_cave(0x1415d1320, 0x1412550e7, DIRECT_PATH_KIND);

        assert!(
            cave.windows(2)
                .filter(|bytes| *bytes == [0xff, 0xd0])
                .count()
                >= 2
        );
        assert!(cave.ends_with(&[0x41, 0xff, 0xe3]));
    }
}
