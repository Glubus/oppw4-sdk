# Live Event Runtime Execution Plan

Goal: finish moving gameplay decisions from per-feature/ad-hoc paths into one live typed runtime event bus. Signals remain debug/overlay/telemetry only. No legacy compatibility is required.

Current baseline:

- Rewards dispatches `RewardCommitEvent` from the live reward hook and applies `RewardMutation::MultiplyBerry`.
- Rank, difficulty, and player have typed core events/mutations and Lua callback tests, but are not dispatched from live game hooks yet.
- Lua callbacks are testable, but Lua mod states are currently short-lived; live callbacks need a persistent registration model before they can affect result-screen gameplay.
- Legacy command signals and staged rule runtime files were removed.

Hard rules:

- Do not replace or copy `D:\SteamLibrary\steamapps\common\OPPW4\dinput8.dll`.
- Install by `.\tools\package-sdk.ps1 -NoLoader`, then copy `dist\oppw4-sdk\plugins\*` only.
- Every live step must leave the game launchable and one new hypothesis testable.
- Before fixing a crash, read latest `plugins\sdk\logs\sdk_runtime\*.log` and crash logs.

## Step 13 - Single Runtime Bus

Purpose: remove the rewards-only bus and make one process-wide event bus for all runtime features.

Files:

- Create: `official_plugins/sdk/runtime/src/runtime/core/live_bus.rs`
- Modify: `official_plugins/sdk/runtime/src/runtime/core/mod.rs`
- Modify: `official_plugins/sdk/runtime/src/rewards/mod.rs`
- Modify: `official_plugins/sdk/runtime/src/rewards/commit_hook.rs`

Work:

1. Move `REWARD_EVENT_BUS` ownership into `runtime::core::live_bus`.
2. Expose narrow internal functions:
   - `register_runtime_handler(id, handler)`
   - `dispatch_runtime_event(event)`
   - `reset_runtime_handlers_for_tests()`
3. Update rewards to call `dispatch_runtime_event(RuntimeEvent::RewardCommit(event))`.
4. Keep `rewards::register_reward_handler` as an internal wrapper only if tests still need it; otherwise migrate tests to the global bus.

Tests:

```powershell
cargo test -p oppw4-sdk-runtime-plugin runtime::core rewards::commit_hook -- --nocapture
cargo test
```

Game hypothesis:

- Result screen still logs `reward_event`.
- No behavior change expected.

## Step 14 - Player Core Live Snapshot

Purpose: make player core state real, not just Lua helper/front-end syntax.

Files:

- Modify: `official_plugins/sdk/runtime/src/game/active_character/state.rs`
- Modify: `official_plugins/sdk/runtime/src/game/active_character/probe.rs`
- Modify: `official_plugins/sdk/runtime/src/runtime/core/player.rs`
- Modify: `official_plugins/sdk/runtime/src/runtime/core/events.rs`

Work:

1. Convert active character runtime id/alt id into `PlayerSnapshot`.
2. Emit `RuntimeEvent::PlayerChange(PlayerChangeEvent)` when active character sequence changes.
3. Add an in-memory latest-player snapshot:
   - `player::latest_snapshot() -> PlayerSnapshot`
   - `player::update_snapshot(snapshot)`
4. Do not expose Lua as source of truth. `sdk.runtime.player` only reads or builds conditions over the latest core snapshot.

Tests:

```powershell
cargo test -p oppw4-sdk-runtime-plugin game::active_character runtime::core::player runtime::player -- --nocapture
cargo test
```

Game hypothesis:

- Launch into mission and change/start character.
- Logs should show active character changes as before.
- No gameplay mutation yet.

## Step 15 - Rank Result Event Hook

Purpose: publish result-screen rank context as `RankResultEvent`.

Files:

- Modify: `official_plugins/sdk/runtime/src/mission/result/state_hook.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/result/state_hook/snapshot.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/rank/mod.rs`
- Modify: `official_plugins/sdk/runtime/src/runtime/core/rank.rs`

Work:

1. Build `RankResultEvent` from result-state/rank data already read by `state_hook`.
2. Attach:
   - rank value
   - mission id
   - difficulty snapshot if available
   - latest player snapshot from Step 14
