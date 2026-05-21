# Project Layout

The SDK repo is organized around runtime ownership boundaries.

```text
oppw4-sdk/
  crates/
    sdk-abi/          # shared ABI structs and callback types
    sdk-api/          # Rust plugin-facing SDK API
    sdk-core/         # plugin host, Lua runtime orchestration, services
    lua-runtime/      # Lua sandbox, require, std modules, mod execution
    character-bank/   # generated/readable character data types
    hooks/            # hook and memory helper crate
    asm/              # assembly helper crate

  official_plugins/
    sdk_core/         # builds sdk.dll
    sdk_runtime/      # builds runtime.dll
    sdk_linkdata/     # builds linkdata.dll
    sdk_rdb/          # builds rdb.dll
    skin_patcher/
    moveset_patcher/
    fx_director/

  oppw4-data/         # data-only submodule
  docs/               # internal architecture notes and reverse notes
  docs-oppw4-prism/   # public developer/modder book
  tools/              # validators and packaging tools
```

## Runtime Shape

At release time the game folder receives:

```text
dinput8.dll
oppw4-data/
plugins/
  sdk/
    sdk.dll
    runtime.dll
    linkdata.dll
    rdb.dll
  skin_patcher/
  moveset_patcher/
  fx_director/
  configs/
mods/
```

Runtime mods go in `mods/`. Plugin config goes in `plugins/configs/<plugin_id>/`.
Official plugins are not mod folders.

## Why It Is Split This Way

The loader must stay small and boring: inject, load SDK core, expose minimal
Windows/process primitives.

The SDK is allowed to know about modding concepts: plugin manifests,
capabilities, Lua, data, official plugin services, and packaging.

This keeps game-specific and modding-specific logic out of `dinput8.dll`.
