use std::sync::Arc;

use mlua::{Function, Lua, Table};

use crate::config::FxConfig;

use super::{
    fx_options::{apply_fx_options, apply_fx_to_handle, apply_target_from_character},
    state::SharedFxState,
};

pub(super) fn fx_director_module(lua: &Lua, state: SharedFxState) -> mlua::Result<Table> {
    let table = super::fx_module::fx_module(lua, Arc::clone(&state))?;
    table.set(
        "__oppw4_on_import",
        lua.create_function(move |lua, ()| register_character_extensions(lua, Arc::clone(&state)))?,
    )?;
    Ok(table)
}

fn register_character_extensions(lua: &Lua, state: SharedFxState) -> mlua::Result<()> {
    let source = current_mod_id(lua)?;
    state
        .lock()
        .expect("fx state lock")
        .clear_mod_source(&source);
    let register: Function = lua.globals().get("__oppw4_register_character_method")?;
    let add_fx_state = Arc::clone(&state);
    register.call::<()>((
        "fx_director",
        "add_fx",
        lua.create_function(move |lua, (character, options): (Table, Option<Table>)| {
            let source = current_mod_id(lua)?;
            let mut fx = FxConfig::default();
            apply_target_from_character(&mut fx, &character)?;
            if let Some(options) = options {
                apply_fx_options(&mut fx, &options)?;
            }
            let index = add_fx_state
                .lock()
                .expect("fx state lock")
                .push_effect(&source, fx);
            let handle = fx_handle_table(lua, Arc::clone(&add_fx_state), fx, index)?;
            apply_fx_handle_metadata(lua, &handle, &character, fx)?;
            Ok(handle)
        })?,
    ))
}

pub(super) fn current_mod_id(lua: &Lua) -> mlua::Result<String> {
    lua.globals()
        .get::<Option<String>>("__oppw4_mod_id")
        .map(|id| id.unwrap_or_else(|| "unknown_mod".to_string()))
}

fn fx_handle_table(
    lua: &Lua,
    state: SharedFxState,
    fx: FxConfig,
    index: usize,
) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "fx")?;
    table.set("__fx_index", index)?;
    apply_fx_to_handle(lua, &table, fx)?;

    let timing_state = Arc::clone(&state);
    table.set(
        "timing",
        lua.create_function(
            move |_, (this, speed, loop_start, loop_end): (Table, f32, f32, f32)| {
                this.set("animation_speed", speed)?;
                this.set("loop_start", loop_start)?;
                this.set("loop_end", loop_end)?;
                if let Some(index) = this.get::<Option<usize>>("__fx_index")? {
                    timing_state
                        .lock()
                        .expect("fx state lock")
                        .update_effect(index, |fx| {
                            fx.animation_speed = speed;
                            fx.loop_start = loop_start;
                            fx.loop_end = loop_end;
                        });
                }
                Ok(this)
            },
        )?,
    )?;

    let enable_state = Arc::clone(&state);
    table.set(
        "enable",
        lua.create_function(move |_, (this, enabled): (Table, Option<bool>)| {
            let enabled = enabled.unwrap_or(true);
            this.set("enabled", enabled)?;
            if let Some(index) = this.get::<Option<usize>>("__fx_index")? {
                enable_state
                    .lock()
                    .expect("fx state lock")
                    .update_effect(index, |fx| {
                        fx.enabled = enabled;
                    });
            }
            Ok(this)
        })?,
    )?;

    Ok(table)
}

fn apply_fx_handle_metadata(
    lua: &Lua,
    handle: &Table,
    character: &Table,
    fx: FxConfig,
) -> mlua::Result<()> {
    if let Some(name) = character.get::<Option<String>>("name")? {
        handle.set("character", name)?;
    }
    if let Some(model_id) = character.get::<Option<u16>>("model_id")? {
        handle.set("character_id", model_id)?;
    }
    let ids = fx.required_character_ids[..fx.required_character_id_count as usize].to_vec();
    handle.set("required_character_ids", lua.create_sequence_from(ids)?)?;
    Ok(())
}
