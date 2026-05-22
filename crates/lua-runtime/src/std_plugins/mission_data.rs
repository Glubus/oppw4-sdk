use mlua::{Lua, Table, Value};
use struct_api::missions::Mission;

pub(super) fn find_mission(query: Value) -> mlua::Result<Option<&'static Mission>> {
    match query {
        Value::String(id) => Ok(struct_api::missions::find(id.to_str()?.as_ref())),
        Value::Integer(id) if (0..=u16::MAX as i64).contains(&id) => {
            Ok(struct_api::missions::find_by_id(id as u16))
        }
        _ => Ok(None),
    }
}

pub(super) fn mission_summary_table(lua: &Lua, mission: &Mission) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set("kind", "mission")?;
    table.set("id", mission.id.as_str())?;
    if let Some(display_name) = &mission.display_name {
        table.set("display_name", display_name.as_str())?;
    }
    if let Some(mission_id) = mission.mission_id {
        table.set("mission_id", mission_id)?;
    }
    if let Some(linkdata_id) = mission.linkdata_id {
        table.set("linkdata_id", linkdata_id)?;
    }
    table.set("aliases", string_array(lua, &mission.aliases)?)?;
    table.set("modes", string_array(lua, &mission.modes)?)?;
    Ok(table)
}

pub(super) fn missions_with<'a>(
    has_data: impl Fn(&'a Mission) -> bool,
) -> impl Iterator<Item = &'a Mission> {
    struct_api::missions::all()
        .iter()
        .filter(move |mission| has_data(mission))
}

fn string_array(lua: &Lua, values: &[String]) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    for (index, value) in values.iter().enumerate() {
        table.set(index + 1, value.as_str())?;
    }
    Ok(table)
}
