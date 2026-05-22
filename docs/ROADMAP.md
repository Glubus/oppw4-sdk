# OPPW4 SDK Roadmap

## Summary

This roadmap turns the current experimental modloader into a split loader + SDK architecture.

Current checkpoint:

- [x] physical split exists under `oppw4-sdk-split/oppw4-loader` and `oppw4-sdk-split/oppw4-sdk`;
- [x] SDK workspace builds and tests independently;
- [x] SDK core can now be built as `sdk.dll`;
- [x] loader workspace builds independently and loads `plugins/sdk/sdk.dll` dynamically;
- [x] SDK memory primitives are routed back into loader-owned hooks through the loader ABI;
- [x] SDK file providers are routed back into loader-owned hooks through the loader ABI;
- [x] SDK core resolves optional SDK service DLLs from `plugins/sdk/`;
- [x] plugins with missing service capabilities are skipped instead of crashing the SDK core;
- [x] SDK core no longer publishes game status or active character callbacks directly;
- [x] `sdk.runtime` is scaffolded as `runtime.dll` and registers game telemetry providers through the SDK ABI;
- [x] `sdk.runtime` owns the first difficulty telemetry probe for mission id, selected difficulty, mode type, reward mode, and cached difficulty values;
- [x] `sdk.linkdata` owns LinkData patch registry and file virtualization as `linkdata.dll`;
- [x] `sdk.rdb` is scaffolded as `rdb.dll` and owns the `rdb.read` / `rdb.patch` service capabilities;
- [x] RDB patch read providers register through `sdk.rdb` instead of direct plugin file-provider hooks;
- [x] RDB virtual file providers register through `sdk.rdb`; only `sdk.rdb` bridges them to the loader file-provider ABI;
- [x] SDK core seeds core capabilities before resolving optional service capabilities;
- [x] SDK packaging script assembles `plugins/sdk/` plus official plugin folders;
- [x] SDK repository consumes `oppw4-data` as a submodule for collaborative data work;
- [x] SDK package includes the mandatory `oppw4-data/` data tree;
- [x] SDK core initializes the character bank from `game_root/oppw4-data`;
- [x] character bank editable sources are split per character and generated into SDK-facing views;
- [x] Lua mods can use `require("std.character")`;
- [x] legacy global `character` remains only as a transition alias;
- [x] SDK `lua-api` owns current-mod file reads for directory and zip mods;
- [x] SDK `lua-api` reports mod-scoped `std.log` entries back to SDK core;
- [x] SDK core writes `std.log` entries into per-mod log folders under `mods/_oppw4/logs/mods/<mod_id>/`;
- [x] Lua mods run with `os`, `io`, `debug`, and global `package` hidden by the SDK sandbox;
- [x] plugin manifests can declare dependencies, Lua modules, and required/provided capabilities;
- [x] SDK core resolves plugin load order from declared dependencies and rejects duplicate Lua module names;
- [x] SDK core enforces declared capabilities for Lua module registration, `std.character` extension, file virtualization, LinkData patching, and memory read/scan/write APIs;
- [x] SDK plugin ABI carries `struct_size` and reports undersized ABI tables separately from version mismatches;
- [x] Lua module registration now requires both `lua.module` and an explicit `[lua].modules` manifest entry;
- [x] `moveset_patcher` no longer owns mod-file/zip lookup and only consumes SDK helpers for that context.
- [x] `moveset_patcher` no longer scans legacy `mods/LINKDATA_A` folders directly.
- [x] SDK packaging script assembles loader `dinput8.dll`, SDK services, official plugins, data, and root `mods/`.
- [x] Public mdBook documentation moved to the external `oppw4-docs` repository with Mermaid diagrams and CI build checks.
- [x] Character bank tests cover indexed data roots with costumes, assets, body parts, and optional missing movesets.
- [x] LinkData and RDB service tests cover row/entry edit conflicts, row insert/remove behavior, and RDB handle cleanup.
- [x] Difficulty reverse notes include a focused impact export for active difficulty readers and reward row fields.
- [x] Runtime probes now identify Berry reward slots, medal/item reward entries, and crew point totals.
- [x] Crew points are confirmed as the UI "récompense d'équipage" category; no separate crew reward category is tracked for now.
- [x] `rank_threshold_probe` dumps result/rank slots and linked fixed-table rows for threshold comparison.
- [x] Runtime log `2026-05-21-215933` confirms rank row `12` links to condition row `12` for mission `35` normal/free mode.
- [x] Runtime log `2026-05-21-221212` confirms result rank fields include kill count and clear time as f32 bits.
- [x] `data_dumper` concept documented as the bridge from runtime probes to editable `oppw4-data` source folders.
- [x] `sdk.runtime` probe code is split by responsibility: ABI adapters, hook trampolines, memory readers, snapshots, scanners, formatters, hashes, and config parsing are no longer kept as probe monoliths.
- [x] `sdk.debug` exists as `debug.dll` with a hot-reloaded `debug.lua` developer script for memory watches and bounded value scans.
- [x] `sdk.overlay` exists as `overlay.dll` with config, packaging, and renderer module probing for the future in-game UI layer.
- [x] `sdk.debug` publishes changed watch/scan snapshots through `sdk.debug.snapshot`; `sdk.overlay` subscribes and caches those snapshots for future panels.
- [ ] Difficulty row fields `0x334..0x39c` still need runtime labels before a public `difficulty_director` API.
- [ ] Soul reward commit fields still need confirmed runtime labels.
- [ ] LinkData/fixed mission rank threshold fields still need labels from runtime comparison.
- [ ] Mission data domain still needs `oppw4-data/missions/<mission_id>/` source folders, schemas, and generated indexes.
- [ ] `data_dumper` still needs implementation after mission schemas and standard reward/difficulty/rank services are shaped.
- [ ] In-game debug UI/overlay remains future work: `sdk.overlay` currently stops at renderer probing; an `egui`-style UI still needs a DXGI Present/ResizeBuffers backend before it can draw in game.
- [ ] Remaining SDK runtime cleanup: keep shrinking the few files still above roughly 150 lines only when the split exposes a real reusable concept, not as cosmetic churn.

