use std::ffi::c_void;

use mlua::Lua;
use sdk_bridge::{BridgeModContext, BridgeModSource, ModId};

use crate::{module::LuaModule, vm, vm::runtime};

pub fn load(
    context: &BridgeModContext,
    mod_entry: &lua_api::LuaMod,
    modules: &[LuaModule],
) -> Result<vm::LuaVm, String> {
    let lua = runtime::new_lua().map_err(|error| format!("lua create failed: {error}"))?;
    lua_api::install_runtime(&lua)
        .map_err(|error| format!("lua runtime install failed: {error}"))?;
    runtime::install_mod_globals(&lua, mod_entry)
        .map_err(|error| format!("lua globals failed: {error}"))?;
    let handlers = vm::handlers::install(&lua, context)
        .map_err(|error| format!("lua handler registry install failed: {error}"))?;
    install_bridge_modules(&lua, &context.mod_id, modules)
        .map_err(|error| format!("lua module install failed: {error}"))?;
    runtime::hide_unsafe_globals(&lua)
        .map_err(|error| format!("lua sandbox seal failed: {error}"))?;

    lua_api::set_current_mod_file_context(Some(lua_api::CurrentModFileContext {
        root: match &mod_entry.source {
            lua_api::ModSource::Directory(root) => root.clone(),
            lua_api::ModSource::Zip { path, .. } => path.clone(),
        },
        zip_root: match &mod_entry.source {
            lua_api::ModSource::Directory(_) => String::new(),
            lua_api::ModSource::Zip { root, .. } => root.clone(),
        },
        is_zip: matches!(mod_entry.source, lua_api::ModSource::Zip { .. }),
        files: Default::default(),
    }));
    let source = mod_entry
        .read_entry_script()
        .map_err(|error| format!("lua entry read failed: {error}"))?;
    let exec_result = lua
        .load(&source)
        .set_name(format!(
            "{}:{}",
            mod_entry.manifest.id, mod_entry.manifest.entry_lua
        ))
        .exec()
        .map_err(|error| format!("lua entry failed: {error}"));
    lua_api::set_current_mod_file_context(None);
    exec_result?;

    let (handlers, handler_descriptors) = handlers
        .into_inner()
        .map_err(|error| format!("lua handler registry extract failed: {error}"))?;
    Ok(vm::LuaVm::new(lua, handlers, handler_descriptors))
}

pub fn lua_mod_from_context(context: &BridgeModContext) -> lua_api::LuaMod {
    lua_api::LuaMod {
        manifest: lua_api::LuaModManifest {
            id: context.mod_id.as_str().to_string(),
            name: context.name.clone(),
            uses_plugins: context.uses_plugins.clone(),
            entry_lua: context.entry_file.clone(),
        },
        source: match &context.source {
            BridgeModSource::Directory(path) => lua_api::ModSource::Directory(path.clone()),
            BridgeModSource::Zip { path, root } => lua_api::ModSource::Zip {
                path: path.clone(),
                root: root.clone(),
            },
        },
    }
}

fn install_bridge_modules(lua: &Lua, mod_id: &ModId, modules: &[LuaModule]) -> mlua::Result<()> {
    let mod_id = mod_id.as_str().to_string();
    lua.globals().set(
        "__oppw4_trace",
        lua.create_function(move |_, message: String| {
            eprintln!("lua_bridge trace mod={mod_id} {message}");
            Ok(())
        })?,
    )?;
    for module in modules {
        register_plugin_module(lua, module)?;
    }
    Ok(())
}

fn register_plugin_module(lua: &Lua, module: &LuaModule) -> mlua::Result<()> {
    let result = unsafe {
        (module.register)(
            module.context as *mut c_void,
            (lua as *const Lua).cast_mut().cast(),
        )
    };
    if result != 0 {
        return Err(mlua::Error::external(format!(
            "lua module register failed plugin={} module={} result={result}",
            module.plugin_id, module.module_name
        )));
    }
    Ok(())
}
