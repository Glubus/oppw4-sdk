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

    function installGlobalApi(api) {
        defineHidden("__oppw4_handlers", api.handlers);
        defineHidden("__oppw4_register_handler", api.registerHandler);
        defineHidden("__oppw4_dispatch_handlers", api.dispatchHandler);
        defineHidden("__oppw4_query_handlers", api.queryHandler);
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
