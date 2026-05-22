# Difficulty / Reward Reverse Notes - 2026-05-20

## Context

Goal: prepare a future `difficulty_director` / `reward_director` plugin that can add or scale difficulties and end-of-mission rewards without only patching UI text.

Source used:

- `OPPW4.exe` from `D:\SteamLibrary\steamapps\common\OPPW4\OPPW4.exe`
- Ghidra project: `oppw4-ghidra/oppw4_game`
- CT reference: `C:\Users\Osef\Downloads\OPPW4 (2).CT`

Generated exports:

- `oppw4-ghidra/game_difficulty_reward_targets.txt`
- `oppw4-ghidra/game_difficulty_rtti.txt`
- `oppw4-ghidra/game_difficulty_vtables.txt`
- `oppw4-ghidra/game_difficulty_readers.txt`
- `oppw4-ghidra/game_difficulty_impact_targets.txt`

## High Confidence Findings

### The game has explicit difficulty logic

RTTI and vtable names exist:

- `CScCondGameDifficulty::vftable` at `0x14195a370`
- `CDataFixedReward::vftable` at `0x141953408`
- `CGameStateBattleResult::vftable` at `0x1419553f8`
- `CUIBattleResult::vftable` at `0x141974800`
- `CUIBattleResultEnjoyReward::vftable` at `0x141973210`

String/data labels:

- `tb_level_of_difficulty` at `0x1416d0cf0`
- many `tb_reward`, `tb_level_reward_*`, `mai_reward_coin_gold_*`

This strongly suggests both difficulty UI and reward data are real data-driven systems, not just hardcoded display code.

## Difficulty Ids

Vanilla difficulty ids are almost certainly:

- `0`: Easy / Facile
- `1`: Normal / Normale
- `2`: Hard / Difficile
- `3`: Super Hard / Ultra difficile

The reward lookup hard-stops at `difficulty > 3`, which matches the four vanilla difficulties.

### Runtime difficulty byte

`CScCondGameDifficulty` checks:

```c
global = *(DAT_141eba750 + 0x18)->0x28;
current_difficulty = *(u8 *)(global + 0x1d756);
expected_difficulty = *(u32 *)(this + 0x40);
mode = *(u32 *)(this + 0x44);
```

Function:

- `FUN_1413a2330 @ 0x1413a2330`

Observed behavior:

- if current difficulty equals expected and mode is `1`, condition passes;
- if current difficulty differs and mode is `0`, condition passes;
- on pass it writes `0xf` to `this + 8`.

So `global + 0x1d756` is the best current candidate for selected difficulty.

No simple direct writer to `global + 0x1d756` was found by scanning displacement writes. This implies the selected difficulty is likely loaded/copied as part of a larger mission/state struct rather than written with a direct single-field instruction.

A cache copy exists in `FUN_141320af0`:

```c
DAT_141e5ec00 = *(u16 *)(global + 0x1d750); // mission id
DAT_141e5ec04 = *(u8  *)(global + 0x1d756); // difficulty
```

This may be easier to probe at runtime than walking the full global pointer chain.

### Mission mode type byte

Many battle result functions read:

```c
mode_type = *(u8 *)(global + 0x1d753);
```

Common comparisons:

- `0x3`
- `0x4`
- `0x5`

This describes the mission mode type, not selected difficulty.

Observed runtime values from `sdk_runtime` probe:

- `0`: Story / Histoire
- `1`: Free Log / Libre
- `2`: Treasure Log
- `3`: likely DLC/special mode, unconfirmed
- `4`: likely DLC/special mode, unconfirmed
- `5`: likely DLC/special mode, unconfirmed
- `6`: inactive/transition/menu reset state

The three unconfirmed values likely map to the Roger, Yamato, and Koby DLC/special logs, but this still needs a focused runtime pass.

Important implication: difficulty/reward systems should be keyed by `(mode_type, mission_id, effective_difficulty)` rather than only `(mission_id, difficulty)`.

### Mission/level id

Many reward helpers read:

```c
mission_or_level_id = *(u16 *)(global + 0x1d750);
```

This is paired with difficulty in multiple table lookup helpers.

### Reward table indexing

Several functions call:

```c
row = FUN_1412f9be0(global->mission_id, global->difficulty);
table_base = *(DAT_141eba738 + 0x18)->0x20;
row_base = table_base + row * 0x6c;
```

Strong candidates:

- `FUN_1412fa1b0 @ 0x1412fa1b0`
  - returns `table_base + 0x334 + row * 0x6c`
- `FUN_141230780 @ 0x141230780`
  - reads `row_base + 0x340`
  - multiplies by `*(u16 *)(param_2 + 10) / 100`
- `FUN_141230830 @ 0x141230830`
  - reads `row_base + 0x33c`
  - multiplies by `*(u16 *)(param_2 + 8) / 100`
