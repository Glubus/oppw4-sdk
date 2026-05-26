# Crates Cleanup Audit

This document lists the code that should not live in `crates/` anymore, because it
is game-specific, duplicated by SDK runtime plugins, or makes low-level crates own
high-level OPPW4 behavior.

## Rule

`crates/` should stay boring and reusable:

- low-level hook primitives;
- ABI definitions;
- plugin host/client plumbing;
- generic Lua runtime/sandbox pieces;
- raw binary parsers.

OPPW4 gameplay concepts, offsets, player state, ranks, rewards, difficulty,
mission data, curated LinkData entries, and patcher policies should live under
`official_plugins/sdk/*` or a clearly named OPPW4 data/runtime crate.

## Keep In `crates/`

These are fine as crate-level infrastructure, but some need cleanup.

| Crate | Keep | Remove from this crate |
| --- | --- | --- |
| `crates/asm` | Assembly helpers and low-level patch building. | Nothing obvious. |
| `crates/hooks` | Inline hooks, memory reads/writes, signature scanning, signal bus, file-open hook primitives. | Active character state, game status inference, OPPW4 file path semantics. |
| `crates/sdk-abi` | Stable ABI structs, provider tables, callback contracts. | Runtime probing logic. ABI may mention OPPW4 types, but must not own behavior. |
| `crates/sdk-api` | Public plugin SDK API, manifest parsing, typed client helpers. | Curated game data such as named LinkData entries. |
| `crates/sdk-core` | Plugin loading, ABI routing, host lifecycle, generic capability routing. | Character-bank bootstrap, active player snapshots, OPPW4 data ownership. |
| `crates/lua-runtime` | Lua sandbox, `require`, mod file loading, generic std modules. | OPPW4 std modules such as character, player, difficulty, ranks, rewards, mission data. |
| `crates/rdb` | Raw RDB parser and format-level structures. | OPPW4 asset catalog policy, known virtual archive scans, patcher-specific name discovery. |

## Must Move

| Priority | Current path | Problem | Target owner | Action |
| --- | --- | --- | --- | --- |
| P0 | `crates/hooks/src/active_character.rs` | `hooks` reads OPPW4 runtime offsets, computes FX owner offsets, stores active character snapshots, and emits `active_character_changed`. This is gameplay/runtime state, not hook infrastructure. | `official_plugins/sdk/runtime/src/game/active_character/` | Move probing and snapshot ownership there. Keep only the low-level hook/memory helpers in `hooks`. |
| P0 | `crates/hooks/src/status.rs` | `hooks` infers game phase from OPPW4 paths like `.rdb.bin`, `dlc_character_`, virtual resource names, and RDB files. | `official_plugins/sdk/runtime/src/game/status/` or SDK runtime status service | Make `hooks` emit raw file-open observations only. Move OPPW4 interpretation to runtime. |
| P0 | `crates/hooks/src/lib.rs` exports for active character/status | Public exports make `hooks` look like an OPPW4 gameplay crate. | Same as above | Remove `publish_local_player`, `active_character_snapshot`, `ActiveCharacter`, `ACTIVE_CHARACTER_CHANGED`, `game_status`, `mark_file_open`, and `GameStatus` from the `hooks` public API once callers are migrated. |
| P0 | `crates/sdk-core/src/runtime/lua/runner.rs` active player snapshot injection | Core Lua runner injects `__oppw4_active_characters`. The host should not own player-state semantics. | SDK runtime Lua module/provider | Replace direct injection with a registered runtime provider or module export. |
| P1 | `crates/sdk-core/src/runtime/loader/mod.rs` character-bank initialization | Plugin host initializes OPPW4 character/mission data through `struct_api::initialize_data_root`. | SDK runtime/data service | Move data-root initialization to an OPPW4 runtime/data plugin. Core should only start services. |
| P1 | `crates/lua-runtime/src/std_plugins.rs` and OPPW4 modules | The crate is named like a generic Lua runtime but ships `character`, `player`, `difficulty`, `mission_data`, `ranks`, and `rewards`. | `official_plugins/sdk/runtime` or a dedicated SDK data module | Keep generic std modules in `lua-runtime`; register OPPW4 modules from runtime/data ownership. |
| P1 | `crates/sdk-api/src/linkdata/entries/*` | Public SDK contains curated observed entries such as Garp/Rayleigh layouts. That is game data, not SDK surface. | `official_plugins/sdk/linkdata` or `official_plugins/moveset_patcher` | Keep generic LinkData types/helpers in SDK; move named layouts and observed entries out. |
| P2 | `crates/character-bank` package `struct-api` | Name says character bank, but crate also owns missions, game phase, difficulty/rank/reward observations. | Rename/split into OPPW4 data ownership | Either rename to `oppw4-data`/`game-data`, or split `mission-bank` and remove game phase. |
| P2 | `crates/character-bank/src/game.rs` | `GamePhase` overlaps with hook/runtime status concepts. | Runtime status service | Keep one status model. Do not keep a second game phase enum in a data bank. |
| P2 | `crates/rdb/src/catalog.rs` and catalog exports | Name/hash catalog scanning knows asset extensions and archive naming policy. It is useful, but not raw RDB parsing. | `official_plugins/sdk/rdb/patcher` or a clearly named `rdb-tooling` crate | Keep raw parser in `rdb`; move catalog/policy scans to patcher/tooling ownership. |

