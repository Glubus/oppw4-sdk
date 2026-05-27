import { character } from "sdk";

const zoro = character.find("zoro");
const bounceman = character.find("bounceman");

oppw4.trace(`character.zoro=${JSON.stringify(zoro)}`);
oppw4.trace(`character.bounceman=${JSON.stringify(bounceman)}`);

if (!zoro || zoro.id !== "zoro" || zoro.movesetLinkdataEntry !== 69) {
    throw new Error(`bad zoro registry result: ${JSON.stringify(zoro)}`);
}

if (!bounceman || bounceman.id !== "luffy_bounceman" || bounceman.movesetLinkdataEntry !== 208) {
    throw new Error(`bad bounceman registry result: ${JSON.stringify(bounceman)}`);
}
