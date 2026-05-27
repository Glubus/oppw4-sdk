import { character } from "sdk";

const whitebeard = character.find("newgate");
if (!whitebeard || whitebeard.movesetLinkdataEntry == null) {
    throw new Error("Whitebeard moveset entry is missing from sdk.character");
}
if (typeof whitebeard.replace_movesets !== "function") {
    throw new Error("Character.replace_movesets extension is missing");
}

const result = whitebeard.replace_movesets("whitebeard_moveset.bin");

oppw4.trace(`whitebeard.moveset.replace=${JSON.stringify(result)}`);
