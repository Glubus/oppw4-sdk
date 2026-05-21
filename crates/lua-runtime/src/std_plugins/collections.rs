use std::{cell::RefCell, collections::HashMap, rc::Rc};

use mlua::{Lua, RegistryKey, Table, Value};

use crate::runtime::register_std_module;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let collections = lua.create_table()?;
    collections.set("map", lua.create_function(map)?)?;
    collections.set("ring_buffer", lua.create_function(ring_buffer)?)?;
    register_std_module(lua, "collections", collections)
}

fn map(lua: &Lua, (): ()) -> mlua::Result<Table> {
    let state = Rc::new(RefCell::new(MapState::default()));
    let table = lua.create_table()?;

    {
        let state = Rc::clone(&state);
        table.set(
            "set",
            lua.create_function(move |lua, (_self, key, value): (Table, Value, Value)| {
                let key = MapKey::from_lua_value(key)?;
                let registry_key = lua.create_registry_value(value)?;
                let mut state = state.borrow_mut();
                if let Some(old) = state.values.insert(key.clone(), registry_key) {
                    lua.remove_registry_value(old)?;
                } else {
                    state.keys.push(key);
                }
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "get",
            lua.create_function(move |lua, (_self, key): (Table, Value)| {
                let key = MapKey::from_lua_value(key)?;
                match state.borrow().values.get(&key) {
                    Some(value) => lua.registry_value(value),
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "get_or",
            lua.create_function(move |lua, (_self, key, fallback): (Table, Value, Value)| {
                let key = MapKey::from_lua_value(key)?;
                match state.borrow().values.get(&key) {
                    Some(value) => lua.registry_value(value),
                    None => Ok(fallback),
                }
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "has",
            lua.create_function(move |_, (_self, key): (Table, Value)| {
                let key = MapKey::from_lua_value(key)?;
                Ok(state.borrow().values.contains_key(&key))
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "remove",
            lua.create_function(move |lua, (_self, key): (Table, Value)| {
                let key = MapKey::from_lua_value(key)?;
                let mut state = state.borrow_mut();
                state.keys.retain(|existing| existing != &key);
                match state.values.remove(&key) {
                    Some(value) => {
                        let lua_value = lua.registry_value(&value)?;
                        lua.remove_registry_value(value)?;
                        Ok(lua_value)
                    }
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "clear",
            lua.create_function(move |lua, _self: Table| {
                let mut state = state.borrow_mut();
                for (_, value) in state.values.drain() {
                    lua.remove_registry_value(value)?;
                }
                state.keys.clear();
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "len",
            lua.create_function(move |_, _self: Table| Ok(state.borrow().values.len()))?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "keys",
            lua.create_function(move |lua, _self: Table| {
                let output = lua.create_table()?;
                for (index, key) in state.borrow().keys.iter().enumerate() {
                    output.set(index + 1, key.to_lua_value(lua)?)?;
                }
                Ok(output)
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "values",
            lua.create_function(move |lua, _self: Table| {
                let output = lua.create_table()?;
                let state = state.borrow();
                for (index, key) in state.keys.iter().enumerate() {
                    if let Some(value) = state.values.get(key) {
                        output.set(index + 1, lua.registry_value::<Value>(value)?)?;
                    }
                }
                Ok(output)
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "entries",
            lua.create_function(move |lua, _self: Table| {
                let output = lua.create_table()?;
                let state = state.borrow();
                for (index, key) in state.keys.iter().enumerate() {
                    if let Some(value) = state.values.get(key) {
                        let entry = lua.create_table()?;
                        entry.set("key", key.to_lua_value(lua)?)?;
                        entry.set("value", lua.registry_value::<Value>(value)?)?;
                        output.set(index + 1, entry)?;
                    }
                }
                Ok(output)
            })?,
        )?;
    }

    Ok(table)
}

fn ring_buffer(lua: &Lua, capacity: usize) -> mlua::Result<Table> {
    if capacity == 0 {
        return Err(mlua::Error::external(
            "ring_buffer capacity must be positive",
        ));
    }
    let state = Rc::new(RefCell::new(RingBufferState {
        capacity,
        values: Vec::with_capacity(capacity),
    }));
    let table = lua.create_table()?;

    {
        let state = Rc::clone(&state);
        table.set(
            "push",
            lua.create_function(move |lua, (_self, value): (Table, Value)| {
                let registry_key = lua.create_registry_value(value)?;
                let mut state = state.borrow_mut();
                if state.values.len() == state.capacity {
                    let old = state.values.remove(0);
                    lua.remove_registry_value(old)?;
                }
                state.values.push(registry_key);
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "last",
            lua.create_function(
                move |lua, _self: Table| match state.borrow().values.last() {
                    Some(value) => lua.registry_value(value),
                    None => Ok(Value::Nil),
                },
            )?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "get",
            lua.create_function(move |lua, (_self, index): (Table, usize)| {
                if index == 0 {
                    return Err(mlua::Error::external("ring_buffer index starts at 1"));
                }
                match state.borrow().values.get(index - 1) {
                    Some(value) => lua.registry_value(value),
                    None => Ok(Value::Nil),
                }
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "len",
            lua.create_function(move |_, _self: Table| Ok(state.borrow().values.len()))?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "capacity",
            lua.create_function(move |_, _self: Table| Ok(state.borrow().capacity))?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "is_full",
            lua.create_function(move |_, _self: Table| {
                let state = state.borrow();
                Ok(state.values.len() == state.capacity)
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "clear",
            lua.create_function(move |lua, _self: Table| {
                for value in state.borrow_mut().values.drain(..) {
                    lua.remove_registry_value(value)?;
                }
                Ok(())
            })?,
        )?;
    }
    {
        let state = Rc::clone(&state);
        table.set(
            "values",
            lua.create_function(move |lua, _self: Table| {
                let output = lua.create_table()?;
                for (index, value) in state.borrow().values.iter().enumerate() {
                    output.set(index + 1, lua.registry_value::<Value>(value)?)?;
                }
                Ok(output)
            })?,
        )?;
    }

    Ok(table)
}

#[derive(Debug, Default)]
struct MapState {
    values: HashMap<MapKey, RegistryKey>,
    keys: Vec<MapKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MapKey {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl MapKey {
    fn from_lua_value(value: Value) -> mlua::Result<Self> {
        match value {
            Value::String(value) => Ok(Self::String(value.to_str()?.to_string())),
            Value::Integer(value) => Ok(Self::Integer(value)),
            Value::Boolean(value) => Ok(Self::Boolean(value)),
            other => Err(mlua::Error::external(format!(
                "map key must be string, integer, or boolean, got {}",
                other.type_name()
            ))),
        }
    }

    fn to_lua_value(&self, lua: &Lua) -> mlua::Result<Value> {
        match self {
            Self::String(value) => Ok(Value::String(lua.create_string(value)?)),
            Self::Integer(value) => Ok(Value::Integer(*value)),
            Self::Boolean(value) => Ok(Value::Boolean(*value)),
        }
    }
}

#[derive(Debug)]
struct RingBufferState {
    capacity: usize,
    values: Vec<RegistryKey>,
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
    fn std_collections_is_available_through_require_and_std() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local collections = require("std.collections")
                local map = collections.map()
                map:set("garp", 12)
                return tostring(std.collections ~= nil) .. ":" .. map:get("garp")
                "#,
            )
            .eval()
            .expect("std.collections");

        assert_eq!(value, "true:12");
    }

    #[test]
    fn map_tracks_entries_and_len() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local collections = require("std.collections")
                local map = collections.map()
                map:set("garp", 12)
                map:set(42, "answer")
                map:set(true, "yes")
                map:set("garp", 13)
                local entries = map:entries()
                local removed = map:remove(42)
                return map:len() .. ":" .. map:get("garp") .. ":" .. tostring(map:has(42)) ..
                    ":" .. removed .. ":" .. entries[1].key .. ":" .. entries[1].value ..
                    ":" .. map:get_or("missing", "fallback")
                "#,
            )
            .eval()
            .expect("map");

        assert_eq!(value, "2:13:false:answer:garp:13:fallback");
    }

    #[test]
    fn map_rejects_complex_keys() {
        let lua = runtime_lua();

        let error = lua
            .load(
                r#"
                local collections = require("std.collections")
                local map = collections.map()
                map:set({}, "nope")
                "#,
            )
            .exec()
            .expect_err("complex key should fail");

        assert!(error.to_string().contains("map key must be string"));
    }

    #[test]
    fn ring_buffer_keeps_last_values_in_order() {
        let lua = runtime_lua();

        let value: String = lua
            .load(
                r#"
                local collections = require("std.collections")
                local ring = collections.ring_buffer(3)
                ring:push("a")
                ring:push("b")
                ring:push("c")
                ring:push("d")
                local values = ring:values()
                return ring:len() .. ":" .. ring:capacity() .. ":" .. tostring(ring:is_full()) ..
                    ":" .. values[1] .. values[2] .. values[3] .. ":" .. ring:last() .. ":" .. ring:get(1)
                "#,
            )
            .eval()
            .expect("ring buffer");

        assert_eq!(value, "3:3:true:bcd:d:b");
    }

    #[test]
    fn ring_buffer_rejects_zero_capacity() {
        let lua = runtime_lua();

        let error = lua
            .load(
                r#"
                local collections = require("std.collections")
                collections.ring_buffer(0)
                "#,
            )
            .exec()
            .expect_err("zero capacity should fail");

        assert!(error
            .to_string()
            .contains("ring_buffer capacity must be positive"));
    }
}
