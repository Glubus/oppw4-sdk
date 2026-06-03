# Enemy Spawn / Collision / Stats Ghidra Notes

Date: 2026-06-02

Goal: find the native enemy spawn, stats, placement, and mob-vs-mob collision
pipeline. Do not clone live actor/entity memory. Enemy creation must go through
game-owned spawn/request/init functions.

## Current Evidence

### Spawn / table candidates

| Function | Current read | Confidence | Next check |
| --- | --- | --- | --- |
| `FUN_1412505b0` | Reads fixed byte tables at `0xb3d8`, `0xb3dc`, `0xb3e0`; notes say successful roll calls `FUN_1415d1320`. | Medium | Export callers/callees and inspect params passed into `FUN_1415d1320`. |
| `FUN_141250830` | Weighted selection path using `FUN_1412d5b50`; adjacent to spawn/drop probability pipeline. | Medium | Determine candidate meaning and whether output becomes enemy/event/drop request. |
| `FUN_141254a70` | Reads mission/difficulty reward row and fixed probability helpers. | Medium | Trace whether selected category/candidate flows toward enemy creation. |
| `FUN_1415d1320` | Suspected spawn/event/drop function from previous notes. | Low/Medium | Highest priority: decompile, callers, return value, allocation/init calls. |

### Stats / actor candidates

| Function | Current read | Confidence | Next check |
| --- | --- | --- | --- |
| `actor_stat_init_141231100` | Runtime signature exists. Logs actor/source/mode and actor fields `+0x34`, `+0x38`, `+0x3c`, `+0x40`. | High | Use as read-only enemy stats probe until actor team/type filters are proven. |
| `FUN_14124e670` | Combat pressure scalar clamped `0..9`, feeds behavior/chance paths. | Medium | Keep separate from spawn; useful for enemy aggression/pressure later. |

## Runtime Probe Policy

- `enemy_spawn_probe` has a confirmed runtime signature for `FUN_1415d1320`.
  It remains config-gated and only logs request data/callsite source.
- `enemy_stats_probe` reuses the existing actor stat init hook and is read-only
  by default.
- If `write_stats = true`, writes are filtered to the observed commander /
  officer candidate family only: `source_stats.byte00` in `1|5` and
  `stat3c/stat40 = 390/390`. In-game validation showed this is not the small
  mob family.

## Ghidra Workflow

1. Run `ExportEnemySpawnCollisionTargets.java`.
2. Inspect `FUN_1415d1320` first:
   - parameters and return value;
   - callers and immediate callees;
   - allocation/init/register calls;
   - writes to position/team/type/stats.
3. Follow any function that writes transform/floor/collision data:
   - position vectors;
   - capsule/radius/body setup;
   - collision masks;
   - navmesh/floor snap.
4. Only after a confirmed spawn request site exists, add a real
   `enemy_spawn_probe` signature and hook.

## Export 2026-06-02

Output: `docs/reverse-notes/enemy-spawn-collision-targets-2026-06-02.txt`

### `FUN_1415d1320`

Signature from decompile:

```text
FUN_1415d1320(owner, request_type, position_vec4, arg4, arg5) -> bool/u64
```

Evidence:

- `request_type` is filtered against ranges `6..8` and `0x19..0x27`.
- `position_vec4` is copied into the queued request payload.
- `owner + 0x4b80` points at the request pool/queue state.
- `FUN_14001b6e0(owner+0x4b80+0x18)` allocates/gets a slot.
- `FUN_1411a6ac0(slot, payload, sequence)` writes/enqueues the request.
- Multiple callers use it, so this is a generic spawn/event/effect request path,
  not an enemy-only creation function.

Current runtime decision:

- `enemy_spawn_probe` hooks this function only when enabled by config.
- It logs request type, owner, position, arg4/arg5, and result.
- It also wraps the three spawn-table callsites below to log which source path
  emitted the request:
  - `1412550e2`: `direct_141254a70`
  - `1412507c7`: `extra_1412505b0`
  - `141250e66`: `weighted_141250830`
- It does not multiply, patch, or classify requests yet.

### Important callers

- `FUN_141254a70`, `FUN_1412505b0`, `FUN_141250830` call `FUN_1415d1320` from
  the difficulty/spawn-table pipeline.
- `FUN_141254a70` does additional terrain/floor-ish adjustment before calling:
  `FUN_141598ef0`, `FUN_14159bcc0`, then adds `+60.0` to the final Y-like slot.
- This makes `FUN_141254a70` a better next target for placement/floor-snap study
  than patching `FUN_1415d1320` directly.

