# SDK Architecture Rules

## Project Boundaries

- Loader code belongs in the loader repository.
- SDK orchestration belongs in the SDK repository.
- Official plugins belong in `official_plugins/` inside the SDK repository.
- Runtime mods, logs, configs, and release binaries belong in the game install, not in source control.
- Reverse-engineering notes must separate confirmed facts from guesses.

## Loader Rules

- Keep `dinput8.dll` minimal.
- Do not add skin, FX, moveset, RDB, LinkData, or character-bank logic to the loader.
- Do not add Lua runtime logic to the loader.
- The loader may expose only native primitives needed by SDK core.
- Loader logs should stay limited to boot, SDK discovery, SDK compatibility, and fatal errors.

## SDK Core Rules

- SDK core orchestrates; it does not implement feature plugins.
- SDK core owns Lua runtime, plugin lifecycle, dependency ordering, capabilities, logs/config routing, and service registries.
- SDK core must be able to refuse invalid registrations with clear errors.
- SDK core must keep APIs typed and explicit.

## Plugin Rules

- Plugins own feature domains.
- Plugins must declare static intent in `plugin.toml`.
- Plugin dependencies must be declared in `[dependencies].plugins`, not hidden in code.
- Lua modules and critical capabilities must be declared in `plugin.toml` before runtime registration.
- A Lua module registration is valid only when the plugin has both `lua.module` and the exact module name in `[lua].modules`.
- Lua module names in manifests must be lowercase-normalizable ASCII names using only letters, digits, `_`, `-`, and `.`.
- Plugins must register runtime capabilities through the SDK.
- Plugins must not mutate Lua globals directly.
- Plugins must not define private incompatible character identity systems.
- Plugins must request critical capabilities before using them.

## Lua Rules

- Lua mods run in a sandbox.
- Lua access to files, logs, plugins, and game services goes through SDK modules.
- `require` is SDK-controlled.
- Standard SDK modules use `std.*`.
- Feature modules use plugin ids.
- Plugins may extend standard handles only through SDK registration.
- Plugins must declare `std.character.extend` before adding methods to `std.character` handles.
- Plugins must not own current-mod path or zip resolution. If a plugin needs bytes from the active mod, it must ask SDK/Lua runtime helpers.

## Character Bank Rules

- The character bank is mandatory SDK data, but the editable source lives in the `oppw4-data` submodule.
- It must remain editable and reviewable.
- Use one editable JSON source tree per character under `oppw4-data/characters/`.
- Generated unified views and indexes belong under `oppw4-data/generated/`.
- Do not hand-edit generated character bank files.
- Every non-obvious id or relationship should have evidence or notes.
- Plugins may read character bank data but must not own it.
- Plugin-specific additions should use explicit extension data, not random top-level fields.

## File And Module Size Rules

- One responsibility per file.
- Split files over roughly 150 production lines when they mix responsibilities.
- Split functions over roughly 20 production lines when they mix decisions.
- Long tests are acceptable when they act as integration fixtures.
- Avoid many sibling files with repeated prefixes; prefer a focused folder with clear module names.
- When moving code out of a plugin, move ownership upward only if the concept is shared by multiple plugins or belongs to Lua/mod orchestration.

## Current Migration Notes

- `lua-api` currently owns `require`, `std.character`, and current-mod file reads.
- `moveset_patcher` currently owns moveset payload parsing and LinkData patch registration.
- `moveset_patcher` must continue shrinking toward binding registration plus domain parsing only.
- `winapi_file`, `std.character`, `fx_director` reload hooks, and `skin_patcher` virtual manager remain known large files to split.

## Unsafe And Hook Rules

- Unsafe memory and Windows hook code must be isolated.
- Pure parsing and table-building must not depend on unsafe hook modules.
- Signature hooks should be represented as typed SDK services, not hand-written one-off patches in feature plugins.
- Plugins can request hook capabilities, but SDK/host must own validation and diagnostics.

## Config Rules

- Existing configs must never be overwritten.
- Missing default configs may be generated.
- Loader config must stay minimal.
- SDK config belongs to SDK core.
- Plugin config belongs to the plugin folder.
- Mod config belongs to the mod folder or mod manifest.

## Log Rules

- Logs must be grouped by owner: loader, SDK, plugin, mod.
- Debug spam must be behind explicit config.
- Log messages should identify plugin id or mod id when applicable.
- Plugin logs must not be duplicated into loader logs by default.

## Error Rules

- Missing SDK core: clear SDK boot error.
- Invalid plugin manifest: skip plugin.
- Plugin load failure: skip plugin and dependents.
- Lua mod failure: skip mod.
- Duplicate Lua module or extension: fail offending registration.
- Missing capability: refuse operation.
- Plugin callbacks must be authorized through the host context; never trust a plugin id field from FFI payload alone.

## Documentation Rules

- Repository docs are written in English.
- New architecture decisions must be recorded before implementation.
- Public APIs need examples.
- Reverse-engineering evidence should include dates, logs, offsets, hashes, or file paths when useful.

## Git Rules

- Do not commit unless explicitly asked.
- Do not rewrite history unless explicitly asked.
- Before committing, summarize changed files and verification.