- `FUN_1412308e0 @ 0x1412308e0`
  - reads `row_base + 0x334`
  - multiplies by `*(u16 *)(param_2 + 6) / 100`
- `FUN_1412fa200 @ 0x1412fa200`
  - reads multiple `u16` fields from row subranges:
    - `0x374`
    - `0x37c`
    - `0x384`
    - `0x38c`
    - `0x394`

Important: the apparent row size is `0x6c`. This is likely a fixed LinkData/fixed table record keyed by `(mission_id, difficulty)`.

`FUN_1412f9be0 @ 0x1412f9be0` is now confirmed as a row lookup helper:

```c
u16 lookup_reward_row(u32 mission_id, u32 difficulty) {
  if (mission_id > 0xf9 || difficulty > 3) {
    return 0;
  }

  if (!FUN_1412f6fe0(mission_id)) {
    return *(u16 *)(fixed_data_28 + 0xa8 + (mission_id * 0x6e + difficulty) * 2);
  }

  // special/free-mode path uses an episode/page index up to 0x3c
  return *(u16 *)(fixed_data_28 + 0x46cba + (page * 0x2c + difficulty) * 2);
}
```

This is a big constraint for “new difficulty”: vanilla code hard-stops at `difficulty > 3`. A fifth difficulty cannot work only by adding table rows; the lookup must also be patched or wrapped.

Observed fixed data layout clues from `CDataFixedReward` init:

- first block clears `0xfa` mission rows;
- base mission difficulty index area starts around `+0xa8`;
- special/free-mode difficulty index area starts around `+0x46cba`;
- special/free-mode reward rows around `+0x46c88`, stride `0x58`;
- another table initialized at `+0x2ef0`, `10` records of `0xce` bytes.

### End reward commit path

`FUN_14132a670 @ 0x14132a670` looks like a reward commit/update function.

Notable behavior:

- reads `global + 0x1d756` and passes it into `FUN_1412dcc70`;
- reads `global + 0x1d754`;
- gates values with `global + 0x244`;
- writes totals to persistent/global-ish storage:
  - `*(global_10 + 0x14)` capped at `999999999`
  - `*(global_10 + 0x7a0)` capped at `4000000000`

This is a strong target for a reward multiplier/scaler, but patching here would be runtime behavior, not menu/data extension.

### Battle result calculation path

`FUN_14132b570 @ 0x14132b570` is the heavy `CGameStateBattleResult` logic.

It reads:

- `global + 0x1d750`
- `global + 0x1d753`
- `global + 0x1d756`
- `global + 0x1d9b0`
- `global + 0x1dafc`

It calls:

- `FUN_1412dd6c0`
- `FUN_1412dd640`
- `FUN_1412dd9e0`
- `FUN_1412dd950`
- `FUN_14132a510`
- `FUN_14132a670`
- `FUN_14132aae0`
- `FUN_14132d280`

This should be treated as the central result/reward screen pipeline.

## Reward Fields Confirmed Runtime - 2026-05-21

These notes are from `sdk_runtime` probes and should be treated as the current source of truth for end-of-mission reward work.

### Berry / Beli reward

`reward_probe` hooks `FUN_14132a670 @ 0x14132a670`.

Confirmed output shape from mission `35`, normal/free log:

```text
slots=[487000, 16500, 487000, 0, 0, 10750, 1001250, 1351350]
```

Observed mapping:

```text
slot 0 : victory/base berry bonus
slot 1 : combat-obtained or intermediate berry component, still needs more runs
slot 2 : grade S/S+ berry bonus
slot 3 : secondary mission berry bonus
slot 4 : ally/other berry bonus
slot 5 : soul-piece sale bonus or item-sale-derived bonus
slot 6 : visible berry subtotal/total candidate
slot 7 : post-commit/grand total candidate, still needs more runs
```

Previous user-reported run:

```text
victory bonus = 487000
combat obtained = 0
grade S/S+ bonus = 487000
secondary mission bonus = 0
ally bonus = 0
soul-piece sale bonus = 18250
total = 992250
```

So the berry hook is confirmed useful, but slot labels still need a few comparison runs because slot `1`, `5`, `6`, and `7` vary by item sale / display timing.

### Medals / item rewards

`item_reward_probe` hooks `FUN_14132d280 @ 0x14132d280`.

Confirmed capture from mission `35`, normal/free log:

```text
item_reward_probe result=10750
entries=
#0 amount=73 item=0   new=0
#1 amount=64 item=1   new=0
#2 amount=64 item=2   new=1
#3 amount=26 item=3   new=0
#4 amount=26 item=72  new=0
#5 amount=5  item=169 new=0
#6 amount=5  item=181 new=0
#7 amount=31 item=186 new=0
```

This means medal/item reward capture is already working. What remains is naming the item ids against the visible medal names/icons. The hook gives amount, item id, and whether the entry is new.

