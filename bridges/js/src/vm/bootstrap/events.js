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

    function createHandlerQueryDispatcher(handlerStore, currentMod) {
        return (handlerRefs, eventKey, payloadJson) => {
            const ctx = createEventContext(eventKey, payloadJson, currentMod);
            for (const handlerRef of handlerRefs || []) {
                const callback = handlerStore[String(handlerRef)];
                if (typeof callback !== "function") {
                    throw new Error("js handler is not registered: " + handlerRef);
                }
                const result = callback(ctx);
                if (result !== undefined && result !== null) {
                    return JSON.stringify(result);
                }
            }
            return "";
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

    function registryEventStub(registryModuleList, schema, eventKey, event) {
        return freeze(function registryEventStub(callback) {
            assertFunction(callback, "event handler must be a function");
            return registerHandler(eventKey, (ctx) =>
                callTypedEventCallback(registryModuleList, schema, event, callback, ctx)
            );
        });
    }

    function callTypedEventCallback(registryModuleList, schema, event, callback, ctx) {
        if (
            String(schema.namespace) === "sdk" &&
            String(schema.importName) === "player" &&
            String(event.name) === "character_changed"
        ) {
            return callback(freeze(projectCharacterChangedContext(registryModuleList, ctx)));
        }
        if (
            String(schema.namespace) === "sdk" &&
            String(schema.importName) === "difficulty" &&
            String(event.name) === "applied"
        ) {
            return callback(freeze(projectDifficultyAppliedContext(ctx)));
        }
        if (
            String(schema.namespace) === "sdk" &&
            String(schema.importName) === "rank" &&
            String(event.name) === "result"
        ) {
            return callback(freeze(projectRankResultContext(ctx)));
        }
        if (
            String(schema.namespace) === "sdk" &&
            String(schema.importName) === "rewards" &&
            String(event.name) === "event"
        ) {
            return callback(freeze(projectRewardsEventContext(ctx)));
        }
        if (
            String(schema.namespace) === "sdk" &&
            String(schema.importName) === "rewards" &&
            String(event.name) === "medals"
        ) {
            return callback(freeze(projectRewardsItemsContext(ctx)));
        }
        if (
            String(schema.namespace) === "sdk" &&
            String(schema.importName) === "mission" &&
            String(event.name) === "rewards"
        ) {
            const typedCtx = projectMissionRewardsContext(ctx);
            callback(freeze(typedCtx));
            return {
                mutations: typedCtx.mutations.map((mutation) =>
                    freeze({
                        key: "sdk.runtime.rewards.berry.set_total",
                        payload: { total: mutation.total },
                    })
                ),
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
                    payload = wrapRegistryValue(
                        registryModuleList,
                        ctx.mod,
                        event.payload,
                        ctx.payload,
                        schema
                    );
                    wrapped = true;
                }
                return payload;
            },
        });
        return callback(freeze(typedCtx));
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
