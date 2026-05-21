# Known Error Codes

Known negative host call codes:

| Code | Meaning |
| ---: | --- |
| -19 | Null SDK host context |
| -20 | Missing plugin id |
| -21 | Plugin id mismatch |
| -22 | Missing manifest capability |
| -23 | Missing Lua module name |
| -24 | Lua module not declared in manifest |
| -25 | Missing capability name |
| -26 | Missing config schema name |
| -27 | Missing config schema body |
| -28 | Duplicate config schema |

Unknown codes should still be surfaced with the operation name. Do not hide the raw code; it is often the fastest way to find the failing SDK boundary.