Do not list medals as missing anymore; list them as captured but not fully labelled.

### Crew points

Crew reward in UI equals crew points. Do not track a separate "crew reward" category unless a later test proves a distinct field.

`result_state_probe` hooks `FUN_14132b570 @ 0x14132b570` and dumps the result object block written by `FUN_14132a0b0 @ 0x14132a0b0`.

Ghidra writes in `FUN_14132a0b0`:

```text
result_state + 0x498 : component 1
result_state + 0x4a0 : component 2
result_state + 0x4a8 : component 3
result_state + 0x4b0 : component 4
result_state + 0x4b8 : bonus delta
result_state + 0x4c0 : total after multiplier
result_state + 0x4c8 : final/display total
```

Confirmed run where user remembered "180-ish":

```text
crew_points_named=
+0x498:109
+0x4a0:0
+0x4a8:0
+0x4b0:30
+0x4b8:41
+0x4c0:180
+0x4c8:180
```

So:

```text
109 + 30 + 41 = 180
```

Previous run:

```text
raw player stat deltas = 27 + 12 + 51 + 10 + 35 + 7 = 142
visible crew points reported by user = 148
```

This implies raw active/save stat deltas are not the authoritative display total. The authoritative crew point total is `result_state + 0x4c0/+0x4c8`.

### Souls

Souls are not confirmed yet.

Current probes log a `soul_state` area near save offsets `+0xfe6c..+0xfe8c`, but it did not move in the tested result flows and is not yet tied to end-screen soul rewards.

Important correction from tester: the recent missions did not reward souls. Lack of movement in `soul_state` is therefore not negative evidence yet; it only means the tested runs were not valid soul-reward probes.

Status: still missing. Needs either a Ghidra pass around soul reward UI/commit functions or a targeted runtime run with known soul values.

Observed soul-reward run - 2026-05-21:

```text
log: plugins/sdk/logs/sdk_runtime/2026-05-21-221212.log
mission_id=77
difficulty=1(normal)
mode_type=2(treasure_log)
visible result:
  kills = 1110
  rank = A
  time = 7:44.46
  souls = 2 small attack souls, 1 small defense soul

rank_fields=[27,1110,1139292992,3,5]
1139292992 as f32 = 464.462890625 seconds = 7:44.46
```

This confirms `result_state_probe.rank_fields` carries at least:

```text
rank_fields[0] = rank/condition row id
rank_fields[1] = kill count
rank_fields[2] = clear time as f32 bits
```

Soul reward remains unresolved in this run. `value_probe.values` did not include `1` or `2`, so it could not directly search for the visible soul counts. `item_reward_probe` did log one suspicious entry:

```text
#19 amount=1 item=238 new=0
```

This may be the `1` small defense soul, but it does not explain the `2` small attack souls. Next runtime probe should explicitly include `1` and `2` in `value_probe.values`, or better, add a soul-specific item/material watch once the item ids are identified.

Observed soul-reward run - 2026-05-22:

```text
log: plugins/sdk/logs/sdk_runtime/2026-05-22-184942.log
mission_id=69
difficulty=1(normal)
mode_type=2(treasure_log)
visible result reported by tester:
  souls = 1 small defense soul, 1 medium defense soul

final active_stats:
  +0x8fc:0
  +0x900:0
  +0x904:29
  +0x908:0
  +0x90c:551
  +0x910:2
  +0x914:120
  +0x918:1113710727
  +0x91c:1085591814
  +0x920:1
  +0x924:0
  +0x928:14

final soul_state:
  +0xfe6c:40
  +0xfe70:29873
  +0xfe74:37
  +0xfe78:1170598752
  +0xfe7c:73
  +0xfe80:7149
  +0xfe84:4294967295
  +0xfe88:4294967295
  +0xfe8c:65535
```

`player_result_probe.soul_state` still does not move in a useful way, so that area is probably not the result reward source.

The best new lead is `player_result_probe.active_stats`: offsets `+0x910` and `+0x920` contain `2` and `1` on a run with two visible soul rewards. This may be a compact bucket/count summary rather than final item ids. It needs one or two controlled runs with different soul mixes to map:

```text
+0x910 maybe total soul entries/count for one soul group
+0x920 maybe another soul group/count flag
```

Do not expose soul scaling yet. The next probe should log active stat deltas with labels and add a soul-focused item/material hook, because the visible result distinguishes size and type while the current stats only expose small integers.

Known UI/data clues:

```text
FUN_1413d6f50 row 0x70 references:
  text/key: in_soul
  icon: cmn_icon_soul_all
  category-ish value: 0x19
```

This proves souls have a distinct result/reward UI entry. It does not yet identify the commit function or the source value.

Next runtime target:

