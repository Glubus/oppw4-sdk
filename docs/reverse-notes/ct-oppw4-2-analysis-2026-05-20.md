# OPPW4 (2).CT analysis - 2026-05-20

Source: `C:\Users\Osef\Downloads\OPPW4 (2).CT`

This Cheat Engine table is outdated, so none of its raw static offsets should be
ported directly. It is still useful as a map of old ideas, old injection sites,
and probable runtime structures.

## High-value takeaways

- The `CUSTOM BOSS BATTLES` section is the useful part for an NPC/playable
  plugin. It forces two related values:
  - `actor_id`: written into `[rsi+0x148]` and `[rbx+0x148]`, then read as a
    `word`.
  - `slot_or_form_id`: written into `[rax+rcx*2+0x10]`, then read as a `word`.
- For normal playable characters, `actor_id == slot_or_form_id`.
- For DLC, NPCs, and forms, `actor_id` often points to the base actor/model
  family while `slot_or_form_id` points to a playable slot, NPC row, or form row.
- This gives a strong direction for a future `npc_director` or
  `playable_director` plugin:
  - hook the current equivalent of these three reads;
  - log both values first;
  - expose Lua like `npc_director.force_player(character.find("kaku"))`;
  - later expose it through `std.character` extensions.
- The `Custom Movesets > Luffy` script is also useful: it finds a moveset-ish
  structure by signature and copies blocks inside it. That supports the idea
  that some moveset edits can be runtime memory patches, not only LinkData file
  virtualization.

## Important warning

The table mixes old static offsets and old signatures. Treat every address below
as a Ghidra/search hint only. The current SDK should prefer:

- signature scan;
- Ghidra xrefs around nearby code;
- logging/probing before patching;
- SDK hook services instead of raw per-plugin trampolines.

## NPC/player swap hook hypothesis

Every `CUSTOM BOSS BATTLES` child script uses the same three patch points:

| Old site | Original read | Forced write in CT | Meaning guess |
|---|---|---|---|
| `OPPW4.exe+141758E` | `movzx eax, word ptr [rsi+148]` | `mov dword ptr [rsi+148], actor_id` | actor/current character field |
| `OPPW4.exe+12F613A` | `movzx r10d, word ptr [rax+rcx*2+10]` | `mov dword ptr [rax+rcx*2+10], slot_or_form_id` | playable slot/form table |
| `OPPW4.exe+14B714B` | `movzx edx, word ptr [rbx+148]` | `mov dword ptr [rbx+148], actor_id` | second actor/current character read |

The scripts write a dword, but the game reads a word. So the value is probably a
16-bit row id.

## Extracted custom boss battle ids

