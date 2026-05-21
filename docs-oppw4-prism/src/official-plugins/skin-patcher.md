# skin_patcher

`skin_patcher` owns the Lua-facing skin replacement API.

It should not become an RDB implementation. RDB path lookup and virtual file routing belong to `sdk_rdb`; `skin_patcher` should express intent such as "replace this model" or "replace these body-part textures".

Expected Lua direction:

```lua
local character = require("std.character")

character.find("garp"):replace_model("young", "my_super_model.g1m")

character.find("garp"):replace_textures("young", {
  { part = "body", path = "my_super_body.g1t" },
  { part = "left_arm", path = "my_super_left_arm.g1t" },
})
```

Data requirements:

- costumes must be named in the data bank;
- each costume may declare models, textures, portraits, voices, and other assets;
- texture assets may be grouped by body part;
- multiple weapons are supported through named weapon entries, not a single `weapon` field.

The plugin also works without Lua when configured through files. Lua is the ergonomic front end, not the only execution path.
