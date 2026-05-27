import { character } from "sdk";

const target = character.find("ace");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for ace");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("ace_moveset.bin");

oppw4.trace("ace_moveset.replace_movesets=" + JSON.stringify(result));