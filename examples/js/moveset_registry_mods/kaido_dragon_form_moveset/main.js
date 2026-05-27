import { character } from "sdk";

const target = character.find("kaido_d2");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for kaido_d2");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("kaido_dragon_form_moveset.bin");

oppw4.trace("kaido_dragon_form_moveset.replace_movesets=" + JSON.stringify(result));