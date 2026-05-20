local character = require("std.character")
local log = require("std.log")

require("skin_patcher")

local zoro = character.find("zoro")

local model_path = "assets/costumes/default/MPLC001_Zoro_Custom.g1m"
local portrait_path = "assets/portraits/zoro_custom.dds"

zoro:replace_costume(1, model_path)
zoro:replace_portrait(1, portrait_path)

log.info("registered asset replacements for " .. zoro.canonical)
