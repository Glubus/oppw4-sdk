# Handoff - Runtime Rank / Difficulty / Rewards / SDK Merge

Date: 2026-05-25

This note is the short "resume here" file for the next chat. It summarizes what
we are currently doing, what is confirmed, what is still unknown, and where to
continue without redoing the same blind tests.

## Current Goal

We are stabilizing the SDK runtime layer and turning the old prototype plugins
into real SDK-owned services.

The current work has four connected tracks:

- difficulty runtime/reverse work;
- rank condition/runtime work;
- rewards runtime/reverse work;
- merging former standalone plugins into the stable SDK services.

The target is not just "make a cheat patch". The target is to expose clean SDK
APIs that Lua mods can use later, while keeping dangerous/experimental hooks
disabled by default.

## SDK Ownership Direction

The loader stays small. Gameplay systems belong to the SDK.

Current ownership target:

- `sdk.runtime`
  - active player/character state;
  - mission/result runtime probes;
  - rank services;
  - reward services;
  - difficulty services;
  - FX runtime;
  - Lua surfaces such as `std.player` and future `std.difficulty`,
    `std.ranks`, `std.rewards`.
- `sdk.rdb`
  - RDB virtual file/patch service;
  - skin/model/portrait replacement API;
  - Lua module `sdk.rdb.patcher`.
- `sdk.linkdata`
  - LinkData virtual file/patch service.
- `moveset_patcher`
  - remains an external official plugin for now, built on SDK LinkData service.

Important: `fx_director` and `skin_patcher` should no longer be standalone
runtime plugins. The direction is:

- old `fx_director` -> internal `sdk.runtime.fx`;
- old `skin_patcher` -> internal `sdk.rdb.patcher`;
- old plugin-style Lua APIs get replaced by SDK-owned modules and character
  extensions.

## Lua Direction

The clean future Lua shape is:

```lua
local player = require("std.player")
local character = require("std.character")
require("sdk.runtime.fx")
require("sdk.rdb.patcher")

local active = player.active_character()
if active then
  active:add_fx({ effect_id = 2830 })
end

local zoro = character.find("zoro")
zoro:add_fx({ effect_id = 2830 })
zoro:replace_costume(2, "zoro.g1m")
```

Nothing prevents both styles from existing:

- `player.active_character():add_fx(...)` for runtime/player-driven mods;
- `character.find("zoro"):add_fx(...)` for metadata/character-targeted mods.

`std.player` is core SDK API. It should grow slowly and stay clean. For now it
only needs active character handles. Later it can expose local player, party,
controller slots, selected character, and runtime player state.

## Difficulty Track

What is confirmed:

- Vanilla difficulty ids are:
  - `0`: Easy / Facile;
  - `1`: Normal / Normale;
  - `2`: Hard / Difficile;
  - `3`: Super Hard / Ultra difficile.
- Runtime selected difficulty is read from `global + 0x1d756`.
- Mission id is read from `global + 0x1d750`.
- Mission mode/type is read from `global + 0x1d753`.
- Reward row lookup hard-stops at `difficulty > 3`, so a true fifth difficulty
  cannot be added by only inserting LinkData rows.
- The current SDK runtime probes can see mission/difficulty/mode state.

What is not fully confirmed:

- exact labels for all difficulty gameplay tables;
- exact damage/HP/defense formula fields;
- exact spawn/capture/territory scaling fields;
- whether all important difficulty behavior is data-driven or partly hardcoded.

Current warning:

- Do not reintroduce blind "Nightmare patch" writes in `sdk.runtime`.
- Nightmare/overall difficulty should be a Lua mod later, using clean APIs.
- Public SDK API should expose things like:
  - edit a known difficulty row;
  - make Easy S-rankable;
  - override known scalar fields;
  - eventually add/remap difficulty ids if the lookup limit is patched safely.

Where to continue:

- Keep reading Ghidra around active difficulty readers.
- Label tables from `docs/reverse-notes/difficulty-reward-ghidra-2026-05-20.md`.
- Prioritize enemy damage/HP/defense and spawn/capture behavior.
- Only promote fields into public `std.difficulty` / SDK API after the field
  meaning is actually confirmed.

## Rank Conditions Track

What is confirmed:

- The visible result screen and global reward rank are related but not the same
  place in code.
- `FUN_14132b570` builds visible result screen state.
- `FUN_14132aae0` is the cleaner global battle-rank calculator.
- The Easy S/S+ cap exists in the global rank path and can be bypassed, but it
  should be exposed as an explicit API/toggle, not hidden inside a probe.
- Berry and medal/item reward paths receive the global rank, so rank matters
  for reward scaling.
- The old assumption that a raw row starting with `60000,60000,48000` was the
  active visible kill threshold row is not reliable.
- Latest Ghidra notes say the active normal result helper row formula is:

```text
row = fixed_owner+0x28+0x4c+rank_row_id*0xdc
```

What went wrong during tests:

- Some experimental count-threshold hooks/patches caused wrong visible kill
  ranks or result crashes.
- Those experiments should stay disabled/removed from default runtime.
- Probes must stay read-only unless an explicit config option says otherwise.

