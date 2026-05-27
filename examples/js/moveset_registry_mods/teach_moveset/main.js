import { character } from "sdk";

const target = character.find("teach");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for teach");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("teach_moveset.bin");

oppw4.trace("teach_moveset.replace_movesets=" + JSON.stringify(result));