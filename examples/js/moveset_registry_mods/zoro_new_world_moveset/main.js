import { character } from "sdk";

const target = character.find("zoro");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for zoro");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("zoro_new_world_moveset.bin");

oppw4.trace("zoro_new_world_moveset.replace_movesets=" + JSON.stringify(result));