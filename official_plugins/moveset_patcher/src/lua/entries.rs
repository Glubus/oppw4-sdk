use mlua::Table;

pub(super) fn resolve_entry(character: &Table, moveset: &Table) -> mlua::Result<u16> {
    if let Some(entry) = moveset.get::<Option<u16>>("target_entry")? {
        return Ok(entry);
    }
    if let Some(entry) = character.get::<Option<u16>>("moveset_linkdata_entry")? {
        return Ok(entry);
    }
    let name = character_name(character)?.unwrap_or_default();
    if let Some(entry) = fallback_entry(&name) {
        return Ok(entry);
    }
    if let Some(entry) = moveset.get::<Option<u16>>("entry")? {
        return Ok(entry);
    }
    if let Some(entry) = moveset.get::<Option<u16>>("source_entry")? {
        return Ok(entry);
    }
    Err(mlua::Error::external(format!(
        "no moveset LINKDATA_A entry known for character={name}; pass target_entry = ... in moveset()"
    )))
}

pub(super) fn character_name(character: &Table) -> mlua::Result<Option<String>> {
    Ok(character
        .get::<Option<String>>("canonical")?
        .or_else(|| character.get::<Option<String>>("name").ok().flatten()))
}

fn fallback_entry(name: &str) -> Option<u16> {
    match normalize(name).as_str() {
        "garp" | "garp_yng" | "young_garp" => Some(247),
        "rayleigh" | "rayleigh_yng" | "young_rayleigh" => Some(248),
        _ => None,
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn known_fallback_entries_are_resolved_by_alias() {
        let lua = Lua::new();
        let character = lua.create_table().expect("character");
        character.set("canonical", "young_garp").expect("name");
        let moveset = lua.create_table().expect("moveset");

        let entry = resolve_entry(&character, &moveset).expect("entry");

        assert_eq!(entry, 247);
    }
}
