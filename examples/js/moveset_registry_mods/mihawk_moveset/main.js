import { character } from "sdk";

const target = character.find("mihawk");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for mihawk");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("mihawk_moveset.bin");

oppw4.trace("mihawk_moveset.replace_movesets=" + JSON.stringify(result));