## Next Work Queue

Immediate SDK work after the `sdk.debug` and `sdk.overlay` scaffolds:

1. Build the real overlay backend: hook DXGI `IDXGISwapChain::Present`, handle `ResizeBuffers`, initialize an egui-compatible D3D11 renderer, and capture input through WndProc or a minimal input polling layer.
2. Turn the cached `sdk.debug.snapshot` payloads into renderable overlay panels with pause/resume, clear, and export actions.
3. Extend `sdk.debug` carefully: optional memory writes can exist later, but must be disabled by default and require explicit config.
4. Continue runtime reverse work with the game: label difficulty row fields `0x334..0x39c`, soul reward commit fields, and fixed mission rank threshold fields.
5. Add mission data to `oppw4-data`: `missions/<mission_id>/` source folders, schemas, generated indexes, and evidence notes.
6. Implement `data_dumper` once mission/reward/rank schemas are shaped enough to export probe findings into data files instead of ad-hoc logs.

## Progress Checklist

- [x] Phase 0: Design Freeze
- [x] Phase 1: Loader And SDK Contract
- [ ] Phase 2: SDK Workspace
- [x] Phase 3: Lua Standard Runtime
- [ ] Phase 4: Character Bank
- [x] Phase 5: Plugin Registration And Capabilities
- [x] Phase 6: Official Plugin Migration
- [x] Phase 7: Loader Reduction
- [x] Phase 8: Developer Experience
- [x] Phase 9: Release Packaging

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
- [x] Remove business concepts from the loader contract.

Deliverables:

- [x] host ABI structs and version constants;
- [x] loader-owned memory primitive callbacks;
- [x] loader-owned file provider registration callback;
- [x] no loader-owned game status or active character callbacks;
- [x] `plugins/sdk/sdk.dll` discovery rules;
- [x] missing SDK core disables SDK features without aborting the game;
- [x] tests for missing SDK core and incompatible SDK core.

Exit criteria:

- [x] loader can boot with SDK core;
- [x] loader tolerates missing SDK core without aborting;
- [x] loader does not know official plugin names.

## Phase 2: SDK Workspace

Progress: partially complete.

Goals:

- [x] Create the SDK workspace as its own Git project.
- [x] Move or recreate SDK-facing crates.
- [x] Add SDK core.
- [x] Add `oppw4-data` as the mandatory data submodule.
- [x] Resolve optional SDK service DLLs from `plugins/sdk/`.
- [x] Move LinkData patching/virtualization out of SDK core into `sdk.linkdata`.
- [x] Add `sdk.rdb` service DLL as the capability owner for RDB APIs.
- [x] Route skin patcher RDB `patch_read` through `sdk.rdb`.

