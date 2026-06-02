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

- `enemy_spawn_probe` stays disabled by default and is pending a confirmed
  signature for `FUN_1415d1320` or a safer spawn request site.
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

## Open Questions

- Does `FUN_1415d1320` create enemies, drops, event objects, or all of them?
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
