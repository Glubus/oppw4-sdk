use mlua::{Function, Lua, Table, Value};

pub fn install_require_hook(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    restrict_package_searchers(lua)?;
    let require: Function = globals.get("require")?;
    let imported = lua.create_table()?;
    globals.set(
        "require",
        lua.create_function(move |_, name: String| {
            let module: Value = require.call(name.as_str())?;
            let already_imported = imported
                .get::<Option<bool>>(name.as_str())?
                .unwrap_or(false);
            if !already_imported {
                imported.set(name.as_str(), true)?;
                if let Value::Table(table) = &module {
                    if let Some(on_import) = table.get::<Option<Function>>("__oppw4_on_import")? {
                        on_import.call::<()>(())?;
                    }
                }
            }
            Ok(module)
        })?,
    )
}

fn restrict_package_searchers(lua: &Lua) -> mlua::Result<()> {
    let package: Table = lua.globals().get("package")?;
    let searchers: Table = package.get("searchers")?;
    let preload_searcher: Value = searchers.get(1)?;
    let restricted = lua.create_table()?;
    restricted.set(1, preload_searcher)?;
    package.set("searchers", restricted)
}

pub fn register_module(lua: &Lua, name: &str, table: Table) -> mlua::Result<()> {
    let package: Table = lua.globals().get("package")?;
    let preload: Table = package.get("preload")?;
    let key = lua.create_registry_value(table)?;
    preload.set(
        name,
        lua.create_function(move |lua, ()| lua.registry_value::<Table>(&key))?,
    )
}

pub(crate) fn register_std_module(lua: &Lua, name: &str, table: Table) -> mlua::Result<()> {
    let globals = lua.globals();
    let std = match globals.get::<Option<Table>>("std")? {
        Some(std) => std,
        None => {
            let std = lua.create_table()?;
            globals.set("std", std.clone())?;
            std
        }
    };
    std.set(name, table.clone())?;
    register_module(lua, &format!("std.{name}"), table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_returns_registered_module() {
        let lua = Lua::new();
        install_require_hook(&lua).expect("require hook");
        let module = lua.create_table().expect("table");
        module.set("value", 42).expect("value");
        register_module(&lua, "fx_director", module).expect("module");

        let value: i64 = lua
            .load(r#"local fx_director = require("fx_director"); return fx_director.value"#)
            .eval()
            .expect("eval");

        assert_eq!(value, 42);
    }

    #[test]
    fn require_only_uses_registered_modules() {
        let lua = Lua::new();
        install_require_hook(&lua).expect("require hook");

        let error = lua
            .load(r#"require("not_registered_anywhere")"#)
            .exec()
            .expect_err("unregistered module should fail");
        let message = error.to_string();

        assert!(message.contains("not_registered_anywhere"));
    }
}
