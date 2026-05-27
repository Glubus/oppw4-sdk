import { character } from "sdk";

const target = character.find("shanks");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for shanks");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("shanks_moveset.bin");

oppw4.trace("shanks_moveset.replace_movesets=" + JSON.stringify(result));