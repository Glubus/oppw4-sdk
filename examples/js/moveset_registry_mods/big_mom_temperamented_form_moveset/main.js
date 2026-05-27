import { character } from "sdk";

const target = character.find("linlin_temperamented");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for linlin_temperamented");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("big_mom_temperamented_form_moveset.bin");

oppw4.trace("big_mom_temperamented_form_moveset.replace_movesets=" + JSON.stringify(result));