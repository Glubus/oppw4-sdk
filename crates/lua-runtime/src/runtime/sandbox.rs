use mlua::{Lua, LuaOptions, StdLib, Value};

const HIDDEN_GLOBALS: &[&str] = &["debug", "io", "os", "package"];

pub(crate) fn new_lua() -> mlua::Result<Lua> {
    Lua::new_with(sandbox_libs(), LuaOptions::default())
}

pub(crate) fn hide_unsafe_globals(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    for name in HIDDEN_GLOBALS {
        globals.set(*name, Value::Nil)?;
    }
    Ok(())
}

fn sandbox_libs() -> StdLib {
    StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::MATH | StdLib::PACKAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_globals_are_hidden_after_seal() {
        let lua = new_lua().expect("lua");
        hide_unsafe_globals(&lua).expect("hide globals");

        let visible: String = lua
            .load(
                r#"
                return tostring(os) .. ":" .. tostring(io) .. ":" .. tostring(debug) .. ":" .. tostring(package)
                "#,
            )
            .eval()
            .expect("eval");

        assert_eq!(visible, "nil:nil:nil:nil");
    }

    #[test]
    fn safe_libraries_remain_available() {
        let lua = new_lua().expect("lua");
        hide_unsafe_globals(&lua).expect("hide globals");

        let value: String = lua
            .load(
                r#"
                local values = { "zoro", "law" }
                return string.upper(values[1]) .. ":" .. math.floor(1.8)
                "#,
            )
            .eval()
            .expect("eval");

        assert_eq!(value, "ZORO:1");
    }
}
