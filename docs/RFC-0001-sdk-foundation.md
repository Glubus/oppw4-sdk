# RFC-0001: OPPW4 SDK Foundation

## Status

Draft.

## Summary

OPPW4 modding should be split into two Git projects:

- `oppw4-loader`: the minimal native `dinput8.dll` loader.
- `oppw4-sdk`: the real modding platform, shipped as `sdk.dll` plus SDK crates, Lua standard modules, resources, examples, and official plugins.

The loader must become boring and small. The SDK must become the place where modding concepts live: plugins, Lua, character metadata, LinkData/RDB helpers, hooks, signals, logs, configuration, and user-facing APIs.

Current plugin and Lua APIs may be broken during this reset. Long-term clarity is more important than compatibility with the experimental prototype.

## Repository Model

### `oppw4-loader`

The loader repository builds `dinput8.dll`.

It owns:

- DirectInput proxying;
- game root discovery;
- minimal boot/fatal logging;
- loading `plugins/sdk/sdk.dll`;
- loading `sdk.dll` before anything else;
- exposing a small host ABI to SDK core.

It does not own:

- Lua runtime;
- mod discovery;
- plugin orchestration beyond SDK core;
- `std.character`;
- RDB, LinkData, skin, FX, moveset, or gameplay logic.

### `oppw4-sdk`

The SDK repository builds the official SDK and official plugins.

It owns:

- `sdk.dll`;
- SDK ABI/API crates;
- Lua runtime and Lua standard modules;
- character bank resources;
- hook/signal abstractions;
- LinkData/RDB helpers;
- official plugins;
- examples and modder documentation.

The loader consumes the SDK through a Git dependency during early development. Later, tagged releases can be used for stable packaging.

## Boot Model

`dinput8.dll` starts first because the game loads it.

Boot order:

1. Load and forward the real system DirectInput DLL.
2. Locate the OPPW4 game root.
3. Initialize minimal loader logging.
4. Locate `OPPW4/plugins/sdk/sdk.dll`.
5. Load `sdk.dll`.
6. Pass the host ABI to SDK core.
7. SDK core takes over plugin/mod orchestration.

If SDK core is missing, invalid, or incompatible, the loader logs a clear fatal SDK error and disables SDK-dependent features. The loader should still forward DirectInput when possible, so the game is not blocked only because modding is unavailable.

## Host ABI

The loader-to-SDK ABI must stay small and stable.

Initial host primitives:

- game root path;
- loader version and host ABI version;
- native log callback;
- main module base;
- memory read/write/scan primitives;
- file hook/provider primitive;
- DirectInput/proxy status.

Everything else belongs in SDK-level APIs.

## SDK Core

SDK core is mandatory for the standard modloader.

It owns:

- plugin discovery after SDK boot;
- plugin lifecycle;
- dependency ordering;
- `plugin.toml` validation;
- capability and permission registry;
- Lua sandbox creation;
- Lua standard module installation;
- SDK-controlled `require`;
- mod discovery and hot reload;
- plugin log/config routing;
- hook/signal service routing;
- file provider routing;
- RDB and LinkData service routing.

SDK core is an orchestrator. It must not contain business logic for skins, effects, movesets, rewards, or other gameplay features. Those belong to plugins.

## Lua Runtime

Lua mods run inside an SDK-controlled sandbox.

Allowed by default:

- safe base functions needed for normal scripts;
- `math`;
- `string`;
- `table`;
- `utf8`.

Blocked or replaced by SDK-controlled APIs:

- unrestricted `os`;
- unrestricted `io`;
- `debug`;
- free `package` mutation and filesystem require paths.

`require` resolves through the SDK registry. Mods should not load arbitrary files or native libraries through Lua itself.

Standard modules use the `std.*` namespace:

- `std.character`;
- `std.log`;
- `std.mod`;
- `std.files`.

Feature modules are provided by plugins:

- `skin_patcher`;
- `fx_director`;
- `moveset_patcher`;
- future plugins.

Example:

```lua
local character = require("std.character")
require("skin_patcher")
require("fx_director")

local zoro = character.find("zoro")
zoro:add_fx({ effect_id = 2830 })
zoro:replace_costume(2, "zoro.g1m")
```

