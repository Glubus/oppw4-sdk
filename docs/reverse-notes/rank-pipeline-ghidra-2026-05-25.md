# Rank Pipeline Ghidra Notes - 2026-05-25

## Current conclusion

The rank problem should be followed through the result pipeline, not by blindly
editing the first row that contains plausible thresholds.

`FUN_14132b570` builds the result screen state. In the normal result path it
computes two visible sub-ranks:

- `FUN_1412dd9e0(row, value: f32)` for lower-is-better values, likely clear
  time;
- `FUN_1412dd950(row, value: u32, divisor: f32)` for higher-is-better values,
  likely kill/count style values.

For the normal path, the helper row is built from:

```text
rank_row_id = *(u16 *)(global + 0x1d750)
row = *(usize *)(*(usize *)(DAT_141eba738 + 0x18) + 0x28)
    + 0x4c
    + rank_row_id * 0xdc
```

The row seen in the latest runtime logs for mission 35 was row id `12`.
Its count slot contained:

```text
[2000, 1500, 1200, 1050, 750]
```

That means the old `60000,60000,48000` assumption is not the active visible
count helper row for that result screen. Any patch expecting that prefix will
correctly skip.

## Global rank and reward path

`FUN_14132aae0` is the cleaner global battle-rank calculator. It receives the
mission/player values, calls the same helper families, then applies mode and
Easy difficulty caps before writing persistent/result values.

Important observed behavior:

- `iVar7` is the time-like sub-rank.
- `iVar8` is the count/score-like sub-rank.
- `FUN_1412dd790(iVar7, iVar8)` merges the two normal sub-ranks.
- Easy difficulty (`global + 0x1d756 == 0`) downgrades rank `4` and `5` to `3`
  in this global path.
- The berry reward hook receives this global rank as `param4`.
- The item/medal reward hook receives the same global rank as `context`.

So a future `rank_director` / `reward_director` must control the global rank
path, not only result-screen display fields.

## Merge helper details

`FUN_1412dd790(left_rank, right_rank)` is not a simple `min` or `max`.
Ghidra shows it maps each input rank through a small rank-score index table at
`DAT_141953390`, reads score values from:

```text
fixed_owner+0x20+0x1048 + score_index*4
```

then scales both scores by `0.001`, adds them, and compares the sum against a
grade target index table ending at `DAT_1419533bc`.

The loop starts at grade candidate `5` and walks downward. In rough form:

```text
left_score  = scaled_score(rank_score_index[left_rank])
right_score = scaled_score(rank_score_index[right_rank])
combined    = left_score + right_score

for grade = 5 down to 0:
  target = scaled_score(grade_target_index[grade])
  if target <= combined:
    return grade

return 0
```

This makes the global rank pipeline score-weighted. A pair of visible sub-ranks
can produce a higher or lower global grade depending on the fixed score table,
so rank tuning must eventually expose merge policy separately from raw
time/count thresholds.

Runtime follow-up added:

```toml
[rank_helper_probe]
enabled = false
merge_enabled = false
```

This installs a read-only hook on `FUN_1412dd790` and logs:

```text
rank_helper_probe kind=merge caller=... left=... right=... result=...
  left_score=... right_score=... combined_score=... grade_targets=[...]
```

It stays disabled by default. The first result-screen run with helper hooks
enabled reached the time helper once, then crashed later in the result flow
before any merge log appeared. Treat callee-entry helper hooks as unsafe for
normal testing. Prefer fixed table snapshots or post-call/callsite probes for
future correlation before promoting `rank.map_global_rank(...)` or any broader
rank director API.

## Unchecked result-screen branches

The latest static pass over `FUN_14132b570` found more result state after the
two visible sub-ranks. The rough result struct mapping is now:

```text
0x28 / +0x28c / +0x330  active result row id from global+0x1d9b0
+0x2c / +0x290 / +0x334 count/score raw value
+0x30 / +0x294 / +0x338 time/raw float value
+0x34 / +0x298          count/score visible rank
+0x38 / +0x29c          time visible rank
+0x3c / +0x2a0          global/merged rank copy
+0x40 / +0x2a8          rank row id from global+0x1d750
+0x60 / +0x328          downgrade/cap-applied flag
```

The mode `4` branch copies the normal visible state into the `+0x28c` block,
then computes extra aggregate state:

- `FUN_14132d7a0(rank_row_id, lambda)` is called five times while filling
  `+0x2ec..+0x300`.
- `FUN_14132d7a0` changes behavior based on `global+0x1d754`.
- When `(global+0x1d754 - 1) < 2`, it sums `FUN_14132d9a0`, which iterates four
  player/slot rows from `global+0x1d9b0 + slot*0x50`.
