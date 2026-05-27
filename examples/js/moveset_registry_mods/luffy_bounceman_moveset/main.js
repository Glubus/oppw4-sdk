import { character } from "sdk";

const target = character.find("luffy_bounceman");
if (!target || target.movesetLinkdataEntry == null) {
    throw new Error("Moveset entry is missing for luffy_bounceman");
}
if (typeof target.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = target.replace_movesets("luffy_bounceman_moveset.bin");

oppw4.trace("luffy_bounceman_moveset.replace_movesets=" + JSON.stringify(result));