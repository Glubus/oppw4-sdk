# Step 5.1 Game Test

Date: 2026-05-26

## Build And Package

Commands run from the SDK repository:

```powershell
cargo test
cargo build --release
.\tools\package-sdk.ps1 -NoLoader
Copy-Item -Path .\dist\oppw4-sdk\plugins\* -Destination D:\SteamLibrary\steamapps\common\OPPW4\plugins -Recurse -Force
```

Results:

- `cargo test`: pass.
- `cargo build --release`: pass.
- `package-sdk.ps1 -NoLoader`: pass.
- Plugin copy to `D:\SteamLibrary\steamapps\common\OPPW4\plugins`: pass.

The package was generated at:

```text
C:\Users\Osef\Documents\Codex\oppw4-sdk-split\oppw4-sdk\dist\oppw4-sdk
```

`-NoLoader` was used. `dinput8.dll` was not copied or modified.

## Installed Hypothesis

The installed runtime should be launchable and should expose the new rewards
runtime-core path at the result screen.

Expected log chain during reward commit:

```text
reward_event ...
reward_mutations ...
reward_apply ...
```

Expected signal:

```text
sdk.runtime.rewards.event
```

No Berry multiplier test rule was intentionally enabled in this step. The
default expected gameplay behavior is vanilla rewards plus better event logs.

## What To Test In Game

1. Launch OPPW4 normally.
2. Run any mission that reaches the result screen.
3. Confirm the game does not crash before or during result rewards.
4. After closing the game or returning to menu, inspect the latest runtime log:

```text
D:\SteamLibrary\steamapps\common\OPPW4\plugins\sdk\logs\sdk_runtime\*.log
```

Useful strings to search:

```text
reward_event
reward_mutations
reward_apply
sdk.runtime.rewards.event
```

## Known Warnings

The release build currently warns that the Lua `rewards.on_commit` dispatch
helpers are not used by the runtime hook yet. This is expected at the end of
Step 5: the Rust staged-rule path is connected to the reward bus, while the Lua
frontend exists and is tested but still needs a later integration pass to attach
runtime Lua callbacks to the bus.

## Status

Ready for a live launch/result-screen test.

