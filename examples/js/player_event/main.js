const modules = oppw4.registry.modules().map((module) => module.name).join(", ");
oppw4.trace(`modules=[${modules}]`);

oppw4.on("sdk.runtime.player.character_changed", (ctx) => {
    oppw4.trace(`event=${ctx.eventKey} payload=${ctx.payloadJson}`);
});