3. Dispatch via global runtime bus.
4. Keep `RANK_SNAPSHOT` and `RANK_HELPER_CALL` read-only.
5. Do not apply `RankMutation` yet in this step.

Tests:

```powershell
cargo test -p oppw4-sdk-runtime-plugin mission::result mission::rank runtime::core::rank -- --nocapture
cargo test
```

Game hypothesis:

- Reach result screen.
- Expected: existing result/rank logs plus a new compact `rank_event` log.
- No rank behavior change yet.

## Step 16 - Rank Mutation Application

Purpose: apply rank mutations directly from core, without signal commands.

Files:

- Create: `official_plugins/sdk/runtime/src/mission/rank/director.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/rank/mod.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/rank/easy_cap.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/rank/threshold_patch.rs`

Work:

1. Register a rank handler on the global bus.
2. Support only confirmed mutations:
   - `RankMutation::SetCap { rank: S | SPlus, enabled }` -> `easy_cap::set_easy_s_rankable`
   - `RankMutation::ShiftCountThresholds` -> existing threshold patch
   - `RankMutation::OverrideCountThresholds` -> existing threshold override
3. Gate unsupported rank slots with a log, not a crash.
4. Add a config-only test handler first; do not make Lua live yet.

Tests:

```powershell
cargo test -p oppw4-sdk-runtime-plugin mission::rank runtime::core -- --nocapture
cargo test
```

Game hypothesis:

- Enable one test handler that sets S/S+ cap only.
- Reach result screen on Easy/Normal.
- Expected: rank logs show handler fired; no crash.

## Step 17 - Difficulty Apply Event

Purpose: publish difficulty apply context before any gameplay mutation.

Files:

- Modify: `official_plugins/sdk/runtime/src/mission/difficulty/state_probe.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/difficulty/tables.rs`
- Create: `official_plugins/sdk/runtime/src/mission/difficulty/director.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/difficulty/mod.rs`

Work:

1. Use the existing difficulty snapshot/probe data to build `DifficultyApplyEvent`.
2. Dispatch event periodically or on changed snapshot only; avoid per-frame spam.
3. Add logs:
   - `difficulty_event mission=... mode=... difficulty=...`
4. No mutation in this step.

Tests:

```powershell
cargo test -p oppw4-sdk-runtime-plugin mission::difficulty runtime::core::difficulty -- --nocapture
cargo test
```

Game hypothesis:

- Start mission on Normal and Super Hard.
- Expected: `difficulty_event` logs show mode/difficulty.
- No gameplay behavior change yet.

## Step 18 - Difficulty Mutation MVP

Purpose: apply only confirmed difficulty mutations.

Files:

- Modify: `official_plugins/sdk/runtime/src/mission/difficulty/director.rs`
- Modify: `official_plugins/sdk/runtime/src/reverse/spawn_scaling_probe/snapshot.rs` if shared readers are needed
- Modify: `official_plugins/sdk/runtime/src/mission/difficulty/tables.rs`

Work:

1. Support:
   - `DifficultyMutation::CombatPressure`
   - `DifficultyMutation::KnownTable`
2. Explicitly reject/log:
   - hp
   - attack
   - defense
   - unknown raw writes
3. First live hypothesis should be a tiny/loggable known-table change, not a huge gameplay change.
4. Add saturation and bounds checks before writing.

Tests:

```powershell
cargo test -p oppw4-sdk-runtime-plugin mission::difficulty reverse::spawn_scaling_probe -- --nocapture
cargo test
```

Game hypothesis:

- Start mission on Super Hard.
- Expected: difficulty mutation log with before/after for one known table.
- If crash: stop, read logs/crash logs before touching code.

## Step 19 - Persistent Lua Runtime Registration

Purpose: make `sdk.runtime.*.on_*` callbacks affect live events.

Files:

- Modify: `crates/lua-runtime/src/runtime/runner.rs`
- Modify: `crates/sdk-core/src/runtime/lua/state.rs`
- Modify: `official_plugins/sdk/runtime/src/rewards/lua.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/rank/lua.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/difficulty/lua.rs`
- Modify: `official_plugins/sdk/runtime/src/runtime/player/lua.rs`

Work:

1. Do not store raw short-lived Lua callbacks in a global Rust bus.
2. Add a persistent runtime registration object per Lua mod:
   - callback metadata
   - callback registry key tied to the owning Lua state
   - owner mod id
