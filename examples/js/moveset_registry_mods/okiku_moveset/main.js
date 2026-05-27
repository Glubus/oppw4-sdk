import { character } from "sdk";

const target = character.find("kiku");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for kiku");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("okiku_moveset.bin");

oppw4.trace("okiku_moveset.replace_movesets=" + JSON.stringify(result));