```text
Run with a visible soul reward value on the result screen and add that exact value to `value_probe.values`.
Then compare:
  - `value_probe` hits;
  - `result_state_probe source_rewards/result_copy/soul_counter`;
  - `player_result_probe soul_state`.
```

If the exact visible value never appears in these dumps, the soul reward is probably assembled through a material/item category path rather than a single result-state integer.

### Character XP

OPPW4 does not appear to have a character XP reward category in the end-of-mission result flow. Characters are upgraded through external progression/resources rather than XP gained directly from missions.

Do not track "character XP" as a reward category unless later evidence proves a hidden separate field. For the current reward director model, remove it from scope.

### Rank conditions

Rank conditions are not reward values. They are the thresholds for grade/rank display, for example kill count and clear time conditions.

These likely live in LinkData/fixed mission tables rather than the runtime reward commit block. Future work should identify and dump them from LinkData so a difficulty/rank director can edit:

```text
rank thresholds: kills, time, possibly mode-specific S/S+/future ranks
```

This is needed before adding custom ranks like `SS`, `SSS`, or `X`.

Known Ghidra clues:

```text
global_state + 0x1d9b0 : result/rank table area
global_state + 0x31    : active player/result slot index
result/rank stride     : 0x50 bytes per player/result slot
```

One reader does:

```c
rank_id = *(u16 *)(global + 0x1d9b0 + *(u8 *)(global + 0x31) * 0x50);
fixed_base = *(DAT_141eba738 + 0x18)->0x8;
field_14 = *(u16 *)(fixed_base + rank_id * 0x44 + 0x14);
field_16 = *(u16 *)(fixed_base + rank_id * 0x44 + 0x16);
```

This strongly suggests the visible rank/result conditions are data-backed:

```text
rank/result row id -> fixed table record stride 0x44
```

But the exact field labels are still unknown. The next useful probe/dumper should dump `global + 0x1d9b0` and the linked fixed rows during a result screen, then compare against visible rank conditions such as kills and mission time.

Implemented next probe:

```text
rank_threshold_probe
  dumps global + 0x1d9b0, 4 slots, 0x50 bytes each as u16 words
  reads fixed row: *(DAT_141eba738 + 0x18)->0x8 + rank_row_id * 0x44
  reads candidate condition row: fixed_base + 0xc43c + field_16 * 0x34 when field_16 < 0x68
```

Next runtime comparison needs visible end-screen thresholds, for example:

```text
mission id / mode / difficulty
visible rank
kill condition threshold
clear-time condition threshold
any other visible S/S+ condition
```

Then match those values against `rank_threshold_probe ... fixed=[...] condition=[...]`.

Observed runtime comparison - 2026-05-21:

```text
log: plugins/sdk/logs/sdk_runtime/2026-05-21-215933.log
mission_id=35
difficulty=1(normal)
mode_type=1(free_log)
active_player=0

active result slot:
  rank_row=12

fixed rank row 12:
  +0x04=2
  +0x06=17
  +0x0a=771
  +0x0c=3
  +0x10=50000
  +0x14..+0x22 all point to condition row 12
  +0x24=111
  +0x28=36

condition row 12:
  +0x00=1
  +0x04=2000
  +0x08=5000
  +0x0c=70
  +0x10=70
  +0x14=1
  +0x18=7000
  +0x1c=9000
  +0x20=630
  +0x24=630
  +0x28=6
  +0x2c=65535
  +0x2e=13
  +0x30=2
  +0x32=216
```

This confirms the probe is following a live rank row into a condition row. Field labels are still unknown, but the shape looks like paired threshold groups. The values `70/70` and `630/630` are especially likely to be visible condition thresholds or converted thresholds, while `2000/5000/7000/9000` may be score/time/point gates or internal scaled values. A screenshot or manual copy of the result-condition UI for mission 35 normal/free is now enough to start naming fields.

During transition after the mission the active slot moved to `rank_row=51` / `condition_row=51`; treat that as transition/result-context data until correlated with a visible screen.

Current remaining reward/rank unknowns:

```text
missing:
  - souls source/commit value
  - rank threshold field labels

not missing anymore:
  - Berry/Beli
  - medals/items
  - crew points / recompense d'equipage
  - rank row -> condition row linkage
```

## Working Model

Current likely structure:

```text
global_state = *(DAT_141eba750 + 0x18)->0x28

global_state + 0x1d750 : u16 mission_or_level_id
global_state + 0x1d753 : u8  mode type / mission category
global_state + 0x1d754 : u8  reward/material mode
global_state + 0x1d756 : u8  current difficulty
global_state + 0x1d9b0 : result/rank table area
global_state + 0x1dafc : battle result work flag

fixed_data_base = *(DAT_141eba738 + 0x18)->0x20
reward_row_index = FUN_1412f9be0(mission_or_level_id, current_difficulty)
reward_row_base = fixed_data_base + reward_row_index * 0x6c
```

