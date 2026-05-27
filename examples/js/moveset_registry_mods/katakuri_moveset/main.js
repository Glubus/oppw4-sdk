import { character } from "sdk";

const target = character.find("katakuri");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for katakuri");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("katakuri_moveset.bin");

oppw4.trace("katakuri_moveset.replace_movesets=" + JSON.stringify(result));