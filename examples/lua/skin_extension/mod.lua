local character = require("std.character")
local log = require("std.log")

require("sdk.rdb.patcher")

local law = character.find("law")

law:replace_costume(1, "MPLC026_Law_Custom.g1m")
law:replace_portrait(1, "ui/portraits/law_custom.dds")

log.info("registered skin replacements for " .. law.canonical)
