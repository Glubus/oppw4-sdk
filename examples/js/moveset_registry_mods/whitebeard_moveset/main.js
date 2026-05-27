import { character } from "sdk";

const target = character.find("newgate");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for newgate");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("whitebeard_moveset.bin");

oppw4.trace("whitebeard_moveset.replace_movesets=" + JSON.stringify(result));