## Character Bank

The character bank is central SDK data. It is not owned by any single plugin.

It should grow into a complete OPPW4 character database:

- canonical id;
- display names and aliases;
- playable/runtime/boss/model ids;
- model variants, forms, costumes, stems, slots, files;
- text keys, UI labels, descriptions;
- portraits, icons, materials, related assets;
- LinkData entries for movesets, stats, type, and future gameplay tables;
- RDB archive references;
- relationships between base characters, forms, variants, and DLC rows;
- evidence sources and notes.

Source files should stay editable and reviewable under `resources/character_bank/`.
The source of truth is one JSON file per character:

```text
resources/character_bank/characters/law.json
resources/character_bank/characters/zoro.json
```

The SDK can generate a unified Rust/Lua view from multiple source files:

- `characters/*.json`;
- `models.json`;
- `linkdata.json`;
- `text.json`;
- `assets.json`;
- JSON schemas.

Generated machine-facing files live under `resources/character_bank/generated/`
and must not be hand-edited.

`std.character` exposes the unified view to Lua.

Plugins may extend character handles, but they must not create incompatible private character concepts.

## Plugin Model

Plugins declare static intent in `plugin.toml`:

- id;
- version;
- entry DLL;
- dependencies;
- Lua modules;
- required services;
- provided capabilities;
- requested permissions.

Plugin DLLs register runtime capabilities:

- Lua modules;
- extensions to SDK standard types;
- file providers;
- LinkData/RDB patch providers;
- hooks;
- signals;
- config schema.

SDK core validates:

- dependencies;
- load order;
- module name collisions;
- duplicate type extensions;
- permissions/capabilities;
- manifest-vs-registration consistency.

## Official Plugins

Official plugins live in the SDK repository under `official_plugins/`.

They are production plugins and examples for third-party plugin authors.

Initial official plugins:

- `skin_patcher`: RDB/catalog/hash/asset replacement, character skin/model/portrait APIs.
- `fx_director`: runtime effect activation, character FX APIs.
- `moveset_patcher`: LinkData moveset replacement, character moveset APIs.

## Configuration

Configuration ownership:

- loader: minimal boot/debug config only;
- SDK: `plugins/sdk/config.toml`;
- plugin: `plugins/<plugin_id>/config.toml`;
- mod: `mods/<mod_id>/mod.toml` and optional mod-local config files.

Generated default configs may be created when missing, but existing configs must never be overwritten.

## Logging

Logging ownership:

- loader logs: boot and fatal errors only;
- SDK logs: orchestration, Lua, plugin lifecycle, hot reload;
- plugin logs: `plugins/<plugin_id>/logs/`;
- mod logs: routed through `std.log`, grouped by mod id.

Default logs should be useful but quiet. High-volume diagnostics must require debug config.

## Errors

Default error policy:

- missing SDK core: log fatal SDK error and disable SDK features;
- invalid plugin manifest: skip plugin;
- plugin load error: skip plugin and dependents;
- Lua mod error: skip mod;
- duplicate module or extension: fail offending registration;
- missing permission/capability: refuse operation and log.

## Permissions And Capabilities

Capabilities should be visible even before they become strong security boundaries.

Initial examples:

- `memory.scan`;
- `memory.write`;
- `hooks.install`;
- `files.virtualize`;
- `linkdata.patch`;
- `rdb.patch`;
- `lua.module`;
- `std.character.extend`;
- `signals.subscribe`;
- `signals.emit`.

Critical operations should be refused when the plugin did not declare the required capability.

## Versioning

Track versions separately:

- loader host ABI version;
- SDK core ABI version;
- SDK API version;
- Lua API version;
- plugin manifest version;
- mod manifest version.

Version mismatch behavior:

- incompatible SDK: loader logs and disables SDK features;
- incompatible plugin: SDK skips plugin;
- incompatible mod manifest: SDK skips mod.

## Open Follow-Ups

- Decide exact host ABI struct names and version constants.
- Decide exact `plugin.toml` manifest schema.
- Decide exact `mod.toml` manifest schema.
- `std.character` is exposed through `require("std.character")` and as `std.character`; legacy global `character` remains a transition alias only.
- Decide release packaging layout once the SDK repository exists.
