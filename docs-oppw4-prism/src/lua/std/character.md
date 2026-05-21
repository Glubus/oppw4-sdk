# std.character

`std.character` exposes character bank metadata and character handles.

```lua
local character = require("std.character")

local garp = character.find("garp")
local law = character.find("law")
```

Common fields include canonical names, known IDs, model metadata, costumes, and
plugin extension methods.

Official plugins extend character handles. For example, `skin_patcher` can add
model and texture replacement helpers to character handles.