Where to continue:

- Follow the rank helper calls in Ghidra instead of patching random table bytes.
- Focus on:
  - `FUN_14132aae0`;
  - `FUN_1412dd790`;
  - `FUN_1412dd090`;
  - `FUN_1412dd950`;
  - `FUN_1412dd9e0`.
- Label exactly how rank values map to visible labels.
- Confirm the real source of kill/time thresholds per mission/mode/difficulty.
- Build public APIs only after that, likely:
  - `rank.set_easy_s_rankable(true)`;
  - `rank.override_thresholds(...)`;
  - `rank.map_global_rank(...)`.

Design idea parked for later:

- The future overall mod may remap rank labels by removing/repurposing low
  ranks and making higher labels like SS/SSS/PK or X.
- Do not implement that now. First finish clean rank/runtime APIs.

## Rewards Track

What is confirmed:

- Berry rewards are hooked and observed.
- Medal/item reward snapshots are observed.
- Crew points are confirmed as "récompense d'équipage".
- There is no separate character XP reward category; character progression is
  upgrade-based.
- Souls are still not fully labelled because many tested missions did not drop
  useful soul rewards.

Known categories:

- Berry / Beli;
- medals/items;
- crew points;
- souls, still incomplete.

What is missing:

- soul reward commit fields;
- exact reward row labels for all item/soul categories;
- clean reward multiplier/override API.

Where to continue:

- Keep the reward probes in SDK runtime.
- Use missions that actually drop souls when testing.
- Label soul fields before exposing a high-level API.
- `std.rewards` should become a service-backed SDK API, not a separate plugin.
- Future `reward_director` behavior belongs in SDK runtime services plus Lua
  mods, not in the loader.

## Data Direction

`oppw4-data` is the long-term source of truth. Runtime probes should feed data
back into editable files instead of becoming permanent one-off hacks.

Needed data work:

- mission folders split like character folders;
- rank condition data;
- difficulty rows and known scalar labels;
- reward row labels;
- soul reward labels;
- later UI/text/asset references for changing labels such as Easy -> Nightmare.

Important future tool:

- `data_dumper`: select a mission or character, dump the known runtime/fixed
  data into `oppw4-data`, then let humans clean/label it.

## Merge Plugins To Stable SDK

Current direction:

- `fx_director` becomes SDK runtime code, exposed as `sdk.runtime.fx`.
- `skin_patcher` becomes SDK RDB patching code, exposed as `sdk.rdb.patcher`.
- `moveset_patcher` stays external for now.

Why:

- FX is runtime/player/game-state behavior, so it belongs in `sdk.runtime`.
- Skin patching is RDB virtual patching, so it belongs in `sdk.rdb`.
- Difficulty, rank, and rewards are core runtime/game systems, so they should
  also be SDK-owned.
- External plugins should be feature packages built on top of SDK services, not
  duplicate their own file hooks, Lua host, character concepts, or runtime state.

Current practical warning:

- When moving old plugin code into SDK, avoid keeping plugin-shaped globals and
  names forever. The public module can be stable, but the internal code should
  be organized as SDK services.
- The user specifically wants core things inside SDK source trees, not a pile of
  old plugin crates sitting next to SDK services.

## Immediate Next Steps

1. Finish the SDK merge cleanup:
   - verify `sdk.runtime.fx` is fully inside `sdk.runtime`;
   - verify `sdk.rdb.patcher` is owned by `sdk.rdb`;
   - update packaging/docs/tests after the move.

2. Keep rank probes safe:
   - default to read-only;
   - remove or gate experimental patches;
   - no hidden threshold hooks.

3. Continue rank reverse:
   - Ghidra-follow the global rank pipeline;
   - identify real threshold sources;
   - confirm global rank values and reward rank inputs.

4. Continue difficulty reverse:
   - label gameplay scalar fields;
   - identify enemy HP/defense/damage/spawn/capture fields;
   - do not build Nightmare until the field meanings are known.

5. Continue rewards reverse:
   - finish soul labels;
   - shape SDK reward API;
   - keep reward scaling for later Lua/overall mod.

6. Feed confirmed knowledge into `oppw4-data`:
   - missions;
   - rank rows;
   - difficulty rows;
   - reward rows.

## Do Not Repeat

- Do not blindly patch threshold rows just because values look plausible.
- Do not make probe code alter gameplay unless an explicit config toggle enables
  it.
- Do not stop after a crash by leaving the game in a fully safe/no-hypothesis
  state. Each iteration should leave the installed game launchable with exactly
  one new test hypothesis enabled, unless the user explicitly asks to pause.
- Do not put difficulty/reward/rank logic into the loader.
- Do not make difficulty/reward/rank standalone plugins if they are core SDK
  systems.
- Do not let old plugin APIs create separate incompatible character concepts.

## Useful Existing Notes

- `docs/reverse-notes/difficulty-reward-ghidra-2026-05-20.md`
- `docs/reverse-notes/rank-pipeline-ghidra-2026-05-25.md`
- `docs/ROADMAP.md`
- `docs/API-SURFACES.md`