- Otherwise it checks how many `0x1d9b0` rows look active, then either calls the
  passed lambda directly or sums `FUN_14132d910`, which iterates two slots.

That means `global+0x1d9b0` is not just the visible row id. It also controls
aggregation rules for mode/result variants. Any rank API that only patches the
normal `fixed+0x4c+row*0xdc` helper row will miss these result branches.

Mode `4` also has a separate aggregate grade at `+0x2a4`: it counts four
global fields from the active player block (`+0xf3c`, `+0xf40`, `+0xf44`,
`+0xf48`, `+0xf4c`) and grades that count through the special table
`fixed+0x46c88+context*0x58` using selector `2`. On Easy, grade `4` is capped to
`3` and sets `+0x328`.

Mode `5` has another unlabelled fork. If `FUN_1412fa4c0(rank_row_id) == 4`, the
normal result fields are copied into `+0x384..+0x3a4`; otherwise the function
fills `+0x348..+0x380` from player block fields around `+0xf6c..+0xf74`,
`FUN_1412ee2c0(...)`, and fixed row flags at `fixed+0xb6+row*0xdc`.

Open points from this pass:

- label `global+0x1d754` beyond "reward/material mode"; it directly changes
  `FUN_14132d7a0` aggregation.
- label `global+0x1d762`; when nonzero, special context is forced to `0`,
  otherwise context comes from save/runtime `+0xff60`.
- label the active-player block fields `+0xf3c..+0xf74` used by mode `4`/`5`.
- label the `+0x2a4`, `+0x330..+0x344`, and `+0x348..+0x3a4` result-state
  regions before exposing a public rank/result director.
- replace fragile callee hooks with callsite probes around:
  - `14132b8ee` after `FUN_1412dd9e0`;
  - `14132b917` after `FUN_1412dd950`;
  - `14132b987` / `14132b995` around `FUN_1412dd090`;
  - `14132bc75` after `FUN_14132d7a0`.

Runtime follow-up implemented:

```toml
[rank_helper_probe]
callsite_enabled = false
```

When explicitly enabled, this installs tiny post-call caves at:

```text
14132b917 result count rank write
14132b995 special cap flag write
14132bc7a mode 4 aggregate write after FUN_14132d7a0
```

Each cave verifies the exact original bytes before patching, replays those
bytes, logs the already-written result-state fields, restores volatile
registers, and jumps back. This is meant for result-screen correlation only and
keeps the previous helper-entry hooks off by default.

The first attempted time-rank callsite at `14132b8ee` was removed. It required
copying `48 8b 05 ...`, a RIP-relative load, into the cave. Without relocating
that `disp32`, the copied instruction points at the wrong address and crashes
at cave entry. Do not re-add time-rank callsite logging until the cave builder
can relocate RIP-relative instructions or a safer patch site is chosen.

The runtime installer now refuses callsite probes whose copied original bytes
contain a RIP-relative instruction, and tests cover the removed `14132b8ee`
pattern.

Latest successful result-screen run:

```text
mission=35 difficulty=1 result_mode=1 reward_mode=3
active rank row=12
count raw=1307 -> count rank=4(S)
time raw bits=0x430b7950 -> time rank=5(S+)
reward/global rank param4=4(S)
```

Follow-up run on the same mission/difficulty:

```text
count raw=1534 -> count rank=5(S+)
time raw bits=0x4313baed -> time raw=147.730 -> time rank=5(S+)
reward/global rank param4=5(S+)
```

This confirms the normal reward/global rank follows the merged visible
sub-ranks for this path: `S + S+` produced reward rank `S`, while `S+ + S+`
produced reward rank `S+`.

The old threshold patch did not alter row `12` during this run:

```text
thresholds=[2000,1500,1200,1050,750]
expected_prefix=[60000,60000,48000]
patched=0 skipped=1
```

So the active fixed count thresholds for mission 35 are the small row-12 values,
not the earlier `60000` prefix. Do not use the old prefix for this mission.

Next installed hypothesis:

```toml
[rank_runtime]
shift_count_thresholds = true
shift_count_rank_row_ids = [12]
shift_count_source_prefix = [2000, 1500, 1200]
count_threshold_override = [2000, 1200, 1200, 1050, 750]
```

Expected result: if the helper uses the second threshold as the `S+` boundary
for this selector, a result around the previous `1307` sample should move from
count `S` to count `S+`, and reward/global rank should also become `S+` as long
as time remains `S+`.

Result: the fixed helper row patch applied, but count `1285` still produced
`S`. The next installed runtime adds a wrapper at `14132b912`, the count helper
call itself. It logs the exact `RCX` helper-row pointer, count raw value, slot
selectors, and thresholds immediately before calling `FUN_1412dd950`, then
calls the original helper and returns to `14132b917`.

