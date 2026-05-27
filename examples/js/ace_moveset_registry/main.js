import { character } from "sdk";

const ace = character.find("ace");
if (!ace || ace.movesetLinkdataEntry == null) {
    throw new Error("Ace moveset entry is missing from sdk.character");
}
if (typeof ace.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = ace.replace_movesets("ace_moveset.bin");

oppw4.trace(`ace.moveset.replace=${JSON.stringify(result)}`);
