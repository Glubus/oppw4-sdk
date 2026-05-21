use std::path::Path;

use mlua::{Lua, Table};

use crate::{mod_files, runtime::register_std_module};

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let files = lua.create_table()?;
    files.set("read_text", lua.create_function(read_text)?)?;
    files.set("read_bytes", lua.create_function(read_bytes)?)?;
    register_std_module(lua, "files", files)
}

fn read_text(lua: &Lua, path: String) -> mlua::Result<String> {
    mod_files::read_mod_text(lua, Path::new(&path))
}

fn read_bytes(lua: &Lua, path: String) -> mlua::Result<Table> {
    let bytes = mod_files::read_mod_bytes(lua, Path::new(&path))?;
    let table = lua.create_table()?;
    for (index, byte) in bytes.into_iter().enumerate() {
        table.set(index + 1, byte)?;
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_files_reads_text_from_current_mod_folder() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        let root = std::env::temp_dir().join(format!("oppw4-std-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(root.join("data.txt"), "hello").expect("write");
        lua.globals()
            .set("__oppw4_mod_root", root.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", false)
            .expect("zip global");

        let text: String = lua
            .load(
                r#"
                local files = require("std.files")
                return files.read_text("data.txt")
                "#,
            )
            .eval()
            .expect("read text");

        assert_eq!(text, "hello");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn std_files_rejects_parent_paths() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        lua.globals()
            .set("__oppw4_mod_root", r"C:\missing")
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", false)
            .expect("zip global");

        let error = lua
            .load(
                r#"
                local files = require("std.files")
                return files.read_text("../nope.txt")
                "#,
            )
            .eval::<String>()
            .expect_err("path should be rejected");

        assert!(error.to_string().contains("mod file path must be relative"));
    }

    #[test]
    fn std_files_reads_bytes_as_lua_array() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        let root =
            std::env::temp_dir().join(format!("oppw4-std-files-bytes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        std::fs::write(root.join("data.bin"), [1u8, 2, 255]).expect("write");
        lua.globals()
            .set("__oppw4_mod_root", root.to_string_lossy().to_string())
            .expect("root global");
        lua.globals()
            .set("__oppw4_mod_is_zip", false)
            .expect("zip global");

        let value: String = lua
            .load(
                r#"
                local files = require("std.files")
                local bytes = files.read_bytes("data.bin")
                return bytes[1] .. ":" .. bytes[2] .. ":" .. bytes[3] .. ":" .. tostring(bytes[4])
                "#,
            )
            .eval()
            .expect("read bytes");

        assert_eq!(value, "1:2:255:nil");
        let _ = std::fs::remove_dir_all(&root);
    }
}
