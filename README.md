# OPPW4 SDK

This workspace is the first physical split of the experimental OPPW4 patcher
into a real SDK project.

The SDK owns the modding platform:

- SDK core orchestration;
- plugin ABI/API crates;
- Lua runtime and standard modules;
- character bank resources;
- RDB and LinkData helpers;
- official plugins;
- examples and modder documentation.

This is a v0 split. Some Rust package names still use the prototype names such
as `plugin-sdk`, `plugin-host`, `lua-api`, and `struct-api`. The folder layout
already follows the target SDK shape so later renames can happen deliberately.

## Layout

```text
crates/
  sdk-abi/         # current package: plugin-abi
  sdk-api/         # current package: plugin-sdk
  sdk-core/        # current package: plugin-host
  lua-runtime/     # current package: lua-api
  character-bank/  # current package: struct-api
  hooks/
  asm/
  rdb/
official_plugins/
  sdk_core/       # builds sdk.dll
  sdk_runtime/    # builds runtime.dll
  sdk_linkdata/   # builds linkdata.dll
  sdk_rdb/        # builds rdb.dll
  skin_patcher/
  fx_director/
  moveset_patcher/
resources/
  character_bank/
    characters/      # editable per-character source files
    generated/       # generated unified views and indexes
    schemas/
docs/
```

## Build

```powershell
cargo test --workspace
```

Official plugins can be built from this workspace once the loader and SDK
contract has stabilized.

Validate a plugin manifest:

```powershell
cargo run -p plugin-manifest-tool -- official_plugins/fx_director/plugin.toml
```

Validate a Lua mod manifest:

```powershell
cargo run -p mod-manifest-tool -- path/to/mod.toml
```

Examples are documented in [examples/README.md](examples/README.md).

## Architecture Docs

- [SDK foundation RFC](docs/RFC-0001-sdk-foundation.md)
- [Roadmap](docs/ROADMAP.md)
- [Architecture rules](docs/RULES.md)
- [API surfaces](docs/API-SURFACES.md)
