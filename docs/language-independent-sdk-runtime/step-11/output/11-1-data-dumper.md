# Step 11.1 - Data Dumper

Implemented offline exporter:

- `oppw4-data/scripts/export_runtime_snapshots.py`

Behavior:

- Reads sdk.runtime `.log` files or log directories.
- Groups `difficulty_probe`, `rank_threshold_probe`, `rank_helper_*`, `reward_event`, `reward_probe`, `result_state`, and `fixed_data_probe` lines by `mission_id`.
- Appends compact evidence blocks into `oppw4-data/missions/mission_XXXX/evidence.md`.
- Supports `--dry-run`.
- Does not run in-game and does not mutate gameplay.

Example:

```powershell
python .\oppw4-data\scripts\export_runtime_snapshots.py D:\SteamLibrary\steamapps\common\OPPW4\plugins\sdk\logs\sdk_runtime --dry-run
```
