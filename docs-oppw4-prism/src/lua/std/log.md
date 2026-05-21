# std.log

`std.log` writes mod-scoped logs.

```lua
local log = require("std.log")

log.debug("debug detail")
log.info("loaded")
log.warn("fallback used")
log.error("failed")
```

Logs are routed through SDK core and written under the SDK mod log folder.
Release host logs mirror only warning and error entries.
