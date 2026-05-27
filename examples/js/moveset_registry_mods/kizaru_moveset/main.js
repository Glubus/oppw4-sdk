import { character } from "sdk";

const target = character.find("kizaru");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for kizaru");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("kizaru_moveset.bin");

oppw4.trace("kizaru_moveset.replace_movesets=" + JSON.stringify(result));