Target workspace:

```text
oppw4-sdk/
  crates/
  official_plugins/
    sdk/
      core/         # builds sdk.dll
      runtime/      # builds runtime.dll
      debug/        # builds debug.dll
      overlay/      # builds overlay.dll
      linkdata/     # builds linkdata.dll
      rdb/          # builds rdb.dll
    skin_patcher/
    fx_director/
    moveset_patcher/
  oppw4-data/       # data-only submodule
  examples/
  docs/
```

Exit criteria:

- [x] SDK workspace builds independently;
- [x] SDK core can be packaged as a plugin;
- [x] loader can consume SDK via the shared ABI during development.

Status: mostly complete for the split prototype. The workspace exists, tests pass, `official_plugins/sdk/core` can build `sdk.dll`, and the loader consumes it dynamically through `Oppw4LoaderSdkInit`.
The SDK service plugins are grouped under `official_plugins/sdk/{core,runtime,debug,overlay,linkdata,rdb}`. `sdk.runtime` now has domain folders for game telemetry, mission probes, rewards, reverse probes, shared memory readers, and runtime exposure orchestration. `sdk.debug` is the generic reverse/debug tool layer and should stay separate from fixed OPPW4 runtime probes. `sdk.overlay` is the future UI/draw service and should stay separate from debug memory logic.

## Phase 3: Lua Standard Runtime

Progress: complete for the current SDK std pass.

Goals:

- [x] Move Lua orchestration into SDK core.
- [x] Implement safe Lua sandbox.
- [x] Implement SDK-controlled `require`.
- [x] Add initial standard modules.

Standard modules:

- [x] `std.character`;
- [x] `std.log`;
- [x] `std.mod`;
- [x] `std.files`;
- [x] `std.math`;
- [x] `std.path`;
- [x] `std.time`;
- [x] `std.collections`;
- [x] `std.buffer`.

Exit criteria:

- [x] Lua mods can load `std.character`;
- [x] unsafe Lua libraries are unavailable by default;
- [x] directory and zip/nested zip mods can load;
- [x] directory mods can hot reload;
- [x] finalize log level/filter policy.

Status: complete for the current SDK std pass. `std.character`, `std.files`, `std.mod`, `std.math`, `std.path`, `std.time`, `std.collections`, `std.buffer`, and `std.log` are implemented. Standard modules live under `crates/lua-runtime/src/std_plugins/` with one folder per module, while runtime internals keep ownership of sandboxing, `require`, mod context, and registration. SDK-owned mod-file reads exist in Rust and Lua. `std.log` entries return to SDK core after each mod run and are written into per-mod log folders. Release host logs mirror only mod `warn` and `error` entries. Lua mods run with unsafe filesystem/process/debug globals hidden, while SDK-controlled `require` remains available.

## Phase 4: Character Bank

Progress: complete for the current SDK capability pass.

Goals:

- [ ] Turn character data into a complete SDK-readable bank.
- [x] Split editable data into per-character files.
- [x] Move editable data to the `oppw4-data` submodule.
- [x] Add schemas.
- [x] Generate or expose a unified Rust/Lua view.
- [x] Read `oppw4-data/generated/index.json` and source character folders at SDK boot.

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
- [x] each character has editable source files under `oppw4-data/characters/`;
- [x] runtime/index data is generated rather than hand-edited;
- [x] plugins can read bank data through SDK APIs;
- [x] plugins can extend character handles without redefining character identity.

Status: in progress. Per-character editable JSON, costume files, generated indexes, schemas, and Rust/Lua lookup exist. SDK core now initializes the runtime character bank from `game_root/oppw4-data`, so packaged data can be improved without recompiling SDK code. The bank is not complete yet for every asset/text/model relationship.

Recent validation added coverage for indexed data roots that preserve costume
assets, body parts, multiple named parts/weapons, and deliberately missing
moveset references. This protects the data shape needed by `skin_patcher` and
future community data contributions.

## Phase 5: Plugin Registration And Capabilities

Progress: in progress.

Goals:

- [x] Define manifest schema.
- [x] Define initial runtime registration API.
- [x] Add dependency ordering.
- [x] Add capability declarations and partial validation.

