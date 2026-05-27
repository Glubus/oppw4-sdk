import { character } from "sdk";

const target = character.find("aokiji");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for aokiji");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("kuzan_moveset.bin");

oppw4.trace("kuzan_moveset.replace_movesets=" + JSON.stringify(result));