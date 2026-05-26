# Runtime Ownership Map

## Runtime core

- `official_plugins/sdk/runtime/src/runtime/mod.rs`: plugin bootstrap; it registers game runtime, control subscribers, runtime Lua modules, probes, and FX. It stays in `sdk.runtime` as orchestration, but it currently mixes core ownership with Lua frontend registration.
- `official_plugins/sdk/runtime/src/runtime/exposure.rs`: common `RuntimeExposure` install trait for config-driven runtime features. This is core-compatible because it has no scripting VM dependency.
- `official_plugins/sdk/runtime/src/runtime/signals.rs`: SDK signal names and JSON emission helpers. This is core-compatible as the bridge between hooks/control paths and frontends, but signal names should remain public concepts rather than reverse names.
- `official_plugins/sdk/runtime/src/runtime/memory.rs`: runtime memory helpers. This belongs in core/runtime infrastructure because hooks and mutations need typed memory access.
- `official_plugins/sdk/runtime/src/runtime/reader.rs`: shared runtime readers used by probes and snapshots. It is core-compatible if it stays a read helper and does not become a Lua API.
- `official_plugins/sdk/runtime/src/rewards/control.rs`: subscribes to `sdk.runtime.rewards.stage_rule` and stages parsed reward rules. It is a temporary core control path and should be replaced or backed by the event/mutation bus in later steps.
- `official_plugins/sdk/runtime/src/rewards/rules.rs`: current reward rule store and berry mutation implementation. It is the closest existing reward mutation MVP, but the types are still staged-rule oriented instead of `RewardCommitEvent`/`RewardMutation`.
- `official_plugins/sdk/runtime/src/rewards/commit_hook.rs`: owns the `reward_commit_14132a670` hook, logs reward snapshots, emits `REWARD_COMMIT`, and applies staged berry rules after the original call. This is runtime core territory because it owns the game hook and memory mutation point.
- `official_plugins/sdk/runtime/src/mission/rank/control.rs`: subscribes to rank runtime signals and applies cap/threshold runtime config. It is core-compatible as a typed control adapter, though condition handling is still partial.
- `official_plugins/sdk/runtime/src/mission/rank/easy_cap.rs`: applies the easy S/S+ cap toggle from runtime config/control. It is runtime core because it patches runtime behavior, not script syntax.
- `official_plugins/sdk/runtime/src/mission/rank/threshold_patch.rs`: applies count threshold patch/override behavior. It is runtime core because it writes runtime rank thresholds.
- `official_plugins/sdk/runtime/src/mission/difficulty/control.rs`: subscribes to `DIFFICULTY_STAGE_RULE` and logs staged rules. It belongs in runtime core as the future difficulty mutation entry point, but currently it does not apply mutations.
- `official_plugins/sdk/runtime/src/mission/difficulty/ids.rs`: typed `DifficultyId`. This can stay core/data-adjacent because it is a stable runtime identifier, not Lua-specific syntax.
- `official_plugins/sdk/runtime/src/mission/difficulty/reward_row.rs`: reward-row inspection/format support used by difficulty probing. It is runtime/reverse support and should not become public script API.
- `official_plugins/sdk/runtime/src/mission/difficulty/tables.rs`: known difficulty table/layout helpers. It belongs to runtime data/reverse support while the offsets are still being validated.

## Lua frontend temporaire

- `official_plugins/sdk/runtime/src/runtime/lua_module.rs`: helper macro/registration layer for runtime Lua modules. Exact debt: `sdk.runtime` directly owns VM module registration; frontend target is a dedicated `sdk_lua` plugin or Lua adapter crate.
- `official_plugins/sdk/runtime/src/runtime/player/mod.rs`: registers `sdk.runtime.player` Lua module. Exact debt: player API is exposed as a Lua module from core runtime; frontend target is a Lua adapter over typed player/runtime state.
- `official_plugins/sdk/runtime/src/runtime/player/lua.rs`: Lua table/functions for player state. Exact debt: Lua syntax and `mlua` types live in runtime core; frontend target is `sdk_lua`.
- `official_plugins/sdk/runtime/src/runtime/fx/mods/lua_modules.rs`: FX Lua module registration. Exact debt: FX modder API is Lua-specific and registered by runtime; frontend target is a VM-specific FX adapter.
- `official_plugins/sdk/runtime/src/mission/rank/lua.rs`: builds Lua rank cap/staged-rule DSL and serializes `RankCapRule` JSON. Exact debt: Lua builders are load-time rule emitters, not runtime events; frontend target is Lua syntax over typed rank APIs.
- `official_plugins/sdk/runtime/src/mission/difficulty/lua.rs`: builds Lua difficulty staged-rule DSL and serializes `DifficultyRule`. Exact debt: the API creates rules at load time and depends on `mlua`; frontend target is Lua syntax over core difficulty mutation types.
- `official_plugins/sdk/runtime/src/rewards/lua.rs`: builds Lua rewards staged-rule DSL, parses Lua tables, emits `REWARD_STAGE_RULE`, and also reads mission reward observations via `struct_api`. Exact debt: it mixes Lua syntax, staged rule serialization, and data-service reads; frontend target is `sdk_lua` calling core reward event/mutation APIs plus `sdk_data` for mission data.
- `official_plugins/sdk/runtime/src/*/lua_tests.rs`: Lua frontend tests. Exact debt: useful compatibility coverage, but they should migrate with the Lua frontend when `sdk_lua` splits out.

## Data service

