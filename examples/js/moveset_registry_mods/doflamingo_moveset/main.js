import { character } from "sdk";

const target = character.find("doflamingo");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for doflamingo");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("doflamingo_moveset.bin");

oppw4.trace("doflamingo_moveset.replace_movesets=" + JSON.stringify(result));