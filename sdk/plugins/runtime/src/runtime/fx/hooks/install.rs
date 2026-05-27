use plugin_sdk::HostApi;

use crate::runtime::fx::{config::InstallMode, log, mods::FxInstallPlan};

use super::{
    arena::{patch_jump, CaveArena, CAVE_ARENA_SIZE},
    caves::{build_duration_cave, build_local_player_cave},
    data::build_shared_data,
    detour::install_aura_update_hook,
    diagnostics::{start_character_probe_logger, start_counter_logger, start_observed_id_logger},
    reload::start_config_reloader,
    InstallState, WorkerApi, AURA_DURATION_MASK, AURA_DURATION_PATTERN, LOCAL_PLAYER_MASK,
    LOCAL_PLAYER_PATTERN, WEAPON_AURA_ID_MASK, WEAPON_AURA_ID_PATTERN,
};

pub(super) fn install_now(
    api: HostApi<'_>,
    worker_api: WorkerApi,
    config: FxInstallPlan,
) -> Result<Option<InstallState>, String> {
    if !config.fx.enabled {
        log::write_line("fx_director disabled by fx definition");
        return Ok(None);
    }

    let needs_local_player = config.plugin.install_mode == InstallMode::LocalPlayerProbe
        || config.plugin.debug.observe_character_probe;
    let local_player_site = if needs_local_player {
        log::write_line("fx_director scanning LocalPlayerHook signature");
        let site = api
            .memory()
            .scan(LOCAL_PLAYER_PATTERN, LOCAL_PLAYER_MASK)
            .unwrap_or(0);
        if site == 0 {
            return Err("local player signature not found".to_string());
        }
        log::write_line(format!(
            "fx_director LocalPlayerHook signature site=0x{site:x}"
        ));
        Some(site)
    } else {
        log::write_line("fx_director LocalPlayerHook skipped: using host active_character API");
        None
    };

    if config.plugin.install_mode == InstallMode::LocalPlayerProbe {
        let Some(local_player_site) = local_player_site else {
            return Err("local player probe requested without local player hook".to_string());
        };
        log::write_line("fx_director local_player_probe mode: patching LocalPlayerHook only");
        let mut arena = CaveArena::new(local_player_site)?;
        log::write_line(format!(
            "fx_director cave arena base=0x{:x} size=0x{:x}",
            arena.base, CAVE_ARENA_SIZE
        ));
        let data = build_shared_data(&mut arena, config)?;
        log::write_line(format!(
            "fx_director shared data enabled=0x{:x} force_effect_id=0x{:x} effect_id=0x{:x} local_player_filter=0x{:x} local_player=0x{:x} local_player_fx_owner=0x{:x}",
            data.enabled, data.force_effect_id, data.effect_id, data.local_player_filter, data.local_player, data.local_player_fx_owner
        ));
        let cave = build_local_player_cave(&mut arena, local_player_site + 7, data)?;
        log::write_line(format!(
            "fx_director LocalPlayerHook cave=0x{:x}",
            cave.entry
        ));
        unsafe {
            log::write_line("fx_director patching LocalPlayerHook jump");
            patch_jump(api, local_player_site, cave.entry, 7)?;
        }
        log::write_line(format!(
            "fx_director local_player_probe installed local_player_site=0x{local_player_site:x}"
        ));
        start_character_probe_logger(data);
        return Ok(Some(InstallState {
            data,
            _aura_update_hook: None,
        }));
    }

    log::write_line("fx_director scanning TestWeaponAuraV2 signature");
    let id_site = api
        .memory()
        .scan(WEAPON_AURA_ID_PATTERN, WEAPON_AURA_ID_MASK)
        .unwrap_or(0);
    if id_site == 0 {
        return Err("weapon aura id signature not found".to_string());
    }
    log::write_line(format!(
        "fx_director TestWeaponAuraV2 signature site=0x{id_site:x}"
    ));

    let duration_site = if config.fx.force_effect_id {
        log::write_line("fx_director scanning TestWeaponAuraDuration signature");
        let site = api
            .memory()
            .scan(AURA_DURATION_PATTERN, AURA_DURATION_MASK)
            .unwrap_or(0);
        if site == 0 {
            return Err("aura duration signature not found".to_string());
        }
        log::write_line(format!(
            "fx_director TestWeaponAuraDuration signature site=0x{site:x}"
        ));
        Some(site)
    } else {
        log::write_line("fx_director observer mode: TestWeaponAuraDuration skipped");
        None
    };

    if config.plugin.install_mode == InstallMode::ScanOnly {
        log::write_line("fx_director scan_only mode: signatures found, patching skipped");
        return Ok(None);
    }

    log::write_line("fx_director allocating cave arena");
    let mut arena = CaveArena::new(id_site)?;
    log::write_line(format!(
        "fx_director cave arena base=0x{:x} size=0x{:x}",
        arena.base, CAVE_ARENA_SIZE
    ));
    log::write_line("fx_director allocating shared data");
    let data = build_shared_data(&mut arena, config)?;
    log::write_line(format!(
        "fx_director shared data enabled=0x{:x} force_effect_id=0x{:x} effect_id=0x{:x} local_player_filter=0x{:x} local_player=0x{:x} local_player_fx_owner=0x{:x}",
        data.enabled, data.force_effect_id, data.effect_id, data.local_player_filter, data.local_player, data.local_player_fx_owner
    ));
    let local_cave = if let Some(local_player_site) = local_player_site {
        log::write_line("fx_director building LocalPlayerHook cave");
        let cave = build_local_player_cave(&mut arena, local_player_site + 7, data)?;
        log::write_line(format!(
            "fx_director LocalPlayerHook cave=0x{:x}",
            cave.entry
        ));
        Some((local_player_site, cave))
    } else {
        None
    };
    let aura_update_hook = if config.fx.force_effect_id || config.plugin.debug.observe_effect_ids {
        log::write_line("fx_director installing function hook for aura update");
        let (function_entry, hook) = install_aura_update_hook(api, &mut arena, id_site, data)?;
        log::write_line(format!(
            "fx_director aura update hook entry=0x{function_entry:x} trampoline=0x{:x}",
            hook.trampoline
        ));
        Some(hook)
    } else {
        log::write_line("fx_director aura update hook skipped: passthrough mode");
        None
    };
    let duration_cave = if let Some(duration_site) = duration_site {
        log::write_line("fx_director building TestWeaponAuraDuration cave");
        let cave = build_duration_cave(
            &mut arena,
            duration_site + AURA_DURATION_PATTERN.len(),
            data,
        )?;
        log::write_line(format!(
            "fx_director TestWeaponAuraDuration cave=0x{:x}",
            cave.entry
        ));
        Some((duration_site, cave))
    } else {
        None
    };

    unsafe {
        if let Some((local_player_site, local_cave)) = local_cave {
            log::write_line("fx_director patching LocalPlayerHook jump");
            patch_jump(api, local_player_site, local_cave.entry, 7)?;
        }
        if let Some((duration_site, duration_cave)) = duration_cave {
            log::write_line("fx_director patching TestWeaponAuraDuration jump");
            patch_jump(
                api,
                duration_site,
                duration_cave.entry,
                AURA_DURATION_PATTERN.len(),
            )?;
        }
    }

    log::write_line(format!(
        "fx_director hooks installed target={:?} local_player_site={} aura_update_site=0x{id_site:x} duration_site={}",
        config.fx.target,
        local_player_site
            .map(|site| format!("0x{site:x}"))
            .unwrap_or_else(|| "skipped".to_string()),
        duration_site
            .map(|site| format!("0x{site:x}"))
            .unwrap_or_else(|| "skipped".to_string())
    ));
    start_config_reloader(worker_api.clone(), data, config);
    if (config.fx.force_effect_id || config.plugin.debug.observe_effect_ids)
        && worker_api.debug_enabled()
    {
        start_observed_id_logger(data);
        start_counter_logger(data);
    } else {
        log::write_line("fx_director diagnostics disabled");
    }
    if config.plugin.debug.observe_character_probe && worker_api.debug_enabled() {
        start_character_probe_logger(data);
    } else {
        log::write_line("fx_director character probe disabled");
    }
    Ok(Some(InstallState {
        data,
        _aura_update_hook: aura_update_hook,
    }))
}