- `official_plugins/sdk/data/src/lib.rs`: `sdk_data` plugin initializes `struct_api` from `<game_root>/oppw4-data` and registers `std.character`. This is already data-service ownership.
- `official_plugins/sdk/data/src/character/mod.rs`: Lua-facing `std.character` install for character data. It belongs to data service content, but the `mlua` binding should eventually be a Lua frontend adapter over data APIs.
- `official_plugins/sdk/data/src/character/handles.rs`: character handle lookup/normalization support for `std.character`. It belongs to `sdk_data` because it serves curated character data.
- `official_plugins/sdk/data/src/character/extensions.rs`: character extension helpers for Lua/data presentation. It belongs to `sdk_data`, with the same future Lua-adapter split caveat.
- `official_plugins/sdk/runtime/src/rewards/lua.rs`: uses `struct_api::missions::all/find/find_by_id` for reward observations. That mission/reward lookup belongs to `sdk_data`; runtime core should receive typed data through a data service/provider instead of importing data reads in the Lua rewards module.
- `official_plugins/sdk/runtime/src/mission/difficulty/ids.rs`: can stay shared if treated as a runtime identifier, but curated difficulty names/labels should come from `sdk_data` once names become moddable.
- `crates/character-bank` / package `struct-api`: documented in `CRATES-CLEANUP-AUDIT.md` as broader OPPW4 data ownership, not generic core. It should either become the backed data service or be renamed/split so runtime does not own mission/character/reward catalogs.

## Reverse probes read-only

- `official_plugins/sdk/runtime/src/mission/difficulty/state_probe.rs`: difficulty state snapshot probe, enabled by `[difficulty_probe]`. Risk known: default config enables it and it can dump reward rows; keep read-only until difficulty mutation targets are validated.
- `official_plugins/sdk/runtime/src/mission/rank/threshold_probe.rs`: rank threshold table snapshot probe, enabled by `[rank_threshold_probe]`. Risk known: old static rows were misleading for runtime result/global rank behavior; keep as evidence collection only.
- `official_plugins/sdk/runtime/src/mission/rank/helper_probe.rs`: rank helper/callsite probe, enabled by `[rank_helper_probe]` with `count_enabled`, `merge_enabled`, and `callsite_enabled`. Risk known: it can detour hot rank helpers and should remain diagnostic unless explicitly applying `RankRuntimeConfig` threshold overrides.
- `official_plugins/sdk/runtime/src/mission/result/player_probe.rs`: player result snapshot probe, enabled by `[player_result_probe]`. Risk known: reads result/global/save structures and should not mutate save/result memory.
- `official_plugins/sdk/runtime/src/mission/result/memory_probe.rs`: result memory diff probe, enabled by `[result_probe]`. Risk known: scans result memory and logs changed words; keep read-only to avoid corrupting result screen state.
- `official_plugins/sdk/runtime/src/mission/result/state_hook.rs`: result state hook, enabled by `[result_state_probe]`. Risk known: detours result-state flow and emits snapshots; any mutation should go through a later typed event path.
- `official_plugins/sdk/runtime/src/rewards/commit_hook.rs`: reward commit hook, enabled by `[reward_probe]`. Risk known: currently both probes and mutates berry slot through staged rules; Step 3 should split read-only snapshot emission from explicit reward mutations.
- `official_plugins/sdk/runtime/src/rewards/item_hook.rs`: item reward hook, enabled by `[item_reward_probe]`. Risk known: reads triplet item reward entries after original call; keep read-only until souls/medals mutation layout is proven.
- `official_plugins/sdk/runtime/src/reverse/fixed_data_probe.rs`: fixed data probe, enabled by `[fixed_data_probe]`. Risk known: reverse-only pointer/table discovery; no gameplay mutation should be added here.
- `official_plugins/sdk/runtime/src/reverse/entity_counter_probe.rs`: entity counter probe, enabled by `[entity_counter_probe]`. Risk known: broad memory scan around game state; read-only diagnostics only.
- `official_plugins/sdk/runtime/src/reverse/spawn_scaling_probe.rs`: spawn scaling probe, enabled by `[spawn_scaling_probe]`. Risk known: it observes spawn/difficulty candidates and should not become the spawn-rate mutation implementation directly.
- `official_plugins/sdk/runtime/src/reverse/damage_formula_probe.rs`: actor stat init probe, enabled by `[damage_formula_probe]`. Risk known: detours actor stat initialization; use it to identify difficulty attack/defense fields before any write path.
- `official_plugins/sdk/runtime/src/reverse/value_scan_probe.rs`: value scan probe, enabled by `[value_probe]`. Risk known: arbitrary value scanning can create noisy false positives; keep read-only and config-gated.
- `official_plugins/sdk/runtime/src/runtime/fx/hooks/diagnostics.rs`: FX character probe diagnostics behind FX debug config. Risk known: diagnostic field probing only; keep separate from the language-independent runtime event bus unless FX gets its own typed events.

## Decisions pour Step 1

1. Step 1.2 should define core reward event/mutation types outside `rewards/lua.rs`; Lua builders must parse into those types instead of owning the model.
2. Keep `commit_hook.rs` as the reward mutation application point, but split snapshot/probe logging from explicit mutation application in Step 3.
3. Do not move `sdk_data` back into runtime; runtime should depend on typed providers/services for missions, rewards, characters, and future difficulty labels.
4. Treat all files under `runtime/src/reverse/*` and result/rank/difficulty probes as read-only evidence collectors unless a later step explicitly promotes a validated field into a typed mutation.
5. `runtime/mod.rs` can keep orchestration for now, but VM registration should become adapter registration once `sdk_lua` exists.
6. No file size currently blocks Step 1; the main blocker is mixed ownership, not oversized modules.
