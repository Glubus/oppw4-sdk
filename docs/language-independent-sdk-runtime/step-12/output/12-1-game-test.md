# Step 12.1 - Packaging / Game Test

Status: packaged and installed again after live runtime bus/player/rank/difficulty event work; pending live game test.

Expected install flow:

```powershell
cargo test
cargo build --release
.\tools\package-sdk.ps1 -NoLoader
Copy-Item -Path .\dist\oppw4-sdk\plugins\* -Destination D:\SteamLibrary\steamapps\common\OPPW4\plugins -Recurse -Force
```

Actual install flow run:

```powershell
cargo test
cargo build --release
.\tools\package-sdk.ps1 -NoLoader
Copy-Item -Path .\dist\oppw4-sdk\plugins\* -Destination D:\SteamLibrary\steamapps\common\OPPW4\plugins -Recurse -Force
```

Verification:

- `cargo test`: passed.
- `cargo build --release`: passed.
- `package-sdk.ps1 -NoLoader`: wrote `dist\oppw4-sdk`.
- Installed only `plugins` to the game directory.
- `dinput8.dll` was not copied or replaced.
- Legacy command signal names are absent from runtime/sdk-api Rust sources.
- `export_runtime_snapshots.py --dry-run` ran with the bundled Python runtime against existing sdk_runtime logs.

2026-05-26 live-event-runtime install:

- `cargo test -p oppw4-sdk-runtime-plugin mission::difficulty -- --nocapture`: passed.
- `cargo test -p oppw4-sdk-runtime-plugin runtime::core -- --nocapture`: passed.
- `cargo test`: passed.
- `cargo build --release`: passed.
- `.\tools\package-sdk.ps1 -NoLoader`: passed.
- `Copy-Item -Path .\dist\oppw4-sdk\plugins\* -Destination D:\SteamLibrary\steamapps\common\OPPW4\plugins -Recurse -Force`: passed.
- `dinput8.dll` was not touched.

2026-05-26 persistent-Lua/event-only install:

- `sdk.runtime.rewards` exposes `on_commit` only.
- `sdk.runtime.ranks` exposes `on_result` only.
- `sdk.runtime.difficulty` exposes `on_apply` only.
- `sdk.runtime.player` exposes `on_change` only.
- Live Lua callbacks now register persistent runtime bus handlers; tests verify handlers still fire after the local Lua loader handle is dropped.
- `cargo test -p oppw4-sdk-runtime-plugin lua_tests -- --nocapture`: passed.
- `cargo test`: passed.
- `cargo build --release`: passed.
- `.\tools\package-sdk.ps1 -NoLoader`: passed.
- `Copy-Item -Path .\dist\oppw4-sdk\plugins\* -Destination D:\SteamLibrary\steamapps\common\OPPW4\plugins -Recurse -Force`: passed.
- `dinput8.dll` was not touched.

2026-05-26 live Lua probe installed:

- Installed `D:\SteamLibrary\steamapps\common\OPPW4\mods\runtime_event_probe\mod.toml`.
- Installed `D:\SteamLibrary\steamapps\common\OPPW4\mods\runtime_event_probe\mod.lua`.
- Probe uses only event APIs: `sdk.runtime.rewards.on_commit` and `sdk.runtime.difficulty.on_apply`.
- Probe emits no intended gameplay change: berry multiplier is `1`, combat pressure multiplier is `1.0`.
- Expected log proof: `reward_event` should report one callback-produced mutation at result commit, and `difficulty_event` should report one accepted combat-pressure mutation with writer still pending.

Live test target:

- Launch game without replacing `dinput8.dll`.
- Start one mission on Normal or Super Hard.
- Confirm `sdk_runtime` logs contain `difficulty_event mission=... mode=... difficulty=...`.
- Reach a result screen.
- Confirm `sdk_runtime` logs contain observation-only `reward_event` and `rank_event`.
- Confirm active character startup still logs player/core snapshot changes.
- If a Lua mod registers `sdk.runtime.*.on_*`, confirm the callback-produced mutations appear in the matching event log.
- Confirm no `stage_rule` or difficulty/rank signal-command path appears.

Result: pending live user test.