`CDataFixedReward` probably owns or parses this fixed reward table.

## Plugin Direction

### Short term diagnostic

Added a safe debug probe in `sdk_runtime`, not a gameplay patch:

- read/log `global + 0x1d750`, `+0x1d753`, `+0x1d754`, `+0x1d756`;
- log only on change;
- expose it as SDK game-state debug, not in a random plugin;
- ask tester to switch difficulty/menu/mission and collect values.

Also probe:

- `DAT_141e5ec00`
- `DAT_141e5ec04`

These look like cached mission/difficulty values updated after menu/result state work.

Runtime implementation:

- plugin: `sdk_runtime`
- config: `plugins/configs/sdk_runtime/config.toml` in the current SDK service layout
- section:

```toml
[difficulty_probe]
enabled = true
interval_ms = 250
dump_reward_row = true
```

Expected log shape:

```text
difficulty_probe mission_id=<u16> difficulty=<0..3>(<label>) mode_type=<u8>(<label>) reward_mode=<u8> special_flag=<u8> cached_mission=<u32> cached_difficulty=<u32> global=0x...
```

When `dump_reward_row = true`, the probe also logs a fixed reward row candidate for vanilla mission/difficulty ids:

```text
reward_row index=<u16> fixed20=0x... fixed28=0x... u32=0x334:<v>,0x33c:<v>,0x340:<v>,0x348:<v> u16x4=0x34c:[...],... bytes_39c=[...]
```

This intentionally reads table data only. It does not call game functions from the probe thread and does not patch gameplay.

Tester observations:

```text
mission_id=92  difficulty=2(hard)       mode_type=2(treasure_log)
mission_id=122 difficulty=1(normal)     mode_type=2(treasure_log)
mission_id=131 difficulty=1(normal)     mode_type=2(treasure_log)
mission_id=57  difficulty=1(normal)     mode_type=2(treasure_log)
mission_id=34  difficulty=3(super_hard) mode_type=1(free_log)
mission_id=34  difficulty=3(super_hard) mode_type=0(story)
mission_id=65535 difficulty=255         mode_type=6(inactive_or_transition)
```

Treasure Log appears to use forced/effective difficulties: the player's global difficulty selection may not be the value applied to the mission. A future difficulty patcher must decide whether it patches selected difficulty, effective difficulty, or the mode-specific table/rule that forces the effective value.

`DAT_141e5ec00` / `DAT_141e5ec04` returned `2147483648` in these tests, so the cache hypothesis is weak for this flow. The reliable signal is currently the walked global-state struct.

## Menu / Unlock Findings

Focused export:

- `oppw4-ghidra/game_difficulty_menu.txt`

The menu builder around `FUN_14155f280 @ 0x14155f280` appears to render/select several scenario setup rows through:

```c
FUN_14155cc50(ui, row_kind, lock_or_variant, selected_value);
```

Observed row kinds:

- `0`
- `1`
- `2`
- `3`

The row kind `3` is a strong difficulty-row candidate because it uses:

```c
FUN_14155cc50(*(param_1 + 0x70), 3, cVar, *(u32 *)(param_1 + 0x554));
```

Important menu struct fields:

```text
param_1 + 0x53c : menu/category/state selector
param_1 + 0x540 : scenario mode variant
param_1 + 0x544 : focused row / current setup step
param_1 + 0x550 : selected list index
param_1 + 0x554 : selected mission/quest id for row kind 3
param_1 + 0x558 : costume/form slot used for label lookup
param_1 + 0x538 : generated list count
param_1 + 0x144 : generated mission list
```

`FUN_1415615b0 @ 0x1415615b0` returns a mission-list filter/key:

```c
if (*(u32 *)(menu + 0x53c) == 3 && mission < 0xfa) {
  if (*(u32 *)(menu + 0x540) < 2) {
    return *(u8 *)(fixed_data_28 + mission * 0xdc + 0xb8);
  }
  if (*(u32 *)(menu + 0x540) == 2) {
    return *(u8 *)(fixed_data_28 + mission * 0xc8 + 0xd7c8);
  }
}
```

`FUN_1416137b0 @ 0x1416137b0` generates a list of missions matching that filter/key:

```c
for mission in 0..0xfa {
  if ((mission_flags & 1) &&
      FUN_1412f7560(mission) &&
      !(mission_flags & 0x8000) &&
      fixed_data_28[mission * 0xc8 + 0xd7c8] == filter_key &&
      optional_predicate(mission)) {
    output[count] = mission;
    output_sort_key[count] = *(u16 *)(fixed_data_28 + mission * 0xc8 + 0xd7c6);
    count++;
  }
}
qsort(output, count, 8, ...);
```

This looks more like scenario/mission list generation than the final difficulty choices themselves, but it explains why scenario mode and unlock flags affect what the menu can show.

Super Hard unlock is not yet identified. The likely next target is either:

