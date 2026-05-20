# SDK API Surfaces

## Summary

This document lists the intended public surfaces after the loader/SDK split. It is not a final ABI schema yet. It exists to prevent business logic from leaking back into the loader.

## Loader Host ABI

The loader exposes only native primitives to SDK core.

Required surface:

- loader version;
- host ABI version;
- game root path;
- process module base;
- memory read/write/scan;
- file hook/provider primitive;
- DirectInput/proxy status.

Current SDK core bootstrap ABI:

- `OPPW4_LOADER_SDK_ABI_VERSION`;
- `Oppw4LoaderSdkInit`;
- `oppw4_sdk_core_initialize`;
- loader-owned memory module base/read/write/scan callbacks.
- loader-owned file provider registration callback.
- SDK-owned plugin capability validation callback.

Not allowed in the loader ABI:

- Lua modules;
- plugin dependency logic;
- RDB replacement policy;
- LinkData moveset policy;
- character bank APIs;
- game status and active character services;
- official plugin names.

## SDK Core API

SDK core consumes the loader host ABI and exposes higher-level services to plugins.

Core services:

- plugin registry;
- dependency resolver;
- capability registry;
- Lua runtime;
- Lua module registry;
- standard type extension registry;
- mod discovery;
- hot reload;
- log routing;
- config routing;
- file provider routing;
- hook service;
- signal bus;
- memory service;
- service registration and capability routing.

SDK core does not interpret OPPW4-specific memory or file formats directly.
Game runtime state, LinkData, and RDB behavior should be provided by SDK service
plugins under `plugins/sdk/`.

Expected SDK service bundle:

```text
plugins/
  sdk/
    sdk.dll
    runtime.dll
    linkdata.dll
    rdb.dll
```

`sdk.dll` starts independently. Missing service DLLs disable only the plugins
that depend on those services.

Game telemetry providers register through the SDK ABI. `sdk.runtime` owns the
OPPW4-specific probes and publishes `game.status` / `game.active_character` to
the core; the core only routes those callbacks to consumers.

LinkData providers follow the same rule. `sdk.linkdata` owns LinkData patch
state and virtual file replacement; the core only enforces capabilities and
routes patch requests to the registered service.

Core capabilities such as `plugin.host`, `files.virtualize`, `lua.runtime`,
and memory primitives are available before optional SDK services load. Service
capabilities such as `linkdata.patch` are available only after the matching
service DLL registers successfully.

`sdk.rdb` is the capability owner for `rdb.read` and `rdb.patch`. Existing RDB
patch consumers can keep using `host.rdb().register_patch_provider(...)` during
migration, but new RDB-specific behavior should move behind the RDB service.

RDB virtual providers register through `host.rdb()`. `sdk.rdb` owns the single
loader file provider used for RDB replacement and dispatches open/read/seek,
metadata, and `patch_read` calls to registered RDB providers; consumers should
not register RDB replacements directly through `host.files()`.

## Plugin Manifest Surface

`plugin.toml` declares static intent.

Expected fields:

- manifest version;
- plugin id;
- plugin version;
- entry DLL;
- dependencies;
- Lua modules provided;
- SDK services required;
- capabilities requested;
- capabilities provided.

Current optional schema:

```toml
[plugin]
id = "fx_director"
version = "0.2.0"
entry = "fx_director.dll"

[dependencies]
plugins = ["skin_patcher"]

[lua]
modules = ["fx_director"]

[capabilities]
requires = ["lua.module", "hooks.install"]
provides = ["std.character.extend"]
```

SDK core currently resolves plugin load order from `[dependencies].plugins` and rejects duplicate Lua module names at runtime. Manifest dependency, Lua module, and capability lists are de-duplicated after normalization while preserving first occurrence order. A plugin must declare each Lua module in `[lua].modules`; having only `lua.module` is not enough. Module and capability names are normalized to lowercase and may contain only ASCII letters, digits, `_`, `-`, and `.`. Dotted names cannot be empty, start/end with `.`, or contain `..`. Capability checks are enforced for:

- `lua.module`;
- `std.character.extend`;
- `files.virtualize`;
- `hooks.install`;
- `rdb.patch` for RDB providers that patch existing RDB reads;
- `linkdata.patch`;
- `memory.read`;
- `memory.scan`;
- `memory.write`.
- `signals.subscribe`;
- `signals.emit`.

Other capability declarations are parsed and available for validation, but not every runtime operation has a dedicated SDK API yet.

## Runtime Plugin Registration Surface

Plugin DLLs register runtime capabilities with SDK core.

Registration categories:

- Lua module registration;
- `std.*` type extension registration;
- file provider registration;
- RDB virtual/patch provider registration;
- LinkData patch provider registration;
- hook registration;
- signal subscription/emission registration;
- config schema registration.

SDK core validates manifest declarations against runtime registrations.

Current validation rules:

