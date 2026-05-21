# SDK Services

SDK services are official plugins that add game-aware behavior to SDK core.

Expected runtime bundle:

```text
plugins/
  sdk/
    sdk.dll
    runtime.dll
    linkdata.dll
    rdb.dll
```

Responsibilities:

- `sdk.dll`: starts SDK core and plugin loading.
- `runtime.dll`: runtime probes, status providers, active character providers.
- `linkdata.dll`: LinkData patch state and patch routing.
- `rdb.dll`: RDB virtual file routing and patch reads.

Missing service DLLs should disable dependent features cleanly instead of crashing the whole SDK.
