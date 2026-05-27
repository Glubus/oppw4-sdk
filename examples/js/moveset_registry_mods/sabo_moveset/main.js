import { character } from "sdk";

const target = character.find("sabo");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for sabo");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("sabo_moveset.bin");

oppw4.trace("sabo_moveset.replace_movesets=" + JSON.stringify(result));