First wrapper build was bad: it overwrote the argument registers before logging
and did not zero-extend the helper's `u8` return. The bad run showed impossible
values such as `helper_row=0x5` and `count_rank=3836655895`; ignore that data.
Installed follow-up fixes the wrapper by preserving the original `RCX`/`EDX`
before calling the logger and emitting `movzx eax, al` after the helper call.

Super Hard follow-up (`mission=35`, `difficulty=3`) changed the actual result
count helper row. The active fixed row `12` patch still applied:

```text
row_id=12 old=[2000,1500,1200,1050,750] new=[2000,1200,1200,1050,750]
```

but the result-screen count helper call used a different helper row:

```text
count_raw=1474 thresholds=[1500,1000,800,700,500]
```

That means the visible count-rank calculation is not reading the row-12
thresholds in this Super Hard path. The same run also produced
`reward_probe param4=4(S)` / `item_reward_probe context=4`, so the reward/global
rank remained `S`.

The apparent visible `S+` from that run is not trustworthy because the wrapper
still jumped back through `RAX` after `movzx eax, al`, overwriting the helper
return. The installed follow-up now jumps back through `R11`, preserving `EAX`
for both the pre-call wrapper and post-call probes.

Next expected clean log:

```text
rank_callsite_probe kind=result_count_call ... thresholds=[1500,1000,800,700,500]
rank_callsite_probe kind=result_count_post_call ... count_rank=<real 0..5 value>
```

If `count_raw < 1500` stays `S` after this fix, the next target is the reward
path, not the result-screen display path. Ghidra export points at
`FUN_14132aae0`: count helper call `14132ad34`, then merge helper calls
`14132adbc` / `14132aeb5`.

Confirmed runtime threshold override test:

```text
count_threshold_override=[2000,1500,800,700,500]
global_count_call count_raw=1378 thresholds=[1500,1000,800,700,500]
  patched_thresholds=[2000,1500,800,700,500]
result_count_call count_raw=1378 thresholds=[2000,1500,800,700,500]
result_count_post_call count_rank=3(A) time_rank=5(S+)
reward_probe param4=4(S)
```

This proves the real helper row override works for both global and result
count calls. The reward/global rank can still remain `S` because
`FUN_1412dd790` merges `A + S+` into `S`; threshold edits alone do not directly
force the final rank. A full rank API needs separate controls for helper
thresholds and merge policy.

Only `14132b917` fired on that normal result path. `14132b995` and `14132bc7a`
were installed but not reached, so they likely belong to special cap / mode `4`
branches. The `RBX` pointer at `14132b917` is the inner rank block
(`outer_result_state + 0x28`), not the outer result-state base. Runtime callsite
logs now label it as `rank_block` and dump fields relative to that block.

The focused export script is:

```text
tools/ghidra/ExportRankCallsiteTargets.java
```

It writes:

```text
docs/reverse-notes/rank-callsite-targets-2026-05-25.txt
```

## SDK ownership

This should be split by service responsibility:

- `sdk_runtime`: runtime game state, active character, mission status, result
  rank/reward probes, FX runtime/Lua API, and future rank/difficulty services.
- `sdk_rdb`: RDB virtual file/patch service plus the skin/model/portrait Lua API.
- `sdk_linkdata`: LinkData virtual file/patch service.
- `moveset_patcher`: official feature plugin built on `sdk_linkdata`.

`fx_director` and `skin_patcher` are no longer separate runtime plugins. Their
old source directories are internal crates consumed by the SDK service plugins:

- `sdk_runtime` owns `require("sdk.runtime.fx")` through its internal `runtime::fx` module.
- `sdk_rdb` depends on `sdk-rdb-patcher` and owns `require("sdk.rdb.patcher")`.

The only current gameplay plugin that remains external is `moveset_patcher`.

## Next rank work

1. Keep probes read-only by default.
2. Add a focused Ghidra export for:
   - `FUN_14132aae0`;
   - `FUN_1412dd090`;
   - `FUN_1412dd950`;
   - `FUN_1412dd9e0`.
3. Add a focused Ghidra export for `FUN_14132d7a0`, `FUN_14132d9a0`,
   `FUN_14132d910`, and `FUN_14132ca10`.
4. Validate `FUN_1412dd790` merge scores with safer callsite probes or table
   snapshots, not result-screen callee hooks.
5. Label the exact meaning of rank values `3`, `4`, and `5` in the helper and
   global paths.
6. Only then expose public APIs such as:
   - `rank.set_easy_s_rankable(true)`;
   - `rank.override_thresholds(...)`;
   - `rank.map_global_rank(...)`.
