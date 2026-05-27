import { character } from "sdk";

const target = character.find("luffy_snakeman");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for luffy_snakeman");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("luffy_snakeman_moveset.bin");

oppw4.trace("luffy_snakeman_moveset.replace_movesets=" + JSON.stringify(result));