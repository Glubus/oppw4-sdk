use mlua::Lua;

use crate::runtime::register_std_module;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let path = lua.create_table()?;
    path.set("join", lua.create_function(join)?)?;
    path.set("normalize_slashes", lua.create_function(normalize_slashes)?)?;
    path.set("basename", lua.create_function(basename)?)?;
    path.set("extension", lua.create_function(extension)?)?;
    path.set("stem", lua.create_function(stem)?)?;
    path.set("parent", lua.create_function(parent)?)?;
    path.set("is_safe_relative", lua.create_function(is_safe_relative)?)?;
    register_std_module(lua, "path", path)
}

fn join(_: &Lua, segments: mlua::MultiValue) -> mlua::Result<String> {
    let mut output = Vec::new();
    for segment in segments {
        let segment = match segment {
            mlua::Value::String(value) => normalize_slashes_inner(value.to_str()?.as_ref()),
            mlua::Value::Nil => String::new(),
            other => {
                return Err(mlua::Error::external(format!(
                    "path.join expects string segments, got {}",
                    other.type_name()
                )))
            }
        };
        output.extend(
            segment
                .split('/')
                .filter(|part| !part.is_empty())
                .map(str::to_string),
        );
    }
    Ok(output.join("/"))
}

fn normalize_slashes(_: &Lua, path: String) -> mlua::Result<String> {
    Ok(normalize_slashes_inner(&path))
}

fn basename(_: &Lua, path: String) -> mlua::Result<String> {
    Ok(normalized_parts(&path).last().cloned().unwrap_or_default())
}

fn extension(_: &Lua, path: String) -> mlua::Result<String> {
    let base = basename_inner(&path);
    Ok(base
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default())
}

fn stem(_: &Lua, path: String) -> mlua::Result<String> {
    let base = basename_inner(&path);
    Ok(base
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map(|(stem, _)| stem.to_string())
        .unwrap_or_else(|| base.to_string()))
}

fn parent(_: &Lua, path: String) -> mlua::Result<String> {
    let parts = normalized_parts(&path);
    if parts.len() <= 1 {
        return Ok(String::new());
    }
    Ok(parts[..parts.len() - 1].join("/"))
}

fn is_safe_relative(_: &Lua, path: String) -> mlua::Result<bool> {
    Ok(is_safe_relative_inner(&path))
}

fn normalize_slashes_inner(path: &str) -> String {
    let mut output = String::with_capacity(path.len());
    let mut last_was_separator = false;
    for ch in path.chars() {
        let is_separator = ch == '/' || ch == '\\';
        if is_separator {
            if !last_was_separator {
                output.push('/');
            }
            last_was_separator = true;
        } else {
            output.push(ch);
            last_was_separator = false;
        }
    }
    output
}

fn normalized_parts(path: &str) -> Vec<String> {
    let normalized = normalize_slashes_inner(path);
    normalized
        .split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn basename_inner(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .unwrap_or_default()
}

fn is_safe_relative_inner(path: &str) -> bool {
    if path.is_empty() || path.contains('\0') {
        return false;
    }
    let normalized = normalize_slashes_inner(path);
    if normalized.starts_with('/') || has_windows_drive_prefix(&normalized) {
        return false;
    }
    normalized
        .split('/')
        .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':'))
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
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
    fn std_path_is_available_through_require_and_std() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local path = require("std.path")
                return path.join("assets", "garp", "body.g1t") .. ":" .. std.path.extension("body.G1T")
                "#,
            )
            .eval()
            .expect("std.path");

        assert_eq!(value, "assets/garp/body.g1t:g1t");
    }

    #[test]
    fn path_parts_use_normalized_slashes() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local path = require("std.path")
                return path.normalize_slashes("assets\\garp//body.g1t") .. ":" ..
                    path.parent("assets\\garp\\body.g1t") .. ":" ..
                    path.basename("assets\\garp\\body.g1t") .. ":" ..
                    path.stem("assets\\garp\\body.g1t")
                "#,
            )
            .eval()
            .expect("path parts");

        assert_eq!(value, "assets/garp/body.g1t:assets/garp:body.g1t:body");
    }

    #[test]
    fn safe_relative_rejects_unsafe_paths() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local path = require("std.path")
                return tostring(path.is_safe_relative("assets/garp/body.g1t")) .. ":" ..
                    tostring(path.is_safe_relative("../body.g1t")) .. ":" ..
                    tostring(path.is_safe_relative("C:/mods/body.g1t")) .. ":" ..
                    tostring(path.is_safe_relative("/mods/body.g1t"))
                "#,
            )
            .eval()
            .expect("safe relative");

        assert_eq!(value, "true:false:false:false");
    }
}
