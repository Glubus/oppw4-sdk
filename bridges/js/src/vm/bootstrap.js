(() => {
    const freeze = Object.freeze;
    const handlers = Object.create(null);
    const activeMutationCollectors = [];
    const mod = createModInfo();
    const modules = registryModules();
    const namespaces = installSchemaModules(modules, mod);
    const registerHandler = createHandlerRegistrar(handlers);

    exposeNamespaces(namespaces);
    installGlobalApi({
        handlers,
        mod,
        registerHandler,
        registry: createRegistry(),
        dispatchHandler: createHandlerDispatcher(handlers, mod),
    });

    function defineHidden(name, value) {
        Object.defineProperty(globalThis, name, {
            value,
            configurable: false,
            enumerable: false,
            writable: false,
        });
    }

    function createModInfo() {
        return freeze({
            id: String(globalThis.__oppw4_mod_id),
            name: String(globalThis.__oppw4_mod_name),
            root: String(globalThis.__oppw4_mod_root),
            zipRoot: String(globalThis.__oppw4_mod_zip_root),
            isZip: Boolean(globalThis.__oppw4_mod_is_zip),
        });
    }

    function createHandlerRegistrar(handlerStore) {
        return (eventKey, callback) => {
            assertFunction(callback, "event handler must be a function");
            const key = String(eventKey);
            const handlerRef = globalThis.__oppw4_register_handler_ref(key);
            handlerStore[handlerRef] = callback;
            return handlerRef;
        };
    }

    function createHandlerDispatcher(handlerStore, currentMod) {
        return (handlerRefs, eventKey, payloadJson) => {
            const ctx = createEventContext(eventKey, payloadJson, currentMod);
            const mutations = [];
            for (const handlerRef of handlerRefs || []) {
                const callback = handlerStore[String(handlerRef)];
                if (typeof callback !== "function") {
                    throw new Error("js handler is not registered: " + handlerRef);
                }
                const result = withMutationCollector(mutations, () => callback(ctx));
                collectMutations(mutations, result);
            }
            return JSON.stringify(mutations);
        };
    }

    function withMutationCollector(target, callback) {
        activeMutationCollectors.push(target);
        try {
            return callback();
        } finally {
            activeMutationCollectors.pop();
        }
    }

    function currentMutationCollector() {
        if (activeMutationCollectors.length === 0) {
            return null;
        }
        return activeMutationCollectors[activeMutationCollectors.length - 1];
    }

    function collectMutations(target, result) {
        if (!result || !Array.isArray(result.mutations)) {
            return;
        }
        for (const mutation of result.mutations) {
            if (mutation && mutation.key && mutation.payload !== undefined) {
                target.push({
                    key: String(mutation.key),
                    payload: mutation.payload,
                });
            }
        }
    }

    function createEventContext(eventKey, payloadJson, currentMod) {
        const json = String(payloadJson);
        let parsed = false;
        let payload = null;
        const ctx = {
            eventKey: String(eventKey),
            payloadJson: json,
            mod: currentMod,
        };
        Object.defineProperty(ctx, "payload", {
            enumerable: true,
            get() {
                if (!parsed) {
                    payload = parsePayload(json);
                    parsed = true;
                }
                return payload;
            },
        });
        return freeze(ctx);
    }

    function parsePayload(payloadJson) {
        if (payloadJson === "" || payloadJson === undefined || payloadJson === null) {
            return null;
        }
        return JSON.parse(String(payloadJson));
    }

    function registryModules() {
        const json = String(globalThis.__oppw4_registry_modules_json || "[]");
        return JSON.parse(json).map(normalizeRegistryModule);
    }

    function normalizeRegistryModule(module) {
        return freeze({
            providerId: String(module.providerId),
            name: String(module.name),
            load: String(module.load),
            schema: module.schema ? freeze(module.schema) : null,
        });
    }

    function installSchemaModules(registryModuleList, currentMod) {
        const namespaces = Object.create(null);
        for (const module of registryModuleList) {
            installSchemaModule(namespaces, registryModuleList, currentMod, module);
        }
        return namespaces;
    }

    function installSchemaModule(namespaces, registryModuleList, currentMod, module) {
        const schema = module.schema;
        if (!isUsableSchema(schema)) {
            return;
        }
        const namespace = String(schema.namespace);
        const importName = String(schema.importName);
        const moduleObject = createSchemaModule(registryModuleList, currentMod, schema);
        const namespaceObject = namespaces[namespace] || Object.create(null);
        namespaceObject[importName] = freeze(moduleObject);
        namespaces[namespace] = namespaceObject;
    }

    function isUsableSchema(schema) {
        return schema && String(schema.namespace) && String(schema.importName);
    }

    function createSchemaModule(registryModuleList, currentMod, schema) {
        const moduleObject = Object.create(null);
        installSchemaFunctions(moduleObject, registryModuleList, currentMod, schema);
        installSchemaEvents(moduleObject, registryModuleList, currentMod, schema);
        defineSchema(moduleObject, schema);
        return moduleObject;
    }

    function installSchemaFunctions(moduleObject, registryModuleList, currentMod, schema) {
        for (const fn of schema.functions || []) {
            const name = String(fn.name);
            if (name) {
                moduleObject[name] = registryFunctionStub(registryModuleList, currentMod, schema, fn);
            }
        }
    }

    function installSchemaEvents(moduleObject, registryModuleList, currentMod, schema) {
        for (const event of schema.events || []) {
            const name = String(event.name || "");
            const key = String(event.key || "");
            if (name && key) {
                moduleObject[`on_${name}`] = registryEventStub(registryModuleList, schema, key, event);
            }
        }
    }

    function defineSchema(moduleObject, schema) {
        Object.defineProperty(moduleObject, "__schema", {
            value: schema,
            configurable: false,
            enumerable: false,
            writable: false,
        });
    }

    function registryFunctionStub(registryModuleList, currentMod, schema, fn) {
        const qualifiedName = `${schema.namespace}.${schema.importName}.${String(fn.name)}`;
        return freeze(function registryFunctionStub(...args) {
            const result = invokeRegistry(currentMod, qualifiedName, args);
            return wrapRegistryValue(registryModuleList, currentMod, fn.returns, result, schema);
        });
    }

    function registryEventStub(registryModuleList, schema, eventKey, event) {
        return freeze(function registryEventStub(callback) {
            assertFunction(callback, "event handler must be a function");
            return registerHandler(eventKey, (ctx) => callTypedEventCallback(registryModuleList, schema, event, callback, ctx));
        });
    }

    function callTypedEventCallback(registryModuleList, schema, event, callback, ctx) {
        if (String(schema.namespace) === "sdk" &&
            String(schema.importName) === "player" &&
            String(event.name) === "character_changed") {
            return callback(freeze(projectCharacterChangedContext(registryModuleList, ctx)));
        }
        if (String(schema.namespace) === "sdk" &&
            String(schema.importName) === "difficulty" &&
            String(event.name) === "applied") {
            return callback(freeze(projectDifficultyAppliedContext(ctx)));
        }
        if (String(schema.namespace) === "sdk" &&
            String(schema.importName) === "rank" &&
            String(event.name) === "result") {
            return callback(freeze(projectRankResultContext(ctx)));
        }
        if (String(schema.namespace) === "sdk" &&
            String(schema.importName) === "rewards" &&
            String(event.name) === "event") {
            return callback(freeze(projectRewardsEventContext(ctx)));
        }
        if (String(schema.namespace) === "sdk" &&
            String(schema.importName) === "rewards" &&
            String(event.name) === "medals") {
            return callback(freeze(projectRewardsItemsContext(ctx)));
        }
        if (String(schema.namespace) === "sdk" &&
            String(schema.importName) === "mission" &&
            String(event.name) === "rewards") {
            const typedCtx = projectMissionRewardsContext(ctx);
            callback(freeze(typedCtx));
            return {
                mutations: typedCtx.mutations.map((mutation) => freeze({
                    key: "sdk.runtime.rewards.berry.set_total",
                    payload: { total: mutation.total },
                })),
            };
        }
        let wrapped = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(typedCtx, "payload", {
            enumerable: true,
            get() {
                if (!wrapped) {
                    payload = wrapRegistryValue(registryModuleList, ctx.mod, event.payload, ctx.payload, schema);
                    wrapped = true;
                }
                return payload;
            },
        });
        return callback(freeze(typedCtx));
    }

    function projectCharacterChangedContext(registryModuleList, ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(typedCtx, "payload", {
            enumerable: true,
            get() {
                if (!payloadLoaded) {
                    payload = ctx.payload;
                    payloadLoaded = true;
                }
                return payload;
            },
        });
        Object.defineProperty(typedCtx, "previous_character", {
            enumerable: true,
            get() {
                const eventPayload = typedCtx.payload || {};
                const characterId = eventPayload.previous_character_id;
                return characterId ? resolveCharacter(registryModuleList, ctx.mod, characterId) : null;
            },
        });
        Object.defineProperty(typedCtx, "current_character", {
            enumerable: true,
            get() {
                const eventPayload = typedCtx.payload || {};
                const characterId = eventPayload.current_character_id;
                return characterId ? resolveCharacter(registryModuleList, ctx.mod, characterId) : null;
            },
        });
        Object.defineProperty(typedCtx, "active_character_ids", {
            enumerable: true,
            get() {
                const eventPayload = typedCtx.payload || {};
                const ids = eventPayload.active_character_ids;
                return Array.isArray(ids) ? freeze(ids.slice()) : freeze([]);
            },
        });
        return typedCtx;
    }

    function projectDifficultyAppliedContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(typedCtx, "payload", payloadProperty(() => {
            if (!payloadLoaded) {
                payload = ctx.payload || {};
                payloadLoaded = true;
            }
            return payload;
        }));
        Object.defineProperty(typedCtx, "mission_id", valueProperty(() => typedCtx.payload.mission_id ?? null));
        Object.defineProperty(typedCtx, "mode", valueProperty(() => typedCtx.payload.mode ?? null));
        Object.defineProperty(typedCtx, "difficulty", valueProperty(() => typedCtx.payload.difficulty ?? null));
        return typedCtx;
    }

    function projectRankResultContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(typedCtx, "payload", payloadProperty(() => {
            if (!payloadLoaded) {
                payload = ctx.payload || {};
                payloadLoaded = true;
            }
            return payload;
        }));
        Object.defineProperty(typedCtx, "rank", valueProperty(() => freeze({
            final: typedCtx.payload.rank ?? "unknown",
            count: typedCtx.payload.count ?? null,
            time: typedCtx.payload.time ?? null,
            merge: typedCtx.payload.merge ?? null,
        })));
        Object.defineProperty(typedCtx, "mission", valueProperty(() => freeze({
            mission_id: typedCtx.payload.mission_id ?? null,
            mode: typedCtx.payload.mode ?? null,
            difficulty: typedCtx.payload.difficulty ?? null,
        })));
        return typedCtx;
    }

    function projectRewardsEventContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(typedCtx, "payload", payloadProperty(() => {
            if (!payloadLoaded) {
                payload = ctx.payload || {};
                payloadLoaded = true;
            }
            return payload;
        }));
        Object.defineProperty(typedCtx, "rank", valueProperty(() => typedCtx.payload.rank ?? null));
        Object.defineProperty(typedCtx, "berry", valueProperty(() => typedCtx.payload.berry ?? null));
        Object.defineProperty(typedCtx, "souls", valueProperty(() => freeze([])));
        Object.defineProperty(typedCtx, "crew_points", valueProperty(() => typedCtx.payload.crew_points ?? null));
        Object.defineProperty(typedCtx, "medals", valueProperty(() => {
            const medals = typedCtx.payload.medals;
            return Array.isArray(medals) ? freeze(medals.slice()) : freeze([]);
        }));
        Object.defineProperty(typedCtx, "ranks", valueProperty(() => {
            const ranks = [
                typedCtx.payload.count,
                typedCtx.payload.time,
                typedCtx.payload.merge,
                typedCtx.payload.rank,
            ].filter((value) => value != null);
            return freeze(ranks);
        }));
        return typedCtx;
    }

    function projectRewardsItemsContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
        };
        Object.defineProperty(typedCtx, "payload", payloadProperty(() => {
            if (!payloadLoaded) {
                payload = ctx.payload || {};
                payloadLoaded = true;
            }
            return payload;
        }));
        Object.defineProperty(typedCtx, "entries", valueProperty(() => {
            const entries = typedCtx.payload.entries;
            return Array.isArray(entries) ? freeze(entries.slice()) : freeze([]);
        }));
        return typedCtx;
    }

    function projectMissionRewardsContext(ctx) {
        let payloadLoaded = false;
        let payload = null;
        const mutations = [];
        const typedCtx = {
            eventKey: ctx.eventKey,
            payloadJson: ctx.payloadJson,
            mod: ctx.mod,
            mutations,
        };
        Object.defineProperty(typedCtx, "payload", payloadProperty(() => {
            if (!payloadLoaded) {
                payload = ctx.payload || {};
                payloadLoaded = true;
            }
            return payload;
        }));
        Object.defineProperty(typedCtx, "rank", valueProperty(() => typedCtx.payload.rank ?? null));
        Object.defineProperty(typedCtx, "rewards", valueProperty(() => createMissionRewardsView(typedCtx, mutations)));
        return typedCtx;
    }

    function createMissionRewardsView(ctx, mutations) {
        const berry = createBerryRewardView(ctx, mutations);
        return {
            berry,
            medals: Array.isArray(ctx.payload.medals) ? freeze(ctx.payload.medals.slice()) : freeze([]),
            crew_points: ctx.payload.crew_points ?? null,
        };
    }

    function createBerryRewardView(ctx, mutations) {
        let total = Number(ctx.payload.berry ?? 0);
        return {
            get total() {
                return total;
            },
            set_total(value) {
                const next = Number(value);
                if (!Number.isFinite(next) || next < 0) {
                    throw new Error("berry total must be a non-negative finite number");
                }
                total = Math.trunc(next);
                ctx.payload.berry = total;
                invokeRegistry(ctx.mod, "sdk.mission.set_reward_berry_total", [total]);
                mutations.push(freeze({
                    kind: "berry.set_total",
                    total,
                }));
                return total;
            },
        };
    }

    function payloadProperty(getter) {
        return {
            enumerable: true,
            get: getter,
        };
    }

    function valueProperty(getter) {
        return {
            enumerable: true,
            get: getter,
        };
    }

    function resolveCharacter(registryModuleList, currentMod, characterId) {
        const module = lookupPath("sdk.character");
        if (!module || typeof module.find !== "function") {
            return null;
        }
        const value = module.find(String(characterId));
        return wrapRegistryValue(
            registryModuleList,
            currentMod,
            { kind: "named", name: "sdk.Character" },
            value,
            { namespace: "sdk", importName: "character" },
        );
    }

    function invokeRegistry(currentMod, qualifiedName, args) {
        args.push(createCallerInfo(currentMod));
        const resultJson = globalThis.__oppw4_registry_invoke(
            qualifiedName,
            JSON.stringify(args),
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

    function wrapRegistryValue(registryModuleList, currentMod, typeRef, value, schema) {
        if (value == null || !typeRef) {
            return value;
        }
        const kind = String(typeRef.kind || "");
        return wrapRegistryValueByKind(registryModuleList, currentMod, kind, typeRef, value, schema);
    }

    function wrapRegistryValueByKind(registryModuleList, currentMod, kind, typeRef, value, schema) {
        if (kind === "optional") {
            return wrapRegistryValue(registryModuleList, currentMod, typeRef.inner, value, schema);
        }
        if (kind === "array") {
            return wrapRegistryArray(registryModuleList, currentMod, typeRef, value, schema);
        }
        return wrapNamedRegistryValue(registryModuleList, currentMod, kind, typeRef, value, schema);
    }

    function wrapRegistryArray(registryModuleList, currentMod, typeRef, value, schema) {
        if (!Array.isArray(value)) {
            return value;
        }
        return freeze(value.map((item) => wrapRegistryValue(registryModuleList, currentMod, typeRef.inner, item, schema)));
    }

    function wrapNamedRegistryValue(registryModuleList, currentMod, kind, typeRef, value, schema) {
        if (kind !== "named" || typeof value !== "object") {
            return value;
        }
        const targetType = namedType(schema, typeRef.name);
        defineValueType(value, targetType);
        installExtensionMethods(registryModuleList, currentMod, value, targetType);
        return freeze(value);
    }

    function defineValueType(value, targetType) {
        if (Object.prototype.hasOwnProperty.call(value, "__oppw4Type")) {
            return;
        }
        Object.defineProperty(value, "__oppw4Type", {
            value: targetType,
            configurable: false,
            enumerable: false,
            writable: false,
        });
    }

    function installExtensionMethods(registryModuleList, currentMod, value, targetType) {
        for (const extension of extensionMethodsFor(registryModuleList, targetType)) {
            installExtensionMethod(registryModuleList, currentMod, value, extension);
        }
    }

    function installExtensionMethod(registryModuleList, currentMod, value, extension) {
        const methodName = String(extension.method.name || "");
        if (!canInstallMethod(value, methodName, extensionMethodKind(extension.method))) {
            return;
        }
        Object.defineProperty(value, methodName, extensionProperty(registryModuleList, currentMod, value, extension));
    }

    function extensionProperty(registryModuleList, currentMod, value, extension) {
        return {
            value: extensionFunction(registryModuleList, currentMod, value, extension),
            configurable: false,
            enumerable: false,
            writable: false,
        };
    }

    function extensionFunction(registryModuleList, currentMod, value, extension) {
        return freeze(function registryExtensionMethod(...args) {
            if (extension.method.function) {
                const name = extensionQualifiedName(extension);
                const result = invokeRegistry(currentMod, name, [value, ...args]);
                return wrapRegistryValue(registryModuleList, currentMod, extension.method.returns, result, extension.schema);
            }
            if (extension.method.mutation) {
                const collector = currentMutationCollector();
                if (!collector) {
                    throw new Error(`mutation method ${String(extension.method.name)} requires an active event dispatch`);
                }
                collector.push(extensionMutationEnvelope(value, extension, args));
                return undefined;
            }
            throw new Error(`extension method ${String(extension.method.name)} is missing function or mutation binding`);
        });
    }

    function extensionQualifiedName(extension) {
        return `${extension.schema.namespace}.${extension.schema.importName}.${String(extension.method.function)}`;
    }

    function extensionMethodKind(method) {
        if (method.function) {
            return "function";
        }
        if (method.mutation) {
            return "mutation";
        }
        return "";
    }

    function canInstallMethod(value, methodName, kind) {
        return methodName && kind && !Object.prototype.hasOwnProperty.call(value, methodName);
    }

    function extensionMutationEnvelope(target, extension, args) {
        const mutation = mutationContractFor(extension);
        if (!mutation || !mutation.key) {
            throw new Error(`missing mutation contract for extension method ${String(extension.method.name)}`);
        }
        return freeze({
            key: String(mutation.key),
            payload: buildMutationPayload(target, extension, mutation, args),
        });
    }

    function mutationContractFor(extension) {
        const name = String(extension.method.mutation || "");
        return (extension.schema.mutations || []).find((mutation) => String(mutation.name || "") === name) || null;
    }

    function buildMutationPayload(target, extension, mutation, args) {
        const payloadType = mutation.payload;
        if (payloadType && String(payloadType.kind || "") === "named") {
            const payloadDescriptor = namedTypeDescriptor(extension.schema, String(payloadType.name || ""));
            if (payloadDescriptor) {
                const valueFields = (payloadDescriptor.fields || []).filter((field) => String(field.name || "") !== "target");
                if (valueFields.length === 1) {
                    return {
                        target,
                        [String(valueFields[0].name || "value")]: args[0],
                    };
                }
            }
        }
        if (args.length === 1 && args[0] && typeof args[0] === "object" && !Array.isArray(args[0])) {
            return args[0];
        }
        throw new Error(`cannot build payload for mutation method ${String(extension.method.name)}`);
    }

    function extensionMethodsFor(registryModuleList, targetType) {
        return registryModuleList.flatMap((module) => extensionMethodsInModule(module, targetType));
    }

    function extensionMethodsInModule(module, targetType) {
        if (!module.schema) {
            return [];
        }
        return (module.schema.extensions || []).flatMap((extension) => extensionMethods(extension, module.schema, targetType));
    }

    function extensionMethods(extension, schema, targetType) {
        if (String(extension.targetType) !== targetType) {
            return [];
        }
        return (extension.methods || []).map((method) => ({ schema, method }));
    }

    function namedType(schema, name) {
        const raw = String(name);
        return raw.includes(".") ? raw : `${String(schema.namespace)}.${raw}`;
    }

    function namedTypeDescriptor(schema, name) {
        const raw = String(name);
        return (schema.types || []).find((type) =>
            String(type.name || "") === raw ||
            `${String(schema.namespace)}.${String(type.name || "")}` === raw
        ) || null;
    }

    function createRegistry() {
        return freeze({
            modules: () => freeze(registryModules()),
            has(name) {
                return this.module(name) !== null;
            },
            module: lookupPath,
        });
    }

    function lookupPath(path) {
        const parts = String(path).split(".").filter(Boolean);
        let current = globalThis;
        for (const part of parts) {
            current = lookupPart(current, part);
            if (current === null) {
                return null;
            }
        }
        return current ?? null;
    }

    function lookupPart(current, part) {
        if (current == null || !Object.prototype.hasOwnProperty.call(current, part)) {
            return null;
        }
        return current[part];
    }

    function exposeNamespaces(namespaces) {
        for (const [namespace, value] of Object.entries(namespaces)) {
            defineHidden(namespace, freeze(value));
        }
    }

    function installGlobalApi(api) {
        defineHidden("__oppw4_handlers", api.handlers);
        defineHidden("__oppw4_register_handler", api.registerHandler);
        defineHidden("__oppw4_dispatch_handlers", api.dispatchHandler);
        defineHidden("__oppw4_dispatch_handler", (handlerRef, eventKey, payloadJson) => {
            return api.dispatchHandler([handlerRef], eventKey, payloadJson);
        });
        defineHidden("oppw4", createOppw4Api(api));
    }

    function createOppw4Api(api) {
        return freeze({
            mod: api.mod,
            events: freeze({ on: api.registerHandler }),
            registry: api.registry,
            on: api.registerHandler,
            trace: (message) => globalThis.__oppw4_trace(String(message)),
        });
    }

    function assertFunction(value, message) {
        if (typeof value !== "function") {
            throw new TypeError(message);
        }
    }
})();