## Duplicate Map

These are the main duplicate or overlapping ownership zones.

| Concept | Current duplicates | Desired owner |
| --- | --- | --- |
| Active character | `crates/hooks/src/active_character.rs`, `official_plugins/sdk/runtime/src/game/active_character/*`, `crates/sdk-core/src/runtime/ffi/status/*`, Lua runner snapshot injection | SDK runtime active-character service |
| Game loading/status | `crates/hooks/src/status.rs`, `crates/character-bank/src/game.rs`, SDK runtime status FFI | SDK runtime status service |
| Lua OPPW4 std API | `crates/lua-runtime/src/std_plugins.rs`, runtime/data SDK modules, docs for `sdk.ranks`/difficulty/rewards | SDK runtime/data modules registered into Lua |
| Character/mission data | `crates/character-bank`, `crates/lua-runtime` std modules, `sdk-core` loader bootstrap | OPPW4 data service |
| LinkData observed layouts | `crates/sdk-api/src/linkdata/entries/*`, `official_plugins/sdk/linkdata`, `official_plugins/moveset_patcher` | SDK LinkData plugin or moveset patcher data |
| RDB catalog/policy | `crates/rdb/src/catalog.rs`, `official_plugins/sdk/rdb/patcher` | RDB patcher/tooling layer |

## Proposed Target Layout

```text
crates/
  asm/                 # low-level only
  hooks/               # hook/memory/signal primitives only
  lua-runtime/         # sandbox + generic Lua std modules only
  rdb/                 # raw RDB parser only
  sdk-abi/             # ABI contracts
  sdk-api/             # public plugin SDK helpers
  sdk-core/            # host/plugin routing

official_plugins/sdk/runtime/
  src/game/active_character/
  src/game/status/
  src/lua/             # registers OPPW4 Lua modules
  src/data/            # character/mission/rank/difficulty/reward service

official_plugins/sdk/linkdata/
  src/entries/         # observed OPPW4 LinkData layouts

official_plugins/sdk/rdb/patcher/
  src/catalog.rs       # name/hash catalog and archive policy
```

## Migration Order

1. Move `active_character` and `status` behavior out of `hooks`.
   Keep compatibility shims only if needed, but mark them temporary.

2. Stop direct active-character injection from `sdk-core` Lua runner.
   The runtime plugin should register the OPPW4 player/character Lua state.

3. Move OPPW4 Lua std modules out of `lua-runtime`.
   `lua-runtime` should expose a registration API; runtime/data owns module
   content such as ranks, difficulty, rewards, missions, and player state.

4. Move character-bank initialization out of `sdk-core`.
   The host should initialize services, not know the OPPW4 data root layout.

5. Move curated LinkData entries out of `sdk-api`.
   The public SDK can expose generic LinkData types, but named observed layouts
   belong to LinkData tooling or the moveset patcher.

6. Decide the future of `character-bank`.
   If it stays in `crates/`, rename it to match reality. If `crates/` is meant
   to stay infrastructure-only, move it under SDK runtime/data ownership.

7. Split `rdb` raw parser from RDB tooling.
   Raw parsing can stay in `crates/rdb`; name catalog scans and patcher policy
   should move to patcher/tooling ownership.

## Naming Problems To Fix

| Current name | Issue | Better name |
| --- | --- | --- |
| `character-bank` | Contains missions and game state, not only characters. | `oppw4-data`, `game-data`, or split into `character-bank` + `mission-bank`. |
| `struct-api` | Too vague for a public dependency name. It does not say OPPW4, data, or runtime. | `oppw4-data-api` or `game-data-api`. |
| `lua-api` package for `crates/lua-runtime` | Sounds like public SDK API, but it is runtime/sandbox implementation. | `lua-runtime` or `sdk-lua-runtime`. |
| `hooks::status` | Status is game lifecycle logic, not hook logic. | `runtime::game_status` or `oppw4_status`. |
| `hooks::active_character` | Active character is gameplay state, not hook infrastructure. | `runtime::active_character`. |
| `sdk-api::linkdata::entries` | `entries` hides that these are curated observed game layouts. | `observed_layouts`, under `sdk-linkdata` or `moveset_patcher`. |

## Red Line

No new gameplay concept should be added directly to `crates/hooks`,
`crates/lua-runtime`, or `crates/sdk-core`.

If a feature knows an OPPW4 offset, file path convention, character id, mission
rank, reward rule, LinkData entry name, or DLC convention, it belongs to SDK
runtime/data/tooling ownership, not to generic crates.