- a predicate passed into `FUN_1416137b0`;
- a flag checked by the menu row kind `3`;
- or a progression/save flag read before allowing difficulty id `3`.

### LinkData/data path

Find which LinkData/fixed table backs `CDataFixedReward`.

Useful anchors:

- `CDataFixedReward::vftable @ 0x141953408`
- constructor/parser `FUN_1412e03a0`
- row lookup helper `FUN_1412f9be0`
- table reads around `fixed_data_base + 0x334..0x39a`

If this table is LinkData-backed, adding a real new difficulty likely means:

1. add difficulty menu/list data (`tb_level_of_difficulty`);
2. add fixed reward rows for the new difficulty;
3. teach lookup/indexing to accept the new difficulty id;
4. patch text/UI assets for labels.

### LinkData source candidates - 2026-05-22

Binary scans against `D:\SteamLibrary\steamapps\common\OPPW4\LINKDATA` confirm that the fixed rank/condition and difficulty reward byte patterns exist inside LinkData payloads.

Important correction: this is not a full LinkData fixed-data depack yet. The current scanner only extracts/inflates an archive entry payload. The runtime fixed-data pointers use an additional internal depacked layout, so raw payload offsets do not always equal runtime table offsets.

Follow-up Ghidra export:

```text
oppw4-ghidra/game_fixed_data_loaders.txt
oppw4-ghidra/game_fixed_data_id_table.txt
```

The fixed-data loader path is now clearer:

```text
FUN_1415ce9d0(fixed_object, logical_id, ...)
  local_40 = DAT_141e24ee0[logical_id]
  asks the game file system/resource manager for that fixed-data stream

CDataFixedReward vtable group:
  FUN_1412e0300 -> FUN_1415ce9d0(..., 0x14, ...) -> FUN_1412dfa20 parser
  FUN_1412e15f0 -> FUN_1415ce9d0(..., 8,    ...) -> FUN_1412e0e70 parser
  FUN_1412e1af0 -> FUN_1415ce9d0(..., 9,    ...) -> FUN_1412e17a0 parser
```

`DAT_141e24ee0` is zero in the static Ghidra image and appears to be populated at runtime, so `8`, `9`, and `0x14` are logical fixed-data ids, not guaranteed raw archive entry ids. The important part is the parser: it consumes a compact stream with alignment and writes normalized runtime structs at fixed offsets.

Known parser output shapes:

```text
FUN_1412dfa20:
  0xfa records at output +0x10   stride 0x30
  10 records at output +0x2ef0  stride 0xce
  10 records at output +0x36fc  stride 0x30
  3 records at output +0x38dc   stride 0x10

FUN_1412e0e70:
  0xfa records at output +0x10     stride about 0x3e
  0x119 records at output +0x3c9c  stride 0x42
  0xcf records at output +0x8510   stride 0x1c
  1000 records at output +0x9bb4   large text/mission-style block
  0x32 records at output +0x3c834  stride 0x44-ish
  99 records at output +0x3d57c    stride 0x40
  100 shorts at output +0x3ee3c
  10 records at output +0x3ef04

FUN_1412e17a0:
  200 records at output +0x10      stride 0x26
```

This is the depack layer we were missing. Raw payload scans are useful for source candidates, but patching should target the logical fixed-data stream or the normalized runtime layout, not a guessed raw offset.

Confirmed files/entries:

```text
CMN/LINKDATA_A.BIN entry 1    : contains fixed reward / difficulty-scaled reward row byte patterns
CMN/LINKDATA_A.BIN entry 3    : contains fixed rank row and condition row byte patterns among other fixed mission/result data
CMN/LINKDATA_A.BIN entry 2558 : contains a base mission difficulty index-table candidate
LANG/FRA/LINKDATA_LANG_FRA.BIN entry 0 : visible difficulty labels
LANG/ENG/LINKDATA_LANG_ENG.BIN entry 1 : difficulty/help text, including Treasure Log fixed-difficulty text
```

Rank/condition proof:

```text
runtime fixed rank row 12 matched LINKDATA_A entry 3 at 0x320
runtime condition row 12 matched LINKDATA_A entry 3 at 0xc8f4
condition-threshold subsequence also appears around 0xc6bc
```

Offset caveat:

```text
runtime code reads condition rows from fixed_rank_table + 0xc43c + row * 0x34
raw inflated entry 3 contains the exact condition row 12 at payload offset 0xc8f4
raw inflated entry 3 would place condition row 0 around payload offset 0xc684 if stride 0x34 is used
```

So the raw entry payload has either a preceding internal header/table area or a different depacked base than the runtime pointer. Do not patch raw offsets directly until the fixed-data depacker is mapped.

Reward/difficulty proof from runtime log `2026-05-22-184942.log`:

