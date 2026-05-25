use std::sync::Arc;

use mlua::{Lua, Table};

use crate::runtime::fx::config::CycleMode;

use super::{character_ext::current_mod_id, fx_options::fx_from_handle, state::SharedFxState};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct CycleOptions {
    pub(super) mode: Option<CycleMode>,
    pub(super) interval_ms: Option<u64>,
}

pub(super) fn fx_module(lua: &Lua, state: SharedFxState) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    let cycle_state = Arc::clone(&state);
    table.set(
        "cycle",
        lua.create_function(move |lua, (effects, options): (Table, Option<Table>)| {
            apply_preset_cycle(lua, &cycle_state, &effects, options.as_ref())
        })?,
    )?;
    Ok(table)
}

fn apply_preset_cycle(
    lua: &Lua,
    state: &SharedFxState,
    effects: &Table,
    options: Option<&Table>,
) -> mlua::Result<()> {
    let presets = effects
        .sequence_values::<Table>()
        .map(|effect| effect.and_then(|effect| fx_from_handle(&effect)))
        .collect::<mlua::Result<Vec<_>>>()?;
    let options = parse_cycle_options(options)?;
    let source = current_mod_id(lua)?;
    state
        .lock()
        .expect("fx state lock")
        .set_cycle_presets(presets, &source, options);
    Ok(())
}

fn parse_cycle_options(options: Option<&Table>) -> mlua::Result<CycleOptions> {
    let Some(options) = options else {
        return Ok(CycleOptions::default());
    };
    let mut parsed = CycleOptions {
        interval_ms: options.get::<Option<u64>>("interval_ms")?,
        ..CycleOptions::default()
    };
    if let Some(mode) = options.get::<Option<String>>("mode")? {
        parsed.mode = parse_cycle_mode(&mode);
    }
    if let Some(interval) = options.get::<Option<String>>("interval")? {
        parsed.mode = parse_cycle_mode(&interval);
    }
    Ok(parsed)
}

fn parse_cycle_mode(value: &str) -> Option<CycleMode> {
    if value.eq_ignore_ascii_case("after_animation") || value.eq_ignore_ascii_case("animation") {
        Some(CycleMode::AfterAnimation)
    } else if value.eq_ignore_ascii_case("fixed") || value.eq_ignore_ascii_case("fixed_interval") {
        Some(CycleMode::FixedInterval)
    } else {
        None
    }
}
