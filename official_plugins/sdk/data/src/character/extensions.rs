use mlua::{Function, Lua};

const METHODS_TABLE: &str = "__struct_api_methods";
const OWNERS_TABLE: &str = "__struct_api_method_owners";
const AUTHORIZED_OWNERS_TABLE: &str = "__struct_api_authorized_method_owners";

pub(super) fn install_registry(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    let methods = lua.create_table()?;
    let owners = lua.create_table()?;
    let authorized = lua.create_table()?;
    globals.set(METHODS_TABLE, methods.clone())?;
    globals.set(OWNERS_TABLE, owners.clone())?;
    globals.set(AUTHORIZED_OWNERS_TABLE, authorized.clone())?;

    globals.set(
        "__oppw4_register_character_method",
        lua.create_function(
            move |_, (owner, name, method): (String, String, Function)| {
                let owner_key = owner.to_ascii_lowercase();
                if !authorized
                    .get::<Option<bool>>(owner_key.as_str())?
                    .unwrap_or(false)
                {
                    return Err(mlua::Error::external(format!(
                        "character.{name} refused for {owner}: missing std.character.extend"
                    )));
                }
                if let Some(existing_owner) = owners.get::<Option<String>>(name.as_str())? {
                    if !existing_owner.eq_ignore_ascii_case(&owner_key) {
                        return Err(mlua::Error::external(format!(
                            "character.{name} already registered by {existing_owner}, refused by {owner}"
                        )));
                    }
                }
                owners.set(name.as_str(), owner_key)?;
                methods.set(name, method)
            },
        )?,
    )
}

#[cfg(test)]
pub(super) fn authorize_owner(lua: &Lua, owner: &str) -> mlua::Result<()> {
    let authorized: mlua::Table = lua.globals().get(AUTHORIZED_OWNERS_TABLE)?;
    authorized.set(owner.to_ascii_lowercase(), true)
}
