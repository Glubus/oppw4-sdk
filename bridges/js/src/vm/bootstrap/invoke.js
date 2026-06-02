    function invokeRegistry(currentMod, qualifiedName, args) {
        args.push(createCallerInfo(currentMod));
        const resultJson = globalThis.__oppw4_registry_invoke(
            qualifiedName,
            JSON.stringify(args)
        );
        return parseRegistryResult(resultJson);
    }

    function createCallerInfo(currentMod) {
        return freeze({
            __oppw4Caller: true,
            modId: currentMod.id,
            root: currentMod.root,
            zipRoot: currentMod.zipRoot,
            isZip: currentMod.isZip,
        });
    }

    function parseRegistryResult(resultJson) {
        if (resultJson === "" || resultJson === undefined || resultJson === null) {
            return undefined;
        }
        return JSON.parse(String(resultJson));
    }
