import { character } from "sdk";

const target = character.find("akainu");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for akainu");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("akainu_moveset.bin");

oppw4.trace("akainu_moveset.replace_movesets=" + JSON.stringify(result));