use mlua::{Lua, Table, Value};

pub(super) fn from_lua(value: Value) -> mlua::Result<Vec<u8>> {
    match value {
        Value::String(value) => Ok(value.as_bytes().to_vec()),
        Value::Table(table) => table
            .sequence_values::<i64>()
            .map(|value| validate_u8(value?))
            .collect(),
        other => Err(mlua::Error::external(format!(
            "expected string or byte table, got {}",
            other.type_name()
        ))),
    }
}

pub(super) fn to_lua_table(lua: &Lua, bytes: &[u8]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, byte) in bytes.iter().enumerate() {
        table.set(index + 1, *byte)?;
    }
    Ok(table)
}

pub(super) fn validate_u8(value: i64) -> mlua::Result<u8> {
    u8::try_from(value).map_err(|_| mlua::Error::external("u8 value out of range"))
}

pub(super) fn validate_u16(value: i64) -> mlua::Result<u16> {
    u16::try_from(value).map_err(|_| mlua::Error::external("u16 value out of range"))
}

pub(super) fn validate_u32(value: i64) -> mlua::Result<u32> {
    u32::try_from(value).map_err(|_| mlua::Error::external("u32 value out of range"))
}

pub(super) fn validate_i32(value: i64) -> mlua::Result<i32> {
    i32::try_from(value).map_err(|_| mlua::Error::external("i32 value out of range"))
}
