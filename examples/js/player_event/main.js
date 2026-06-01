import { player } from "sdk";

oppw4.trace(`player.on_character_changed=${typeof player.on_character_changed}`);

player.on_character_changed((ctx) => {
    oppw4.trace(`player changed current=${ctx.current_character?.id ?? "none"}`);
});