```text
mission_id=69
difficulty=1(normal)
reward_row index=8
runtime row fields:
  0x334=270
  0x33c=550
  0x340=550
  0x348=2500
  0x34c=[18,12,8,8]
  0x354=[24,16,11,11]
  0x35c=[14,9,6,6]
  0x364=[54,36,25,16]
  0x36c=[54,36,25,16]
  0x374=[2,9,18,18]
  0x37c=[2,9,18,18]
  0x384=[3,13,26,30]
  0x38c=[5,23,46,63]
  0x394=[5,23,46,63]
  0x39c bytes=[0,3,5,6]
```

Those exact runtime fields match `LINKDATA_A.BIN` entry `1` at raw inflated payload offset `0x684` for the row-field block. This strongly suggests entry `1` is the source for the row data used by `FUN_1412f9be0` callers, but the internal depacked layout still needs to be mapped before writing patches by offset.

The row index lookup for mission `69`, difficulty `1` also has a candidate match in `LINKDATA_A.BIN` entry `2558`:

```text
index_offset = 0xa8 + (mission_id * 0x6e + difficulty) * 2
             = 0x3bf6
entry 2558 base adjustment observed = +0x10
entry 2558[0x3c06] = 8
```

Current interpretation:

- `entry 2558` maps `(base mission id, vanilla difficulty id)` to a reward row index.
- `entry 1` likely sources reward/gameplay rows with runtime stride `0x6c`.
- `entry 3` contains matched rank/condition-like bytes, but this is now likely a source-candidate or false-friend payload, not the direct runtime table. The runtime table is built by fixed-data parser functions such as `FUN_1412e0e70`, and the logical fixed-data id must be resolved before treating an archive entry as authoritative.
- visible difficulty names/help text are in language LinkData, while gameplay difficulty behavior is in CMN numeric tables plus executable range checks.

This makes a fifth difficulty a mixed data+code patch:

1. map the internal fixed-data depacker/base adjustment for the source entries;
2. extend or virtualize the index table currently addressed as `mission * 0x6e + difficulty`;
3. add/clone reward rows in the depacked reward-row source;
4. patch/wrap `FUN_1412f9be0` because it returns `0` when `difficulty > 3`;
5. update menu/label text in LANG LinkData or via UI hooks;
6. map script conditions (`CScCondGameDifficulty`) so vanilla scripts expecting `0..3` still behave.

### Runtime path

For a first plugin prototype:

- do not add a menu item yet;
- detect selected mission and vanilla difficulty;
- multiply reward values in `FUN_14132a670` or downstream commit;
- later move from runtime multiplier to data-backed rows.

### Difficulty impact map to build next

We know where the active difficulty byte lives, but not every system it controls. The next reverse pass should classify every reader of `global + 0x1d756` by subsystem:

- rewards: Berry, coins/medals, souls, materials;
- enemy scaling: HP, attack, defense, stagger, AI/aggression, spawn rules;
- mission conditions: difficulty-gated events and completion checks;
- UI/menu: available choices, labels, lock/unlock rules;
- mode-specific overrides: Treasure Log and DLC/special logs that force difficulty;
- save/progression: Super Hard unlock and mode completion state.

Plugin design implication:

- `sdk_runtime` may expose observed telemetry only.
- `difficulty_director` should own gameplay patches for selected/effective difficulty.
- `reward_director` should own reward multipliers or reward row patches.
- LinkData/RDB patches should be preferred where the game is data-driven; exe hooks should be reserved for hardcoded range checks like `difficulty > 3`.

## Impact Export - 2026-05-21

Focused export:

- script: `oppw4-ghidra/ExportDifficultyImpactTargets.java`
- output: `oppw4-ghidra/game_difficulty_impact_targets.txt`

### Confirmed reward row helpers

`FUN_1412f9be0(mission_id, difficulty)` remains the central row index helper. Multiple wrappers read the active difficulty and then index fixed reward/gameplay rows with stride `0x6c`.

Confirmed row pointer:

```c
FUN_1412fa1b0() {
  global = *(DAT_141eba750 + 0x18)->0x28;
  row = FUN_1412f9be0(global->mission_id, global->difficulty);
  return fixed_data_20 + 0x334 + row * 0x6c;
}
```

Confirmed direct reward-ish fields:

```text
row + 0x334 : used by FUN_1412308e0 and multi-reader FUN_141230b20
row + 0x33c : used by FUN_141230830, FUN_141231100, FUN_1413fab30
row + 0x340 : used by FUN_141230780 and multi-reader FUN_141230b20/FUN_141231100
row + 0x348 : used by FUN_1415d8780
```

The simple helpers multiply those fields by percentages from another struct:

```text
FUN_1412308e0 -> row+0x334 * *(u16 *)(param_2 + 6)  / 100
FUN_141230830 -> row+0x33c * *(u16 *)(param_2 + 8)  / 100
FUN_141230780 -> row+0x340 * *(u16 *)(param_2 + 10) / 100
```

