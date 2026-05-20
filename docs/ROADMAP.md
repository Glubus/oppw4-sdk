# OPPW4 SDK Roadmap

## Summary

This roadmap turns the current experimental modloader into a split loader + SDK architecture.

Current checkpoint:

- [x] physical split exists under `oppw4-sdk-split/oppw4-loader` and `oppw4-sdk-split/oppw4-sdk`;
- [x] SDK workspace builds and tests independently;
- [x] SDK core can now be built as `sdk_core.dll`;
- [x] loader workspace builds independently and loads `plugins/sdk_core/sdk_core.dll` dynamically;
- [x] SDK memory primitives are routed back into loader-owned hooks through the loader ABI;
- [x] SDK file providers are routed back into loader-owned hooks through the loader ABI;
- [x] SDK game status and active character reads are routed back into loader-owned hooks through the loader ABI;
- [x] character bank editable sources are split per character and generated into SDK-facing views;
- [x] Lua mods can use `require("std.character")`;
- [x] legacy global `character` remains only as a transition alias;
- [x] SDK `lua-api` owns current-mod file reads for directory and zip mods;
- [x] SDK `lua-api` reports mod-scoped `std.log` entries back to `sdk_core`;
- [x] SDK core writes `std.log` entries into per-mod log folders under `mods/_oppw4/logs/mods/<mod_id>/`;
- [x] Lua mods run with `os`, `io`, `debug`, and global `package` hidden by the SDK sandbox;
- [x] plugin manifests can declare dependencies, Lua modules, and required/provided capabilities;
- [x] SDK core resolves plugin load order from declared dependencies and rejects duplicate Lua module names;
- [x] SDK core enforces declared capabilities for Lua module registration, `std.character` extension, file virtualization, LinkData patching, and memory read/scan/write APIs;
- [x] Lua module registration now requires both `lua.module` and an explicit `[lua].modules` manifest entry;
- [x] `moveset_patcher` no longer owns mod-file/zip lookup and only consumes SDK helpers for that context.

## Progress Checklist

- [x] Phase 0: Design Freeze
- [ ] Phase 1: Loader And SDK Contract
- [ ] Phase 2: SDK Workspace
- [ ] Phase 3: Lua Standard Runtime
- [ ] Phase 4: Character Bank
- [ ] Phase 5: Plugin Registration And Capabilities
- [ ] Phase 6: Official Plugin Migration
- [ ] Phase 7: Loader Reduction
- [ ] Phase 8: Developer Experience
- [ ] Phase 9: Release Packaging

## Phase 0: Design Freeze

Progress: mostly complete.

Goals:

- [x] Write the SDK foundation RFC.
- [x] Write SDK rules and API surface documents.
- [x] Inventory current crates, plugins, runtime folders, and known reverse-engineering data.
- [x] Mark prototype APIs that may be broken.

Deliverables:

- [x] `docs/RFC-0001-sdk-foundation.md`;
- [x] `docs/ROADMAP.md`;
- [x] `docs/RULES.md`;
- [x] `docs/API-SURFACES.md`;
- [x] updated README references when the plan is accepted.

Exit criteria:

- [x] loader responsibilities are explicit;
- [x] SDK responsibilities are explicit;
- [x] Lua runtime policy is explicit;
- [x] character bank ownership is explicit;
- [x] official plugin ownership is explicit.

Status: complete for the current foundation pass. The docs remain living architecture notes and must be updated when ownership changes.

## Phase 1: Loader And SDK Contract

Progress: partially complete.

Goals:

- [x] Define the initial loader-to-SDK ABI.
- [x] Define initial SDK core discovery.
- [x] Define initial SDK missing/incompatible behavior.
- [ ] Remove business concepts from the loader contract.

Deliverables:

- [x] host ABI structs and version constants;
- [x] loader-owned memory primitive callbacks;
- [x] loader-owned file provider registration callback;
- [x] loader-owned game status and active character callbacks;
- [x] `plugins/sdk_core/sdk_core.dll` discovery rules;
- [x] boot/fatal log behavior for missing SDK core;
- [ ] tests for missing SDK core and incompatible SDK core.

Exit criteria:

- [x] loader can boot with SDK core;
- [x] loader can report SDK boot failure clearly;
- [ ] loader does not know official plugin names.

## Phase 2: SDK Workspace

Progress: partially complete.

Goals:

- [x] Create the SDK workspace as its own Git project.
- [x] Move or recreate SDK-facing crates.
- [x] Add `sdk_core`.
- [x] Add character bank resources.

Target workspace:

```text
oppw4-sdk/
  crates/
  official_plugins/
    sdk_core/
    skin_patcher/
    fx_director/
    moveset_patcher/
  resources/
  examples/
  docs/
```

Exit criteria:

- [x] SDK workspace builds independently;
- [x] SDK core can be packaged as a plugin;
- [x] loader can consume SDK via the shared ABI during development.

Status: mostly complete for the split prototype. The workspace exists, tests pass, `official_plugins/sdk_core` can build `sdk_core.dll`, and the loader consumes it dynamically through `Oppw4LoaderSdkInit`.

## Phase 3: Lua Standard Runtime

Progress: in progress.

Goals:

- [x] Move Lua orchestration into SDK core.
- [x] Implement safe Lua sandbox.
- [x] Implement SDK-controlled `require`.
- [x] Add initial standard modules.

Standard modules:

