import { character } from "sdk";

const target = character.find("law");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for law");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("law_moveset.bin");

oppw4.trace("law_moveset.replace_movesets=" + JSON.stringify(result));