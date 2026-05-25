use mlua::{Lua, Table};

use crate::runtime::fx::config::{FxConfig, TargetMode};

pub(super) fn apply_target_from_character(
    fx: &mut FxConfig,
    character: &Table,
) -> mlua::Result<()> {
    if character
        .get::<Option<String>>("kind")?
        .is_some_and(|kind| kind.eq_ignore_ascii_case("local_player"))
    {
        fx.target = TargetMode::LocalPlayer;
    } else {
        fx.set_required_character_ids(character_match_ids(character)?);
    }
    Ok(())
}

pub(super) fn apply_fx_options(fx: &mut FxConfig, options: &Table) -> mlua::Result<()> {
    if let Some(enabled) = options.get::<Option<bool>>("enabled")? {
        fx.enabled = enabled;
    }
    if let Some(effect_id) = options.get::<Option<u32>>("effect_id")? {
        fx.effect_id = effect_id;
    }
    if let Some(force_effect_id) = options.get::<Option<bool>>("force_effect_id")? {
        fx.force_effect_id = force_effect_id;
    }
    if let Some(target) = options.get::<Option<String>>("target")? {
        fx.target = parse_target(&target);
    }
    if let Some(speed) = options.get::<Option<f32>>("speed")? {
        fx.animation_speed = speed;
    }
    if let Some(loop_start) = options.get::<Option<f32>>("loop_start")? {
        fx.loop_start = loop_start;
    }
    if let Some(loop_end) = options.get::<Option<f32>>("loop_end")? {
        fx.loop_end = loop_end;
    }
    if let Some(required_character_id) = options.get::<Option<u16>>("required_character_id")? {
        fx.set_required_character_ids([required_character_id]);
    }
    Ok(())
}

pub(super) fn apply_fx_to_handle(lua: &Lua, handle: &Table, fx: FxConfig) -> mlua::Result<()> {
    handle.set("effect_id", fx.effect_id)?;
    handle.set("force_effect_id", fx.force_effect_id)?;
    handle.set("target", target_mode_name(fx.target))?;
    handle.set("animation_speed", fx.animation_speed)?;
    handle.set("loop_start", fx.loop_start)?;
    handle.set("loop_end", fx.loop_end)?;
    handle.set("enabled", fx.enabled)?;
    handle.set("required_character_ids", lua.create_table()?)?;
    Ok(())
}

pub(super) fn fx_from_handle(effect: &Table) -> mlua::Result<FxConfig> {
    let mut fx = FxConfig::default();
    fx.enabled = effect.get::<Option<bool>>("enabled")?.unwrap_or(fx.enabled);
    fx.effect_id = effect
        .get::<Option<u32>>("effect_id")?
        .unwrap_or(fx.effect_id);
    fx.force_effect_id = effect
        .get::<Option<bool>>("force_effect_id")?
        .unwrap_or(fx.force_effect_id);
    if let Some(target) = effect.get::<Option<String>>("target")? {
        fx.target = parse_target(&target);
    }
    fx.animation_speed = effect
        .get::<Option<f32>>("animation_speed")?
        .unwrap_or(fx.animation_speed);
    fx.loop_start = effect
        .get::<Option<f32>>("loop_start")?
        .unwrap_or(fx.loop_start);
    fx.loop_end = effect
        .get::<Option<f32>>("loop_end")?
        .unwrap_or(fx.loop_end);
    if let Some(ids) = effect.get::<Option<Table>>("required_character_ids")? {
        fx.set_required_character_ids(
            ids.sequence_values::<u16>()
                .collect::<mlua::Result<Vec<_>>>()?,
        );
    }
    Ok(fx)
}

pub(super) fn character_match_ids(character: &Table) -> mlua::Result<Vec<u16>> {
    let mut ids = Vec::new();
    for key in ["runtime_id", "boss_runtime_id", "playable_id", "model_id"] {
        if let Some(id) = character.get::<Option<u16>>(key)? {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}

fn parse_target(value: &str) -> TargetMode {
    if value.eq_ignore_ascii_case("local_player") {
        TargetMode::LocalPlayer
    } else {
        TargetMode::All
    }
}

fn target_mode_name(target: TargetMode) -> &'static str {
    match target {
        TargetMode::All => "all",
        TargetMode::LocalPlayer => "local_player",
    }
}
