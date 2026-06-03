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
- Even if `write_stats = true`, writes are refused until Ghidra/runtime logs
  prove a reliable enemy-only filter. This prevents touching player/allies/bosses.

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