## Runtime Log Analysis 2026-06-03

Log: `D:\SteamLibrary\steamapps\common\OPPW4\plugins\sdk\logs\sdk_runtime\2026-06-03-082343.log`

Install result:

- `enemy_spawn_probe` installed on `FUN_1415d1320`.
- All three spawn-table callsites installed:
  - `direct_141254a70`
  - `extra_1412505b0`
  - `weighted_141250830`
- No callsite byte mismatch or install failure.

Observed request summary:

| Request | Source | Type | Notes |
| --- | --- | --- | --- |
| call 1 | unknown caller | `4` | Generic `FUN_1415d1320` caller; not one of the three wrapped spawn-table paths. |
| call 2 | `direct_141254a70` | `5` | Immediately followed by a large entity-counter jump. |
| call 3 | unknown caller | `0` | Same timestamp as call 2; likely another request produced by the same spawn burst or another unwrapped caller. |
| call 4 | `direct_141254a70` | `0` | Direct path can emit multiple request types, not only one fixed type. |
| call 5 | `extra_1412505b0` | `6` | Matches Ghidra: this path computes `6` or `7` from `param_1+6`. |
| call 6 | `extra_1412505b0` | `6` | Repeated later, still accepted with `result=1`. |
| call 7 | unknown caller | `0` | Generic unwrapped caller; needs source classification before use. |

Counter correlation:

- At `08:25:23`, `direct_141254a70 type=5` plus a nearby unwrapped `type=0`
  are followed by:
  - `+0x44c: 374 -> 605`
  - `+0x904: 4 -> 6`
  - `+0x90c: 31 -> 82`
  - `+0xee0: 374 -> 605`
- At `08:26:00`, `extra_1412505b0 type=6` plus a nearby unwrapped `type=0`
  are followed by:
  - `+0x44c: 819 -> 930`
  - `+0x480: 3 -> 6`
  - `+0x904: 7 -> 9`
  - `+0x90c: 95 -> 116`
  - `+0xee0: 819 -> 930`

Current interpretation:

- `FUN_1415d1320` is confirmed as a request enqueue path, not the final actor
  allocation/init function. It is still useful because accepted requests
  correlate with visible mission entity-count changes.
- `direct_141254a70` appears to be the main table-driven request path. It
  includes terrain/floor adjustment before enqueue and can emit at least
  `type=0` and `type=5`.
- `extra_1412505b0` appears to be a secondary/special request path. Runtime
  confirms `type=6`; Ghidra suggests it can also emit `type=7`.
- `weighted_141250830` installed cleanly but did not fire in this run. Keep it
  wrapped for future missions.
- Unwrapped `type=0` and `type=4` requests prove there are still other callers
  into `FUN_1415d1320`. These must not be used for spawn scaling until their
  caller is identified.

Next safest work:

1. Add return/caller classification for remaining unwrapped `FUN_1415d1320`
   calls, or add callsite wrappers for the other Ghidra callers listed in the
   export.
2. Enable/read `actor_stat_init_probe` in read-only mode and correlate stat-init
   bursts with the request timestamps above.
3. Do not multiply `FUN_1415d1320` calls yet. The safer first prototype is still
   enemy stats classification, then spawn scaling on a confirmed enemy-only
   request path with placement/floor handling.

## Runtime Stats Analysis 2026-06-03

Log: `D:\SteamLibrary\steamapps\common\OPPW4\plugins\sdk\logs\sdk_runtime\2026-06-03-182853.log`

Install result:

- `enemy_stats_probe` armed with `write_stats=false`.
- `actor_stat_init_probe` installed.
- 512 stat-init calls logged, all `write_status=read_only`.

Observed stat families:

| Actor stats | Count | Current guess |
| --- | ---: | --- |
| `stat3c=390 stat40=390` | 509 | Commander/officer candidate family; in-game HP x10 affected commandants, not small mobs. |
| `stat3c=585 stat40=585` | 2 | Likely minibosses; user observed only two spawned from points. |
| `stat3c=1125 stat40=1184` | 1 | Special/boss-like actor or non-standard important unit. |

Observed source categories:

| `source_stats.byte00` | Count | Current guess |
| --- | ---: | --- |
| `1` | 370 | Random/basic mob family. |
| `5` | 123 | Random/basic mob family. |
| `65` | 12 | Unknown; possibly loop/script/special wave group. |
| `71` | 1 | Unknown special source. |
| `97` | 6 | Unknown special source. |

Notes:

- `source_stats.word08` varies like an enemy/source row id. Frequent values in
  this run included `47`, `20`, `48`, `49`, `16`, `21`, `46`, `13`, `14`,
  `17`, and `15`.
- `mode` cycles `0..7` across repeated actors from the same source, so mode is
  not enough by itself to classify enemy type. It may represent slot/index
  within a group or spawn batch.
- The first write prototype targets only the observed
  `stat3c=390/stat40=390` family, now known from in-game validation to affect
  commandants/officers rather than small mobs. Keep unknown special `byte00`
  values excluded.
- Runtime probe was updated to emit compact summaries at calls `64`, `128`,
  `256`, and `512` so future logs can be analyzed without manually parsing 512
  individual lines.

Follow-up log:
`D:\SteamLibrary\steamapps\common\OPPW4\plugins\sdk\logs\sdk_runtime\2026-06-03-200924.log`

- Probe stayed read-only: `write_stats=false`, every stat-init write status was
  `read_only`.
- Spawn request trace stayed stable:
  - `extra_1412505b0` emitted `type=6` requests.
  - `direct_141254a70` emitted `type=0` and `type=4` requests.
  - Additional direct `FUN_1415d1320` calls still emitted `type=0` and
    `type=5`, confirming there are more request callsites to classify.
- The compact summary no longer overflowed at calls `64`, `128`, and `256`, but
  still overflowed by `70` groups at call `512`. The runtime summary capacity
  was increased from 32 to 128 groups after this log.
- The first write prototype remains stats-only on the
  `stat3c=390/stat40=390` family, now classified as commander/officer
  candidate from in-game validation. Spawn scaling is still blocked on a
  cleaner spawn/floor/collision path.
- First write prototype implemented after the `2026-06-03-202608.log` check:
  only `byte00=1|5` with `stat3c/stat40=390/390` can be mutated. HP fields
  `actor+0x3c` and `actor+0x40` are scaled by `hp_multiplier`; unknown/sentinel
  attack-like fields are skipped unless they contain normal scalable values.
- In-game HP x10 validation: this filter increased commandants' HP, not small
  mobs. The runtime game config was switched back to `write_stats=false` after
  this result.
- `source_stats.word08` is now the strongest visible candidate for a character
  type/row id, while `word0a` changes too often and likely represents a
  variation, instance, or spawn-slot value. The runtime log now includes
  `source_stats.head_u16` for offsets `+0x00..+0x1e` so the next run can map
  real mob/commander ids instead of filtering by HP-like stats.
- Ghidra shows `FUN_141231100` writes actor/output ids into the beginning of
  `param_1`: `param_1[0]`, `param_1[1]`, `param_1[2]`, and related head fields.
  Runtime logging now includes `actor_stats.head_u16` too; this should be the
  next place to identify the real small-mob character/type id.
- Runtime `2026-06-03-221538.log` showed ids matching the old CT notes:
  `actor_stats.head_u16[0]=224`, `head_u16[1]=262`, `head_u16[2]=910`, with
  `stat3c/stat40=390/390`. In-game HP x10 validation showed this affects
  miniboss/elite units, not small crowd mobs. The game config was switched back
  to `write_stats=false` after the test.
- Current conclusion: `FUN_141231100` covers full actor/elite/miniboss-style
  units. The low-value crowd/minimob soldiers likely use another spawn/stat
  path and will need a separate hook/probe.

### `FUN_141231100`

This is confirmed actor stat init:

- `param_1` is actor stat output.
- `param_2` is source/config actor data.
- `param_3` is mode.
- output HP-like fields are written at `param_1 + 0x3c` and `param_1 + 0x40`
  because the decompiler indexes `ushort *param_1` with `0x1e` and `0x20`.
- source direct stats come from `param_2 + 0x34`, `param_2 + 0x38`,
  `param_2 + 0x44`.
- mission/difficulty-scaled stats use fixed table offsets `row*0x6c + 0x33c`
  and `row*0x6c + 0x340`.

## Open Questions

- Which `FUN_1415d1320` request types map to enemy spawns vs drops/events?
- Which field identifies enemy vs player vs ally vs boss?
- Which init call registers collision body/capsule?
- Is mob-vs-mob collision controlled by a radius, mask, or avoidance push?
- Is there a floor snap function usable after offset/spread spawn placement?

## Acceptance For First Real Prototype

- Hook logs visible enemy creations with actor pointer, source pointer, type/team
  if known, position, and stats.
- Enemy stat scaling is applied only after an enemy-only filter is confirmed.
- Spawn multiplication repeats native spawn requests or edits native counts before
  creation; no actor/entity memory copy path is allowed.