### Multi-field reward/gameplay arrays

`FUN_1412fa200` and `FUN_1412fa360` read 16-bit arrays inside the same row. They pick different base offsets depending on parameters that look like player/form/slot category selectors.

`FUN_1412fa200` bases:

```text
row + 0x374
row + 0x37c
row + 0x384
row + 0x38c
row + 0x394
```

`FUN_1412fa360` bases:

```text
row + 0x34c
row + 0x354
row + 0x35c
row + 0x364
row + 0x36c
```

These are probably not all final rewards. They may include battle values, drop tables, or per-player reward categories. They should be treated as fixed-row data until runtime tests map each field.

### Result/reward commit path

`FUN_14132a670` remains the best first hook candidate for a simple reward scaler:

- calls `FUN_1412dcc70(active_difficulty, param_3, multiplier)`;
- computes several reward components into `param_1[0..7]`;
- commits totals into global save/result storage;
- caps one value at `999999999`;
- caps another at `4000000000`.

This path is useful for a conservative `reward_director` prototype because it can multiply final values without pretending a fifth difficulty exists yet.

### Mission condition path

`FUN_1413a2330` is confirmed as the `CScCondGameDifficulty` condition:

```c
if (active_difficulty == expected_difficulty) {
  pass = mode == 1;
} else {
  pass = mode == 0;
}
```

This means new/virtual difficulties must account for script conditions. If we ever expose difficulty id `4`, mission conditions expecting `0..3` may fail unless `difficulty_director` maps virtual difficulty back to a vanilla condition id or patches condition checks.

### Candidate gameplay/UI paths

These functions still need labels, but the export shows they all reach difficulty-indexed data:

```text
FUN_1411be490 : calls row lookup and `FUN_1412fa200/360`; likely in-battle reward/drop/effect setup.
FUN_14122fba0 : large setup function, writes values around object offsets 0x1d4/0x1da from row+0x39c style data.
FUN_1413fab30 : uses row+0x33c as a scaled argument to object/effect placement.
FUN_14140a5f0 : calls row lookup and several battle/UI helpers; likely a larger gameplay/reward display bridge.
FUN_1415d2970 : reads reward mode and difficulty, likely late UI/result setup.
FUN_1415d8780 : reads row+0x348 and uses many percent-style table values; likely result UI/reward presentation.
```

The key observation is that difficulty impacts more than the final end-screen money total. A real `difficulty_director` probably needs two layers:

1. a virtual difficulty/effective difficulty layer;
2. a reward/data row layer that can clone or scale fixed rows.

### Practical plugin split

Recommended split after this export:

- `difficulty_director`
  - owns selected/effective difficulty logic;
  - owns virtual difficulty ids and vanilla fallback mapping;
  - patches hard range checks like `difficulty > 3`;
  - eventually owns menu/unlock integration.

- `reward_director`
  - owns final reward scaling;
  - can start by hooking/patching `FUN_14132a670`;
  - later can patch fixed reward rows or clone row data for virtual difficulties.

- `sdk_runtime`
  - remains telemetry only.
  - should not own difficulty gameplay changes.

## Open Questions

- What exact table does `FUN_1412f9be0` search/index?
- Is `tb_level_of_difficulty` an executable UI table, RDB asset, or LinkData string key?
- Are vanilla difficulty ids `0..N` directly stored at `global + 0x1d756`?
- Does `global + 0x244` mean mode/context, and which value means normal battle result commit?
- Which berry `reward_probe` slots are final display fields vs intermediate commit fields?
- Which item ids from `item_reward_probe` map to visible medal names/icons?
- Where are soul rewards computed and committed?
- Which LinkData/fixed mission table stores rank thresholds such as kill count and clear time?

## Next Reverse Steps

1. Map row fields `0x334..0x39c` by runtime/probe or LinkData diff.
2. Decompile/export callers of `FUN_141230780`, `FUN_141230830`, `FUN_1412308e0`, `FUN_1412fa200`, and `FUN_1412fa360`.
3. Find data/source references for `tb_level_of_difficulty`.
4. Identify selected difficulty UI/menu storage before mission start.
5. Find all writes or struct copies that populate `global + 0x1d756`.
6. Compare LinkData rows around the fixed reward table if we can identify the table block.
7. Dump LinkData/fixed mission rank thresholds and map them to visible rank conditions.

Updated next steps:

1. Identify the menu row that maps directly to `tb_level_of_difficulty`.
2. Find the progression/save flag that unlocks difficulty id `3`.
3. Runtime-probe `global + 0x1d756` and `DAT_141e5ec04` while changing difficulty.
4. Patch only the unlock condition first, before attempting a new id `4`.
5. For a new id `4`, patch both menu range and `FUN_1412f9be0` range checks.