3. Decide execution model:
   - preferred: keep Lua state alive for runtime mods using event callbacks;
   - fallback: compile callback declarations into serializable core rules at mod load.
4. Route callback-produced mutations into global runtime bus.
5. Add error isolation per mod callback.

Tests:

```powershell
cargo test -p lua-api
cargo test -p plugin-host runtime::lua
cargo test -p oppw4-sdk-runtime-plugin lua_tests
cargo test
```

Game hypothesis:

- Install a tiny Lua mod registering `rewards.on_commit`.
- Reach result screen.
- Expected: callback fires and logs one mutation.

## Step 20 - Remove Old Lua JSON Builders

Purpose: final cleanup after live callbacks work.

Files:

- Modify: `official_plugins/sdk/runtime/src/rewards/lua.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/rank/lua.rs`
- Modify: `official_plugins/sdk/runtime/src/mission/difficulty/lua.rs`
- Modify tests in each `lua_tests.rs`
- Update docs under `docs/language-independent-sdk-runtime/`

Work:

1. Remove load-time JSON DSL:
   - `rewards.berry:multiply(...)` returning JSON
   - `rewards.souls:*` stubs
   - `ranks.slot(...):enable()`
   - `difficulty.level(...):...`
2. Keep only event-based APIs:
   - `sdk.runtime.rewards.on_commit`
   - `sdk.runtime.ranks.on_result`
   - `sdk.runtime.difficulty.on_apply`
   - `sdk.runtime.player.on_change`
3. Keep mission data reads only if they belong in data frontend; otherwise move to `sdk.data`.

Tests:

```powershell
cargo test -p oppw4-sdk-runtime-plugin lua_tests -- --nocapture
cargo test
```

Game hypothesis:

- Existing event callback test mod still works.
- Old staged/JSON DSL no longer loads, intentionally.

## Step 21 - Data Dumper Integration

Purpose: make runtime evidence export repeatable.

Files:

- Modify: `oppw4-data/scripts/export_runtime_snapshots.py`
- Modify: `oppw4-data/README.md`
- Modify schemas only if new structured output is added.

Work:

1. Add filters:
   - mission id
   - log date
   - event kind
2. Add optional structured output for:
   - mission
   - mode
   - difficulty
   - rank rows
   - reward rows
   - fixed ids
3. Keep default mode as dry-run unless `--write` is passed.

Tests:

```powershell
& 'C:\Users\Osef\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' .\oppw4-data\scripts\export_runtime_snapshots.py D:\SteamLibrary\steamapps\common\OPPW4\plugins\sdk\logs\sdk_runtime --dry-run
cargo test
```

## Step 22 - Final Packaging / Live Matrix

Purpose: validate the whole runtime after each live feature is connected.

Commands:

```powershell
cargo test
cargo build --release
.\tools\package-sdk.ps1 -NoLoader
Copy-Item -Path .\dist\oppw4-sdk\plugins\* -Destination D:\SteamLibrary\steamapps\common\OPPW4\plugins -Recurse -Force
```

Live test matrix:

- Boot game.
- Start one mission on Normal.
- Start one mission on Super Hard.
- Reach result screen.
- Confirm:
  - reward event logs
  - player change snapshots
  - rank result event logs
  - difficulty event logs
  - no legacy signal-command logs
  - no crash

Output doc:

- Update `docs/language-independent-sdk-runtime/step-12/output/12-1-game-test.md` after each live run.

## Execution Order

1. Step 13 single runtime bus.
2. Step 14 player live snapshot.
3. Step 15 rank event only.
4. Step 16 rank mutation application.
5. Step 17 difficulty event only.
6. Step 18 difficulty mutation MVP.
7. Step 19 persistent Lua runtime registration.
8. Step 20 remove old Lua JSON builders.
9. Step 21 improve data dumper.
10. Step 22 package/install/live matrix.

## Stop Conditions

Stop and inspect logs before changing code if:

- The game does not boot.
- The game crashes before or at result screen.
- A mutation log appears without a matching event log.
- A legacy command signal name appears in runtime logs.
- Difficulty writes touch hp/attack/defense paths before reverse is confirmed.
