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
```

Expected log shape:

```text
difficulty_probe mission_id=<u16> difficulty=<0..3>(<label>) mode_type=<u8>(<label>) reward_mode=<u8> special_flag=<u8> cached_mission=<u32> cached_difficulty=<u32> global=0x...
```

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

### Runtime path

For a first plugin prototype:

- do not add a menu item yet;
- detect selected mission and vanilla difficulty;
- multiply reward values in `FUN_14132a670` or downstream commit;
- later move from runtime multiplier to data-backed rows.

### Difficulty impact map to build next

We know where the active difficulty byte lives, but not every system it controls. The next reverse pass should classify every reader of `global + 0x1d756` by subsystem:

- rewards: Berry, character XP, coins/medals, souls, materials;
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

## Open Questions

- What exact table does `FUN_1412f9be0` search/index?
- Is `tb_level_of_difficulty` an executable UI table, RDB asset, or LinkData string key?
- Are vanilla difficulty ids `0..N` directly stored at `global + 0x1d756`?
- Does `global + 0x244` mean mode/context, and which value means normal battle result commit?
- Which reward fields map to Berry, character XP, coins/medals, souls?

## Next Reverse Steps

1. Decompile/export `FUN_1412f9be0`.
2. Decompile/export `CDataFixedReward` parser `FUN_1412e03a0`.
3. Find all writes to `global + 0x1d756`.
4. Find data/source references for `tb_level_of_difficulty`.
5. Compare LinkData rows around the fixed reward table if we can identify the table block.

Updated next steps:

1. Identify the menu row that maps directly to `tb_level_of_difficulty`.
2. Find the progression/save flag that unlocks difficulty id `3`.
3. Runtime-probe `global + 0x1d756` and `DAT_141e5ec04` while changing difficulty.
4. Patch only the unlock condition first, before attempting a new id `4`.
5. For a new id `4`, patch both menu range and `FUN_1412f9be0` range checks.
