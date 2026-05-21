use mlua::Lua;

use crate::runtime::register_std_module;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let math = lua.create_table()?;
    math.set("clamp", lua.create_function(clamp)?)?;
    math.set("saturate", lua.create_function(saturate)?)?;
    math.set("lerp", lua.create_function(lerp)?)?;
    math.set("inverse_lerp", lua.create_function(inverse_lerp)?)?;
    math.set("remap", lua.create_function(remap)?)?;
    math.set("round_to", lua.create_function(round_to)?)?;
    math.set("align_down", lua.create_function(align_down)?)?;
    math.set("align_up", lua.create_function(align_up)?)?;
    register_std_module(lua, "math", math)
}

fn clamp(_: &Lua, (value, min, max): (f64, f64, f64)) -> mlua::Result<f64> {
    validate_range(min, max)?;
    Ok(value.clamp(min, max))
}

fn saturate(_: &Lua, value: f64) -> mlua::Result<f64> {
    Ok(value.clamp(0.0, 1.0))
}

fn lerp(_: &Lua, (start, end, t): (f64, f64, f64)) -> mlua::Result<f64> {
    Ok(start + (end - start) * t)
}

fn inverse_lerp(_: &Lua, (start, end, value): (f64, f64, f64)) -> mlua::Result<f64> {
    validate_non_zero_span(start, end)?;
    Ok((value - start) / (end - start))
}

fn remap(
    _: &Lua,
    (in_min, in_max, out_min, out_max, value): (f64, f64, f64, f64, f64),
) -> mlua::Result<f64> {
    validate_non_zero_span(in_min, in_max)?;
    let t = (value - in_min) / (in_max - in_min);
    Ok(out_min + (out_max - out_min) * t)
}

fn round_to(_: &Lua, (value, step): (f64, f64)) -> mlua::Result<f64> {
    if !step.is_finite() || step <= 0.0 {
        return Err(mlua::Error::external("round_to step must be positive"));
    }
    Ok((value / step).round() * step)
}

fn align_down(_: &Lua, (value, alignment): (i64, i64)) -> mlua::Result<i64> {
    let alignment = validate_alignment(alignment)?;
    Ok(value.div_euclid(alignment) * alignment)
}

fn align_up(_: &Lua, (value, alignment): (i64, i64)) -> mlua::Result<i64> {
    let alignment = validate_alignment(alignment)?;
    let aligned = value
        .checked_add(alignment - 1)
        .ok_or_else(|| mlua::Error::external("align_up overflow"))?
        .div_euclid(alignment)
        .checked_mul(alignment)
        .ok_or_else(|| mlua::Error::external("align_up overflow"))?;
    Ok(aligned)
}

fn validate_range(min: f64, max: f64) -> mlua::Result<()> {
    if !min.is_finite() || !max.is_finite() || min > max {
        return Err(mlua::Error::external("invalid range"));
    }
    Ok(())
}

fn validate_non_zero_span(start: f64, end: f64) -> mlua::Result<()> {
    if !start.is_finite() || !end.is_finite() || start == end {
        return Err(mlua::Error::external("range span must be non-zero"));
    }
    Ok(())
}

fn validate_alignment(alignment: i64) -> mlua::Result<i64> {
    if alignment <= 0 {
        return Err(mlua::Error::external("alignment must be positive"));
    }
    Ok(alignment)
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
    fn std_math_is_available_through_require_and_std() {
        let lua = runtime_lua();

        let values: (f64, f64) = lua
            .load(
                r#"
                local m = require("std.math")
                return m.clamp(12, 0, 10), std.math.saturate(-4)
                "#,
            )
            .eval()
            .expect("std.math");

        assert_eq!(values, (10.0, 0.0));
    }

    #[test]
    fn interpolation_helpers_return_expected_values() {
        let lua = runtime_lua();

        let values: (f64, f64, f64) = lua
            .load(
                r#"
                local m = require("std.math")
                return m.lerp(10, 20, 0.25), m.inverse_lerp(10, 20, 15), m.remap(0, 10, 100, 200, 2.5)
                "#,
            )
            .eval()
            .expect("interpolation");

        assert_eq!(values, (12.5, 0.5, 125.0));
    }

    #[test]
    fn alignment_helpers_return_expected_values() {
        let lua = runtime_lua();

        let values: (i64, i64) = lua
            .load(
                r#"
                local m = require("std.math")
                return m.align_down(4103, 16), m.align_up(4103, 16)
                "#,
            )
            .eval()
            .expect("alignment");

        assert_eq!(values, (4096, 4112));
    }

    #[test]
    fn invalid_ranges_are_rejected() {
        let lua = runtime_lua();

        let error = lua
            .load(
                r#"
                local m = require("std.math")
                return m.inverse_lerp(4, 4, 4)
                "#,
            )
            .eval::<f64>()
            .expect_err("zero span should fail");

        assert!(error.to_string().contains("range span must be non-zero"));
    }
}
