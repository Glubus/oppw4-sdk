use mlua::{Lua, Table, Value};

use crate::{
    runtime::register_std_module,
    std_plugins::{json, mission_data},
};

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let module = lua.create_table()?;
    module.set("for_mission", lua.create_function(for_mission)?)?;
    module.set("missions", lua.create_function(missions)?)?;
    register_std_module(lua, "ranks", module)
}

fn for_mission(lua: &Lua, query: Value) -> mlua::Result<Value> {
    let Some(mission) = mission_data::find_mission(query)? else {
        return Ok(Value::Nil);
    };
    let Some(rank_conditions) = &mission.rank_conditions else {
        return Ok(Value::Nil);
    };
    let table = lua.create_table()?;
    table.set(
        "mission",
        mission_data::mission_summary_table(lua, mission)?,
    )?;
    table.set(
        "observations",
        json::values_to_lua_array(lua, &rank_conditions.observations)?,
    )?;
    Ok(Value::Table(table))
}

fn missions(lua: &Lua, (): ()) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, mission) in
        mission_data::missions_with(|mission| mission.rank_conditions.is_some()).enumerate()
    {
        table.set(
            index + 1,
            mission_data::mission_summary_table(lua, mission)?,
        )?;
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use mlua::Lua;

    #[test]
    fn std_ranks_is_requireable() {
        let lua = Lua::new();
        crate::runtime::install_runtime(&lua).expect("runtime");
        let exists: bool = lua
            .load(
                r#"
                local ranks = require("std.ranks")
                return ranks.for_mission(65535) == nil
                "#,
            )
            .eval()
            .expect("ranks module");
        assert!(exists);
    }
}