- runtime Lua module registration requires `lua.module`;
- runtime Lua module registration must match a module name listed in `[lua].modules`;
- duplicate Lua module names from different plugins are rejected;
- `std.character` method extension requires `std.character.extend`;
- generic file providers require `files.virtualize`;
- hook installation helpers require `hooks.install`;
- RDB virtual and patch providers require `rdb.patch`;
- LinkData entry/row patches require `linkdata.patch`;
- memory read/scan/write callbacks require their matching memory capabilities.
- signal subscriptions require `signals.subscribe`;
- signal emits require `signals.emit`.

## Lua Standard Surface

Standard modules use the `std.*` namespace.

Lua mods run in the SDK sandbox. Safe standard libraries such as `math`, `string`, `table`, and `utf8` remain available. Filesystem/process/debug surfaces are hidden from mod scripts: `os`, `io`, `debug`, and global `package` are `nil` during mod execution. `require(...)` stays SDK-controlled and can only resolve registered SDK/plugin modules.

### `std.character`

Responsibilities:

- find characters by canonical id, alias, or known ids;
- expose character bank metadata;
- expose active character when available;
- create unsafe/manual character handles for experiments;
- allow SDK-approved plugin method extensions.

Example:

```lua
local character = require("std.character")
local law = character.find("law")

print(law.ids.runtime)
print(law.models[1].stem)
print(law.linkdata.moveset_entry)
```

### `std.log`

Responsibilities:

- mod-scoped logging;
- debug/info/warn/error helpers;
- route logs through SDK log policy.

Current Lua surface:

```lua
local log = require("std.log")

log.info("loaded")
log.warn("fallback effect id used")
```

The current implementation records mod-scoped entries inside the Lua runtime and returns them to SDK core after each mod run. SDK core writes all entries under `mods/_oppw4/logs/mods/<mod_id>/<session>.log`. To keep release host logs useful, only `warn` and `error` entries are mirrored to the host log.

### `std.mod`

Responsibilities:

- current mod id/name/version;
- mod source type;
- mod-local metadata;
- dependency information.

Current Lua surface:

```lua
local mod = require("std.mod")

local current = mod.current()
print(current.id)
print(current.is_zip)
```

### `std.files`

Responsibilities:

- safe reads from current mod;
- zip and nested zip support;
- no arbitrary filesystem access by default.

Current Rust surface:

- `lua_api::read_mod_text(lua, path)`;
- `lua_api::read_mod_bytes(lua, path)`.

Current Lua surface:

```lua
local files = require("std.files")

local text = files.read_text("moveset.lua")
local bytes = files.read_bytes("payload.bin")
```

These helpers are SDK-owned even before the Lua-facing `std.files` module is finished. Feature plugins should consume these helpers instead of reading `__oppw4_mod_root` or zip archives directly.

## Plugin Lua Surface

Plugin modules use plugin ids.

Examples:

- `skin_patcher`;
- `fx_director`;
- `moveset_patcher`.

Plugins may also extend standard handles:

```lua
local character = require("std.character")
require("skin_patcher")
require("fx_director")

local zoro = character.find("zoro")
zoro:add_fx({ effect_id = 2830 })
zoro:replace_costume(2, "zoro.g1m")
```

Extension conflicts must be rejected by SDK core.

Current transition rule:

- plugins may expose Lua modules and character extension methods;
- plugins should not manage Lua VM lifecycle, mod discovery, hot reload, current-mod filesystem context, or zip nesting;
- any plugin code that reads SDK Lua globals directly is migration debt unless it is only registering bindings.

## Character Bank Surface

The character bank should expose a unified view built from editable source files.

Editable source files are split per character in the `oppw4-data` submodule:

```text
oppw4-data/characters/law/data.json
oppw4-data/characters/zoro/data.json
```

Generated views and indexes live under `oppw4-data/generated/`.

Main domains:

- identity;
- ids;
- models;
- text;
- assets;
- LinkData references;
- RDB references;
- relationships;
- evidence sources.

Plugins can consume the bank through Rust SDK APIs and Lua `std.character`.

## Hook And Signal Surface

The SDK should expose hooks and signals as services.

Hooks:

- named signature definitions;
- install policy;
- capability checks;
- diagnostics;
- ownership and conflict rules.

Signals:

- typed event ids;
- subscribe;
- emit where allowed;
- host/runtime events such as active character changes;
- plugin-defined events.

The loader should not expose one-off feature hooks.

## RDB And LinkData Surface

RDB and LinkData operations are SDK services.

RDB service:

- parse indexes;
- resolve known names/hashes;
- route asset replacements;
- support file providers.

LinkData service:

- parse archives;
- replace entry payloads;
- patch rows where layout is known;
- rebuild virtual file buffers;
- detect edit ownership conflicts.

Feature policy belongs to plugins. The SDK provides primitives and validation.

## Versioned Surfaces

Separate versions must be tracked:

- loader host ABI;
- SDK core ABI;
- SDK API;
- Lua API;
- plugin manifest;
- mod manifest.

Each surface needs explicit compatibility behavior before the physical repo split.
