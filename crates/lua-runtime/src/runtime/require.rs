use mlua::{Function, Lua, Table, Value};

pub fn install_require_hook(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    restrict_package_searchers(lua)?;
    let package: Table = globals.get("package")?;
    let preload: Table = package.get("preload")?;
    let loaded: Table = package.get("loaded")?;
    let imported = lua.create_table()?;
    globals.set(
        "require",
        lua.create_function(move |lua, name: String| {
            trace(lua, format!("require start name={name}"));
            let cached: Value = loaded.get(name.as_str())?;
            if !matches!(cached, Value::Nil) {
                trace(lua, format!("require cached name={name}"));
                return Ok(cached);
            }
            let loader: Value = preload.get(name.as_str()).map_err(|_| {
                mlua::Error::RuntimeError(format!("module '{name}' is not registered"))
            })?;
            if matches!(loader, Value::Nil) {
                return Err(mlua::Error::RuntimeError(format!(
                    "module '{name}' is not registered"
                )));
            }
            trace(lua, format!("require loader start name={name}"));
            let module: Value = match loader {
                Value::Function(loader) => loader.call(())?,
                value => value,
            };
            trace(lua, format!("require resolved name={name}"));
            let module = if matches!(module, Value::Nil) {
                Value::Boolean(true)
            } else {
                module
            };
            loaded.set(name.as_str(), module.clone())?;
            let already_imported = imported
                .get::<Option<bool>>(name.as_str())?
                .unwrap_or(false);
            if !already_imported {
                imported.set(name.as_str(), true)?;
                if let Value::Table(table) = &module {
                    if let Some(on_import) = table.get::<Option<Function>>("__oppw4_on_import")? {
                        trace(lua, format!("require on_import start name={name}"));
                        on_import.call::<()>(())?;
                        trace(lua, format!("require on_import ok name={name}"));
                    }
                }
            }
            trace(lua, format!("require ok name={name}"));
            Ok(module)
        })?,
    )
}

fn trace(lua: &Lua, message: String) {
    let Ok(Some(trace)) = lua.globals().get::<Option<Function>>("__oppw4_trace") else {
        return;
    };
    let _ = trace.call::<()>(message);
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
    preload.set(name, table)
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
        register_module(&lua, "sdk.runtime.fx", module).expect("module");

        let value: i64 = lua
            .load(r#"local fx = require("sdk.runtime.fx"); return fx.value"#)
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
