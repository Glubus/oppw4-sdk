import { character } from "sdk";

const target = character.find("marco");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for marco");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("marco_moveset.bin");

oppw4.trace("marco_moveset.replace_movesets=" + JSON.stringify(result));