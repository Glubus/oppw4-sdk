import { character } from "sdk";
import { patch } from "moveset";

const ace = character.find("ace");
if (!ace || ace.movesetLinkdataEntry == null) {
    throw new Error("Ace moveset entry is missing from sdk.character");
}

const result = patch.replace({
    entry: ace.movesetLinkdataEntry,
    payloadFile: "ace_moveset.bin",
});

oppw4.trace(`ace.moveset.replace=${JSON.stringify(result)}`);
