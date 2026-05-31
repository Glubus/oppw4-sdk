import "./events/player.js";

const character = sdk.character.get("luffy");
character.replace_costume("default", {
    textures: {
        body: "analyzer-test-body.g1t",
    },
});

oppw4.trace("analyzer-test loaded");
