use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

use mlua::Lua;

use crate::runtime::register_std_module;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let origin = Instant::now();
    let time = lua.create_table()?;
    time.set(
        "now_ms",
        lua.create_function(move |_, ()| Ok(duration_ms_i64(origin.elapsed())))?,
    )?;
    time.set(
        "now_seconds",
        lua.create_function(move |_, ()| Ok(origin.elapsed().as_secs_f64()))?,
    )?;
    time.set("elapsed_ms", lua.create_function(elapsed_ms)?)?;
    time.set("seconds", lua.create_function(seconds)?)?;
    time.set("millis", lua.create_function(millis)?)?;
    time.set("cooldown", lua.create_function(cooldown)?)?;
    register_std_module(lua, "time", time)
}

fn elapsed_ms(lua: &Lua, start_ms: i64) -> mlua::Result<i64> {
    let std: mlua::Table = lua.globals().get("std")?;
    let time: mlua::Table = std.get("time")?;
    let now_ms: mlua::Function = time.get("now_ms")?;
    let now_ms: i64 = now_ms.call(())?;
    Ok(now_ms.saturating_sub(start_ms))
}

fn seconds(_: &Lua, value: f64) -> mlua::Result<i64> {
    if !value.is_finite() || value < 0.0 {
        return Err(mlua::Error::external("seconds value must be non-negative"));
    }
    checked_duration_ms(Duration::from_secs_f64(value))
}

fn millis(_: &Lua, value: i64) -> mlua::Result<i64> {
    if value < 0 {
        return Err(mlua::Error::external("millis value must be non-negative"));
    }
    Ok(value)
}

fn cooldown(lua: &Lua, duration_ms: i64) -> mlua::Result<mlua::Table> {
    if duration_ms < 0 {
        return Err(mlua::Error::external(
            "cooldown duration must be non-negative",
        ));
    }
    let state = Rc::new(RefCell::new(Cooldown {
        duration_ms,
        last_trigger_ms: None,
    }));
    let table = lua.create_table()?;

    {
        let state = Rc::clone(&state);
        table.set(
            "ready",
            lua.create_function(move |lua, _: mlua::Table| {
                let state = state.borrow();
                Ok(match state.last_trigger_ms {
                    Some(last) => current_ms(lua)?.saturating_sub(last) >= state.duration_ms,
                    None => true,
                })
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "trigger",
            lua.create_function(move |lua, _: mlua::Table| {
                state.borrow_mut().last_trigger_ms = Some(current_ms(lua)?);
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "remaining_ms",
            lua.create_function(move |lua, _: mlua::Table| {
                let state = state.borrow();
                Ok(match state.last_trigger_ms {
                    Some(last) => {
                        let elapsed = current_ms(lua)?.saturating_sub(last);
                        state.duration_ms.saturating_sub(elapsed)
                    }
                    None => 0,
                })
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "reset",
            lua.create_function(move |_, _: mlua::Table| {
                state.borrow_mut().last_trigger_ms = None;
                Ok(())
            })?,
        )?;
    }

    Ok(table)
}

#[derive(Debug)]
struct Cooldown {
    duration_ms: i64,
    last_trigger_ms: Option<i64>,
}

fn current_ms(lua: &Lua) -> mlua::Result<i64> {
    let std: mlua::Table = lua.globals().get("std")?;
    let time: mlua::Table = std.get("time")?;
    let now_ms: mlua::Function = time.get("now_ms")?;
    now_ms.call(())
}

fn duration_ms_i64(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn checked_duration_ms(duration: Duration) -> mlua::Result<i64> {
    i64::try_from(duration.as_millis()).map_err(|_| mlua::Error::external("duration is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_lua() -> Lua {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        lua
    }

    #[test]
    fn std_time_is_available_through_require_and_std() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local time = require("std.time")
                return tostring(time.now_ms() >= 0) .. ":" .. tostring(std.time.now_seconds() >= 0)
                "#,
            )
            .eval()
            .expect("std.time");

        assert_eq!(value, "true:true");
    }

    #[test]
    fn duration_helpers_convert_to_milliseconds() {
        let lua = runtime_lua();

        let values: (i64, i64) = lua
            .load(
                r#"
                local time = require("std.time")
                return time.seconds(1.5), time.millis(250)
                "#,
            )
            .eval()
            .expect("durations");

        assert_eq!(values, (1500, 250));
    }

    #[test]
    fn cooldown_tracks_ready_remaining_and_reset() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local time = require("std.time")
                local cooldown = time.cooldown(1000)
                local initial = cooldown:ready()
                cooldown:trigger()
                local after_trigger = cooldown:ready()
                local remaining = cooldown:remaining_ms()
                cooldown:reset()
                return tostring(initial) .. ":" .. tostring(after_trigger) .. ":" ..
                    tostring(remaining <= 1000 and remaining > 0) .. ":" .. tostring(cooldown:ready())
                "#,
            )
            .eval()
            .expect("cooldown");

        assert_eq!(value, "true:false:true:true");
    }

    #[test]
    fn std_time_does_not_need_os_global() {
        let lua = crate::runtime::sandbox::new_lua().expect("lua");
        crate::runtime::install_runtime(&lua).expect("runtime");
        crate::runtime::sandbox::hide_unsafe_globals(&lua).expect("sandbox");

        let value: String = lua
            .load(
                r#"
                local time = require("std.time")
                return tostring(os) .. ":" .. tostring(time.now_ms() >= 0)
                "#,
            )
            .eval()
            .expect("sandbox");

        assert_eq!(value, "nil:true");
    }
}
