# Troubleshooting

## `Incorrect type. Expected "array".`

Check the JSON schema expected by the field. Costume asset lists and body-part lists are arrays even when there is only one entry.

## `movesets: null`

Generated views should avoid meaningless nulls when the canonical data is already present elsewhere. Check the generator and source data before treating a null as useful information.

## Plugin call fails with `-22`

The plugin tried to use a host operation without declaring the required capability in `plugin.toml`.

## Plugin call fails with `-24`

The plugin tried to register a Lua module that is not listed in `[lua].modules`.

## Native Linux build fails on Windows APIs

Build/check Windows runtime crates with:

```bash
cargo test -p <package> --target x86_64-pc-windows-gnu --no-run
```

## A mod is not found

Mods belong under game-level `mods/`, not under `plugins/<plugin_id>/mods/`.

## A plugin config is not found

Plugin configs belong under `plugins/configs/<plugin_id>/`.

## Data changes do not appear

Regenerate and validate `oppw4-data` generated views. Do not hand-edit files under `oppw4-data/generated/`.
