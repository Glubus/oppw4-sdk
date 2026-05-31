import { player } from "sdk";

player.on_character_changed((ctx) => {
    oppw4.trace(`analyzer-test player payload=${ctx.payloadJson}`);
});
