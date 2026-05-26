# OPPW4 SDK

This workspace is the first physical split of the experimental OPPW4 patcher
into a real SDK project.

The SDK owns the modding platform:

- SDK core orchestration;
- plugin ABI/API crates;
- Lua runtime and generic standard modules;
- OPPW4 character/data APIs backed by the `oppw4-data` submodule;
- RDB and LinkData helpers;
- official plugins;
- examples and modder documentation.

This is a v0 split. Some Rust package names still use prototype names such as
`plugin-sdk`, `plugin-host`, and `lua-api`. The folder layout already follows
the target SDK shape so later renames can happen deliberately.

## Layout

```text
crates/
  sdk-abi/         # current package: plugin-abi
  sdk-api/         # current package: plugin-sdk
  sdk-core/        # current package: plugin-host
  lua-runtime/     # current package: lua-api
  hooks/
  asm/
  rdb/
official_plugins/
  sdk/
    core/         # builds sdk.dll
    data/         # builds data.dll; owns OPPW4 data bootstrap and API crate
    runtime/      # builds runtime.dll; owns sdk.runtime.fx APIs
    debug/        # builds debug.dll
    overlay/      # builds overlay.dll
    linkdata/     # builds linkdata.dll
    rdb/          # builds rdb.dll; owns sdk.rdb.patcher APIs
  moveset_patcher/ # external plugin, packaged as moveset_patcher.dll
oppw4-data/       # data-only submodule for community-editable character data
docs/
```

Clone or update the data submodule before building packages or working on
character data:

```bash
git submodule update --init --recursive
```

## Build

```powershell
cargo test --workspace
```

The `sdk.rdb.patcher` and `sdk.runtime.fx` Lua APIs are SDK services now: they
are provided by `plugins/sdk/rdb.dll` and `plugins/sdk/runtime.dll`. The only
external official plugin currently packaged on its own is `moveset_patcher`.

Validate an external plugin manifest:

```powershell
cargo run -p plugin-manifest-tool -- official_plugins/moveset_patcher/plugin.toml
```

Validate a Lua mod manifest:

```powershell
cargo run -p mod-manifest-tool -- path/to/mod.toml
```

Build the SDK release layout:

```powershell
powershell -ExecutionPolicy Bypass -File tools/package-sdk.ps1
```

On Linux or WSL:

```bash
tools/package-sdk.sh
```

The package is written under `dist/oppw4-sdk/` with the loader proxy as
`dinput8.dll`, SDK services in `plugins/sdk/`, the external
`moveset_patcher` plugin in its own plugin folder, and the mandatory data
repository under `oppw4-data/`. Active mods are not copied into plugin folders;
runtime mods belong under the game-level `mods/` directory.

By default the package script also builds the sibling loader workspace at
`../oppw4-modloader`. Use `LOADER_ROOT=/path/to/oppw4-modloader` on shell or
`-LoaderRoot path\to\oppw4-modloader` on PowerShell when the loader repository
is elsewhere.

Examples are documented in [examples/README.md](examples/README.md).

## Architecture Docs

The public mdBook documentation now lives in the sibling `docs-oppw4-prism`
repository.

- [SDK foundation RFC](docs/RFC-0001-sdk-foundation.md)
- [Roadmap](docs/ROADMAP.md)
- [Architecture rules](docs/RULES.md)
- [API surfaces](docs/API-SURFACES.md)
- [Plugin API design](docs/PLUGIN-API.md)
- [Plugin development](docs/PLUGIN-DEVELOPMENT.md)
- [SDK data dumper concept](docs/DATA-DUMPER.md)