Capabilities:

- [x] `config.schema`;
- [x] `memory.scan`;
- [x] `memory.write`;
- [x] `hooks.install`;
- [x] `files.virtualize`;
- [x] `linkdata.patch`;
- [x] `rdb.patch`;
- [x] `lua.module`;
- [x] `std.character.extend`;
- [x] `signals.subscribe`;
- [x] `signals.emit`.

Exit criteria:

- [x] duplicate modules fail clearly;
- [x] duplicate character methods fail clearly;
- [x] missing capabilities are refused for every critical API;
- [x] plugin dependencies are resolved before Lua mods run.

Status: complete for the current SDK capability pass. `plugin.toml` can declare plugin dependencies, Lua modules, and required/provided capabilities. SDK core resolves plugin load order from declared dependencies, rejects duplicate Lua module names from different plugins, and refuses Lua modules that are not declared in `[lua].modules`. Capability enforcement exists for `lua.module`, `std.character.extend`, `files.virtualize`, `hooks.install`, `rdb.patch`, `linkdata.patch`, `memory.read`/`memory.scan`/`memory.write`, `signals.subscribe`/`signals.emit`, and `config.schema`. ABI diagnostics distinguish version mismatch from undersized API tables, and SDK host call errors include human-readable reasons for known failure codes. `fx_director` and `sdk_runtime` register their config defaults through the SDK config schema API.

## Phase 6: Official Plugin Migration

Progress: in progress.

Goals:

- [x] Move official plugins into the SDK repository.
- [x] Rebuild official plugins fully on top of SDK services.
- [x] Remove direct Lua global mutation.

Plugins:

- [x] `skin_patcher`;
- [x] `fx_director`;
- [x] `moveset_patcher`.

Exit criteria:

- [x] plugins register Lua modules through SDK;
- [x] plugins extend `std.character` through SDK;
- [x] plugins use SDK LinkData/RDB/file services completely;
- [x] plugin logs/config are routed by SDK completely.
- [x] `skin_patcher` exposes character model and texture replacement helpers through `std.character` handles.

Status: complete for the current SDK split. Official plugins live in the SDK repo, register Lua modules through SDK APIs, route LinkData/RDB/file operations through SDK services, and use SDK log/config roots. Remaining plugin work is feature-level cleanup, not loader/SDK boundary migration.

Recent service coverage added LinkData tests for row/entry ownership conflicts,
row edits applied from archive payloads, and row insert/remove behavior. RDB
tests now cover handle cleanup, and `sdk_rdb` tests live in a dedicated
`tests.rs` module instead of growing `lib.rs`.

## Phase 7: Loader Reduction

Progress: started.

Goals:

- [x] Remove plugin orchestration from loader.
- [x] Remove Lua runtime from loader.
- [x] Remove business services from loader.
- [x] Keep boot/proxy/primitives only.

Exit criteria:

- [x] loader repository builds independently;
- [x] loader has no dependency on official plugins;
- [x] loader has no dependency on character bank data;
- [x] loader only exposes the minimal host ABI.

## Phase 8: Developer Experience

Progress: started.

Goals:

- [x] Make the SDK usable by third-party modders.
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
- [x] SDK docs explain where each feature belongs.

Status: complete for the current split. Examples, manifest validators, and
plugin development notes exist. Public modder/developer documentation now lives
in the external `oppw4-docs` repository as an mdBook with Mermaid support,
workflow pages, contributor checklists, performance watchpoints, and a GitHub
Actions build check. Future public docs can expand API examples, but the
repository now has enough structure for third-party plugin/mod work.

## Phase 9: Release Packaging

Progress: in progress.

Goals:

- [x] Define SDK release layout.
- [x] Package SDK services and official plugins for end users.
- [x] Package loader and SDK together for end users.

Expected package:

```text
OPPW4/
  dinput8.dll
  oppw4-data/
    characters/
    generated/
    schemas/
  plugins/
    sdk/
      sdk.dll
      runtime.dll
      debug.dll
      overlay.dll
      linkdata.dll
      rdb.dll
    skin_patcher/
    fx_director/
    moveset_patcher/
  mods/
```

Exit criteria:

- [x] package contains mandatory data without embedding it in plugin folders;
- [x] release package can be copied into a clean game folder;
- [x] missing SDK/plugin errors are readable;
- [x] examples are documented but not accidentally enabled.
