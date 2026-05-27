import { character } from "sdk";

const target = character.find("fujitora");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for fujitora");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("fujitora_moveset.bin");

oppw4.trace("fujitora_moveset.replace_movesets=" + JSON.stringify(result));