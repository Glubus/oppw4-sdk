import { character } from "sdk";

const target = character.find("kaido");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for kaido");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("kaido_moveset.bin");

oppw4.trace("kaido_moveset.replace_movesets=" + JSON.stringify(result));