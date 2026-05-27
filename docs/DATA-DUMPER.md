# SDK Data Dumper Concept

## Summary

`data_dumper` should be an official SDK developer plugin that converts runtime
reverse-engineering discoveries into editable `oppw4-data` source files.

The goal is to stop copying offsets and values by hand. When the SDK knows how
to read a mission, character, reward table, rank condition, or runtime state,
the dumper should capture the raw fields, attach evidence, and write a proposed
data folder that follows the same source layout as `oppw4-data/characters`.

This is not a gameplay mod. It is a bridge between runtime probes and the data
repository.

## Initial Target: Missions

Mission data should be split like character data:

```text
oppw4-data/
  missions/
    mission_0035/
      data.json
      difficulties.json
      rank_conditions.json
      rewards.json
      evidence.md
```

`data.json` owns stable identity and references to focused data files.
Specialized files own one responsibility each:

- `difficulties.json`: known effective difficulty rows and labels;
- `rank_conditions.json`: rank rows, condition rows, thresholds, and raw fields;
- `rewards.json`: Berry, medal/item, crew point, and soul reward data;
- `evidence.md`: offsets, logs, source probes, uncertainty, and manual notes.

Generated indexes should be reproducible from these source folders, the same way
character indexes are generated today.

## Plugin Shape

The runtime plugin should live under:

```text
sdk/plugins/
  data_dumper/
```

Suggested runtime layout:

```text
plugins/
  data_dumper/
    data_dumper.dll
    plugin.toml
    config.toml
```

Example configuration:

```toml
[dump]
enabled = true
target = "mission"
mission_id = 35
sections = ["difficulty", "rank_conditions", "rewards", "runtime_state"]
output = "oppw4-data"
```

The dumper should be able to run in capture mode while the user enters a
mission. It should record the active mission id, mode, difficulty, result rows,
reward entries, rank rows, and any known source offsets.

## Output Policy

The dumper should write data cautiously:

- write raw values even when labels are unknown;
- keep source offsets and probe names in `evidence.md`;
- mark uncertain interpretations explicitly;
- avoid silently overwriting hand-edited data;
- prefer creating proposed files or clearly logged merges;
- never write game assets, DLLs, logs, or binary dumps into `oppw4-data`.

## Future Targets

After missions, the same concept can target:

- characters: runtime ids, models, costumes, body parts, movesets, assets;
- effects: effect ids, runtime activation data, files, textures, G1E links;
- LinkData tables: row schemas, table names, references, fixed rows;
- RDB archives: known hashes, names, file types, archive ownership.

## Relationship With Standard APIs

`data_dumper` should consume SDK services instead of owning reverse logic alone:

- `std.difficulty` exposes difficulty state and editable difficulty surfaces;
- `std.ranks` exposes rank rows and threshold data;
- `std.rewards` exposes reward captures and reward table helpers;
- `sdk.runtime`, `sdk.linkdata`, and `sdk.rdb` provide the low-level reads.

The standard APIs should expose what the game can do. Gameplay mods can then
compose those APIs, while `data_dumper` uses them to populate `oppw4-data`.