- [x] `std.character`;
- [x] `std.log`;
- [x] `std.mod`;
- [x] `std.files`.

Exit criteria:

- [x] Lua mods can load `std.character`;
- [x] unsafe Lua libraries are unavailable by default;
- [x] directory and zip/nested zip mods can load;
- [x] directory mods can hot reload;
- [x] finalize log level/filter policy.

Status: in progress. `std.character`, `std.files`, `std.mod`, and `std.log` are implemented. SDK-owned mod-file reads exist in Rust and Lua. `std.log` entries now return to `sdk_core` after each mod run and are written into per-mod log folders. Release host logs mirror only mod `warn` and `error` entries. Lua mods run with unsafe filesystem/process/debug globals hidden, while SDK-controlled `require` remains available.

## Phase 4: Character Bank

Progress: in progress.

Goals:

- [ ] Turn character data into a complete SDK-owned bank.
- [x] Split editable data into per-character files.
- [ ] Add schemas.
- [x] Generate or expose a unified Rust/Lua view.

Data domains:

- [x] per-character source files;
- [x] identities;
- [x] ids;
- [ ] models/forms/costumes;
- [ ] text;
- [ ] assets;
- [x] LinkData;
- [ ] RDB references;
- [ ] relationships;
- [ ] evidence sources.

Exit criteria:

- [x] `std.character.find("law")` returns canonical metadata;
- [x] each character has an editable source file under `resources/character_bank/characters/`;
- [x] runtime/index data is generated rather than hand-edited;
- [x] plugins can read bank data through SDK APIs;
- [x] plugins can extend character handles without redefining character identity.

Status: in progress. Per-character editable JSON, generated indexes, and Rust/Lua lookup exist. The bank is not complete yet for every asset/text/model relationship.

## Phase 5: Plugin Registration And Capabilities

Progress: in progress.

Goals:

- [x] Define manifest schema.
- [x] Define initial runtime registration API.
- [x] Add dependency ordering.
- [x] Add capability declarations and partial validation.

Capabilities:

- [x] `memory.scan`;
- [x] `memory.write`;
- [x] `hooks.install`;
- [x] `files.virtualize`;
- [x] `linkdata.patch`;
- [x] `rdb.patch`;
- [x] `lua.module`;
- [x] `std.character.extend`;
- [ ] `signals.subscribe`;
- [ ] `signals.emit`.

Exit criteria:

- [x] duplicate modules fail clearly;
- [x] duplicate character methods fail clearly;
- [ ] missing capabilities are refused for every critical API;
- [x] plugin dependencies are resolved before Lua mods run.

Status: in progress. `plugin.toml` can now declare plugin dependencies, Lua modules, and required/provided capabilities. SDK core resolves plugin load order from declared dependencies, rejects duplicate Lua module names from different plugins, and refuses Lua modules that are not declared in `[lua].modules`. Capability enforcement exists for `lua.module`, `std.character.extend`, `files.virtualize`, `hooks.install`, `rdb.patch`, `linkdata.patch`, and `memory.read`/`memory.scan`/`memory.write`. Remaining enforcement work: signals, config schema registration, and richer diagnostics.

## Phase 6: Official Plugin Migration

Progress: in progress.

Goals:

- [x] Move official plugins into the SDK repository.
- [ ] Rebuild official plugins fully on top of SDK services.
- [ ] Remove direct Lua global mutation.

Plugins:

- [x] `skin_patcher`;
- [x] `fx_director`;
- [x] `moveset_patcher`.

Exit criteria:

- [x] plugins register Lua modules through SDK;
- [x] plugins extend `std.character` through SDK;
- [ ] plugins use SDK LinkData/RDB/file services completely;
- [ ] plugin logs/config are routed by SDK completely.

Status: in progress. Official plugins live in the SDK repo. `moveset_patcher` is being reduced toward “register functions only”; `skin_patcher` and `fx_director` still contain more transitional runtime glue than desired.

## Phase 7: Loader Reduction

Progress: started.

Goals:

- [ ] Remove plugin orchestration from loader.
- [ ] Remove Lua runtime from loader.
- [ ] Remove business services from loader.
- [ ] Keep boot/proxy/primitives only.

Exit criteria:

- [ ] loader repository builds independently;
- [ ] loader has no dependency on official plugins;
- [ ] loader has no dependency on character bank data;
- [ ] loader only exposes the minimal host ABI.

## Phase 8: Developer Experience

Progress: started.

Goals:

- [ ] Make the SDK usable by third-party modders.
- [x] Provide examples and templates.
- [x] Add validation tools.

Deliverables:

- [x] first Lua mod example;
- [x] first Rust plugin example;
- [x] character extension example;
- [x] LinkData patch example;
- [x] asset replacement example;
- [x] `plugin.toml` validator;
- [x] `mod.toml` validator.

Exit criteria:

- [x] a new developer can create a Lua mod from examples;
- [x] a new developer can create a Rust plugin from a template;
- [ ] SDK docs explain where each feature belongs.

## Phase 9: Release Packaging

Progress: not started.

Goals:

- [ ] Define final release layout.
- [ ] Package loader and SDK together for end users.

Expected package:

```text
OPPW4/
  dinput8.dll
  plugins/
    sdk_core/
    skin_patcher/
    fx_director/
    moveset_patcher/
```

Exit criteria:

- [ ] release package can be copied into a clean game folder;
- [ ] missing SDK/plugin errors are readable;
- [ ] examples are documented but not accidentally enabled.
