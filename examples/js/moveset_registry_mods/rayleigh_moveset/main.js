import { character } from "sdk";

const target = character.find("rayleigh_yng");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for rayleigh_yng");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("rayleigh_moveset.bin");

oppw4.trace("rayleigh_moveset.replace_movesets=" + JSON.stringify(result));