import { character } from "sdk";

const target = character.find("urouge");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for urouge");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("urouge_moveset.bin");

oppw4.trace("urouge_moveset.replace_movesets=" + JSON.stringify(result));