| Name | actor_id | slot_or_form_id |
|---|---:|---:|
| Luffy | 0 | 0 |
| Zoro | 1 | 1 |
| Nami (NPC) | 2 | 2 |
| Usopp | 3 | 3 |
| Sanji | 4 | 4 |
| Chopper | 5 | 5 |
| Robin (NPC) | 6 | 6 |
| Franky (NPC) | 7 | 7 |
| Brook (NPC NO MOVES) | 8 | 8 |
| Ace | 9 | 9 |
| Hancock | 10 | 10 |
| Jimbei | 11 | 11 |
| Whitebeard | 12 | 12 |
| Buggy | 13 | 13 |
| Mihawk | 14 | 14 |
| Crocodile | 15 | 15 |
| Teech | 16 | 16 |
| Kizaru | 18 | 18 |
| Kuzan | 19 | 19 |
| Akainu | 20 | 20 |
| Smoker | 23 | 23 |
| Marco | 24 | 24 |
| Garp (NPC) | 25 | 25 |
| Law | 26 | 26 |
| Doflamingo | 27 | 27 |
| Tashigi | 28 | 28 |
| Fujitora | 30 | 30 |
| Sabo | 31 | 31 |
| Lucci | 32 | 32 |
| Ivankov | 35 | 35 |
| Shanks | 36 | 36 |
| Bartolomeo | 37 | 37 |
| Cavendish | 38 | 38 |
| Carrot | 40 | 40 |
| Reiju | 41 | 41 |
| Ichiji | 42 | 42 |
| Niji | 43 | 43 |
| Yonji | 44 | 44 |
| Bege | 45 | 45 |
| Big Mom | 46 | 46 |
| Katakuri | 47 | 47 |
| Kaido | 48 | 48 |
| Kid | 49 | 49 |
| Hawkins | 50 | 50 |
| New World Luffy | 51 | 51 |
| New World Zoro | 52 | 52 |
| New World Nami | 53 | 53 |
| New World Usopp | 54 | 54 |
| New World Sanji | 55 | 55 |
| New World Chopper | 56 | 56 |
| New World Robin | 57 | 57 |
| New World Franky | 58 | 58 |
| New World Brook | 59 | 59 |
| Smoothie (DLC) | 281 | 60 |
| Cracker (DLC) | 282 | 61 |
| Judge (DLC) | 127 | 62 |
| Drake (DLC) | 109 | 63 |
| Killer (DLC) | 283 | 64 |
| Urouge (DLC) | 284 | 65 |
| Okiku (DLC) | 285 | 66 |
| Kin'emon (DLC) | 102 | 67 |
| Oden (DLC) | 286 | 68 |
| Luffy (Onigashima Battle) (DLC) | 293 | 69 |
| Kaido (Onigashima Battle) (DLC) | 294 | 70 |
| Yamato (DLC) | 287 | 71 |
| Uta (DLC) | 288 | 72 |
| Shanks (DLC) | 36 | 73 |
| Koby (DLC) | 100 | 74 |
| Gold Roger (DLC) | 289 | 75 |
| Young Garp (DLC) | 25 | 76 |
| Young Rayleigh (DLC) | 116 | 77 |
| Mr.2 (NPC) | 77 | 105 |
| Mr.3 (NPC) | 78 | 106 |
| Kaku (NPC) | 80 | 107 |
| Jabra (NPC) | 82 | 108 |
| Blueno (NPC) | 83 | 109 |
| Sentomaru (NPC) | 84 | 110 |
| Pacifista (NPC) | 85 | 111 |
| Jozu (NPC) | 88 | 114 |
| Vista (NPC) | 89 | 115 |
| Mr.1 (NPC) | 93 | 119 |
| Bellamy (NPC) | 94 | 120 |
| Burgess (NPC) | 99 | 125 |
| Koby (NPC) | 100 | 126 |
| Sengoku (NPC) | 101 | 128 |
| Kin'emon (NPC) | 102 | 129 |
| Pica (NPC) | 104 | 132 |
| Big Pica (NPC) | 104 | 135 |
| Diamante (NPC) | 105 | 138 |
| Jack (NPC) | 107 | 140 |
| Perospero (NPC) | 108 | 141 |
| Drake (NPC) | 109 | 142 |
| Pirate (NPC AXE) | 159 | 200 |
| Pirate (NPC CLUB) | 159 | 201 |
| Marine (NPC FISTS) | 160 | 204 |
| Pirate (NPC CLAWS) | 159 | 208 |
| Pirate (NPC MUSCLE) | 159 | 209 |
| Marine (NPC LEGS) | 160 | 211 |
| Franky Family (NPC CLUB) | 172 | 219 |
| Giant Marine (NPC) | 217 | 220 |
| Nutcracker (NPC) | 235 | 222 |
| Big Mom Soldier (NPC) | 236 | 223 |
| Wano Soldier (NPC) | 156 | 224 |
| Germa Soldier (NPC) | 240 | 226 |
| Giant Cracker (Cracker Ability) | 282 | 545 |
| Kaido Dragon | 48 | 565 |
| Big Mom Ability | 46 | 564 |
| Hakuba | 38 | 569 |
| Giant Urouge | 284 | 574 |
| Gear 2 Luffy | 0 | 568 |
| Bounceman | 0 | 558 |
| Snakeman | 0 | 559 |
| Gear 5 Luffy | 0 | 615 |
| Human Beast Yamato | 287 | 617 |
| Gold Gear Uta | 288 | 618 |

## Custom moveset script

Only one child exists in the CT: `Custom Movesets > Luffy`.

Signature:

```text
48 8B 48 10 48 85 C9 74 B4
```

Old injection site:

```text
OPPW4.exe+11F62D8
```

Original code around it:

```asm
mov rax,[rdi+000002D8]
test rax,rax
je ...
mov rcx,[rax+10]
test rcx,rcx
je ...
```

Patch behavior:

- `rax = [rax+10]`
- checks `word ptr [rax] == 0x2DD2`
- copies many 8-byte fields from later offsets back onto earlier offsets
- examples:
  - `[rax+0x270 + x] -> [rax + x]`
  - `[rax+0x3260 + x] -> [rax+0x1040 + x]`
  - `[rax+0x3330 + x] -> [rax+0x12B0 + x]`

This looks like an in-memory moveset table mutation. It may be useful later for:

- runtime moveset remix/debug;
- finding the loaded LinkData/moveset structure in RAM;
- making a moveset inspector that labels copied blocks.

But for production moveset mods, the cleaner path is still the current
`moveset_patcher`/LinkData virtualization approach.

## Other CT ideas worth keeping

### special_move_director

Old entries:

- `Special Move Modifier (Up)`
- `Special Move Modifier (Left)`
- `Special Move Modifier (Right)`
- `Special Move Modifier (Down)`

All are based around:

```text
"OPPW4.exe"+01EC24E8
```

The CT exposes child labels like movelist/combo fields. This could become a
plugin that changes equipped special moves by Lua or per-character config. Need
to rediscover the current structure first.

### move_animation_probe

Old entries:

- `MOVE ID FINDER`: `"OPPW4.exe"+01ECA7D0`
- `ANIMATION ID FINDER`: `"OPPW4.exe"+01ECA7D8`
- `ACTIVATE PLAYER`: `"OPPW4.exe"+01ECA7D8`

This is useful as a Ghidra clue for a future probe plugin:

- log current move id;
- log current animation id;
- correlate moveset rows with animation/effect triggers.

### reward_director

Old scripts:

- `Unlimited Beli`: old injection near `OPPW4.exe+14C61EB`, calls
  `OPPW4.exe+1609C90`.
- `Unlimited Material`: old injection near `OPPW4.exe+14CA133`.
- `Unlimited Soul Materials`: old injection near `OPPW4.exe+150381F`.

Useful idea, but the plugin we want should not just make values infinite. Better
target:

- end-of-mission reward hook;
- multiply rewards by difficulty;
- separate multipliers for character XP, Beli, medals/materials, souls.

### battle_quality/debug plugins

Old scripts:

- `Unlimited Special Move Meter`: old sites `OPPW4.exe+11F4FC8` and
  `OPPW4.exe+11C4AEB`.
- `Unlimited K.O.`: old site `OPPW4.exe+1442294`.
- `Unlimited Stamina`: old site `OPPW4.exe+15D65AF`.
- `Special Move Cutscene Skip`: signature `0F 87 59 01 00 00 BE`, old site
  `OPPW4.exe+125DDB8`.

These are mostly debug/quality-of-life ideas. They should be separate plugins or
Lua modules, not mixed into `fx_director`/`moveset_patcher`.

## Plugin ideas from this CT

### npc_director

Purpose: make NPCs/forms playable or force current battle actor.

Potential Lua:

```lua
local character = require("std.character")
local npc = require("npc_director")

npc.force_player(character.find("kaku"))
npc.force_form(character.find("luffy"), "gear_5")
```

First implementation should be diagnostic only:

- hook the three old-equivalent read sites;
- log `actor_id`, `slot_or_form_id`, pointers, and mission context;
- no writes until current build behavior is confirmed.

### character_id_probe

Purpose: feed the character bank with confirmed IDs.

This should subscribe to the same discovered reads and emit:

- active player actor id;
- form/slot id;
- NPC/boss id when present;
- model/moveset ids if nearby fields reveal them.

### special_move_director

Purpose: edit equipped special moves.

Potential Lua:

```lua
local character = require("std.character")
require("special_move_director")

character.find("zoro"):set_special_move("up", 12345)
```

### reward_director

Purpose: mission reward scaling.

Potential Lua:

```lua
local rewards = require("reward_director")

rewards.scale({
  difficulty = {
    hard = { beli = 1.5, xp = 1.25, souls = 1.2 },
    ultra_hard = { beli = 2.0, xp = 1.5, souls = 1.5 },
  },
})
```

### move_animation_probe

Purpose: reverse moveset rows.

Outputs:

- current move id;
- current animation id;
- current effect id if `fx_director` observer is enabled;
- active character id from SDK.

This would be very useful for connecting LinkData rows to actual gameplay.

## Next reverse steps

1. Search current `OPPW4.exe` for these byte patterns:
   - `0F B7 86 48 01 00 00`
   - `44 0F B7 54 48 10`
   - `0F B7 93 48 01 00 00`
   - `48 8B 48 10 48 85 C9 74 B4`
   - `0F 87 59 01 00 00 BE`
2. If found, build a diagnostic-only hook plugin before any patching.
3. Compare logged `actor_id` and `slot_or_form_id` with the table above.
4. Add confirmed mappings to the character bank, with source marked as
   `ct_oppw4_2` until verified in current runtime.
5. For NPCs/forms, do not assume full playability. Some rows may need matching
   model, skeleton/moveset, special action table, and effect table.
