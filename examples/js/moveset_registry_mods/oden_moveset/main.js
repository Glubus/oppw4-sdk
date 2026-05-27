import { character } from "sdk";

const target = character.find("oden");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for oden");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("oden_moveset.bin");

oppw4.trace("oden_moveset.replace_movesets=" + JSON.stringify(result));