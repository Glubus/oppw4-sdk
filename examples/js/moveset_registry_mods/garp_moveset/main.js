import { character } from "sdk";

const target = character.find("garp_yng");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for garp_yng");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("garp_moveset.bin");

oppw4.trace("garp_moveset.replace_movesets=" + JSON.stringify(result));