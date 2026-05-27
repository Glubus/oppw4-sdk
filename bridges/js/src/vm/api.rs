use rquickjs::Ctx;

pub(super) fn install(ctx: Ctx<'_>) -> rquickjs::Result<()> {
    ctx.eval::<(), _>(BOOTSTRAP_JS)
}

const BOOTSTRAP_JS: &str = r#"
(() => {
    const handlers = Object.create(null);

    const defineHidden = (name, value) => {
        Object.defineProperty(globalThis, name, {
            value,
            configurable: false,
            enumerable: false,
            writable: false,
        });
    };

    const mod = Object.freeze({
        id: String(globalThis.__oppw4_mod_id),
        name: String(globalThis.__oppw4_mod_name),
        root: String(globalThis.__oppw4_mod_root),
        zipRoot: String(globalThis.__oppw4_mod_zip_root),
        isZip: Boolean(globalThis.__oppw4_mod_is_zip),
    });

    const registerHandler = (eventKey, callback) => {
        if (typeof callback !== "function") {
            throw new TypeError("event handler must be a function");
        }
        const handlerRef = globalThis.__oppw4_register_handler_ref(String(eventKey));
        handlers[handlerRef] = callback;
        return handlerRef;
    };

    const parsePayload = (payloadJson) => {
        if (payloadJson === "" || payloadJson === undefined || payloadJson === null) {
            return null;
        }
        return JSON.parse(String(payloadJson));
    };

    const dispatchHandler = (handlerRef, eventKey, payloadJson) => {
        const callback = handlers[String(handlerRef)];
        if (typeof callback !== "function") {
            throw new Error("js handler is not registered: " + handlerRef);
        }
        const ctx = Object.freeze({
            eventKey: String(eventKey),
            payloadJson: String(payloadJson),
            payload: parsePayload(payloadJson),
            mod,
        });
        return callback(ctx);
    };

    const events = Object.freeze({
        on: registerHandler,
    });

    const registryModules = () => {
        const json = String(globalThis.__oppw4_registry_modules_json || "[]");
        return JSON.parse(json).map((module) => Object.freeze({
            providerId: String(module.providerId),
            name: String(module.name),
            load: String(module.load),
            schema: module.schema ? Object.freeze(module.schema) : null,
        }));
    };

    const modules = registryModules();
    const namespaceRoot = Object.create(null);

    const namedType = (schema, name) => {
        const raw = String(name);
        return raw.includes(".") ? raw : `${String(schema.namespace)}.${raw}`;
    };

    const invokeRegistry = (qualifiedName, args) => {
        args.push(Object.freeze({
            __oppw4Caller: true,
            modId: mod.id,
            root: mod.root,
            zipRoot: mod.zipRoot,
            isZip: mod.isZip,
        }));
        const resultJson = globalThis.__oppw4_registry_invoke(
            qualifiedName,
            JSON.stringify(args),
        );
        if (resultJson === "" || resultJson === undefined || resultJson === null) {
            return undefined;
        }
        return JSON.parse(String(resultJson));
    };

    const extensionMethodsFor = (targetType) => {
        const methods = [];
        for (const module of modules) {
            const schema = module.schema;
            if (!schema) {
                continue;
            }
            for (const extension of schema.extensions || []) {
                if (String(extension.targetType) !== targetType) {
                    continue;
                }
                for (const method of extension.methods || []) {
                    methods.push({ schema, method });
                }
            }
        }
        return methods;
    };

    const wrapRegistryValue = (typeRef, value, schema) => {
        if (value == null || !typeRef) {
            return value;
        }
        const kind = String(typeRef.kind || "");
        if (kind === "optional") {
            return wrapRegistryValue(typeRef.inner, value, schema);
        }
        if (kind === "array") {
            if (!Array.isArray(value)) {
                return value;
            }
            return Object.freeze(value.map((item) => wrapRegistryValue(typeRef.inner, item, schema)));
        }
        if (kind !== "named" || typeof value !== "object") {
            return value;
        }
        const targetType = namedType(schema, typeRef.name);
        Object.defineProperty(value, "__oppw4Type", {
            value: targetType,
            configurable: false,
            enumerable: false,
            writable: false,
        });
        for (const { schema: extensionSchema, method } of extensionMethodsFor(targetType)) {
            const methodName = String(method.name || "");
            const functionName = String(method.function || "");
            if (!methodName || !functionName || Object.prototype.hasOwnProperty.call(value, methodName)) {
                continue;
            }
            Object.defineProperty(value, methodName, {
                value: Object.freeze(function registryExtensionMethod(...args) {
                    const qualifiedName = `${String(extensionSchema.namespace)}.${String(extensionSchema.importName)}.${functionName}`;
                    const result = invokeRegistry(qualifiedName, [value, ...args]);
                    return wrapRegistryValue(method.returns, result, extensionSchema);
                }),
                configurable: false,
                enumerable: false,
                writable: false,
            });
        }
        return Object.freeze(value);
    };

    const registryFunctionStub = (qualifiedName, returnType, schema) => {
        return Object.freeze(function registryFunctionStub(...args) {
            const result = invokeRegistry(qualifiedName, args);
            return wrapRegistryValue(returnType, result, schema);
        });
    };

    const installSchemaModule = (module) => {
        const schema = module.schema;
        if (!schema) {
            return;
        }
        const namespace = String(schema.namespace);
        const importName = String(schema.importName);
        if (!namespace || !importName) {
            return;
        }
        const namespaceObject = namespaceRoot[namespace] || Object.create(null);
        namespaceRoot[namespace] = namespaceObject;

        const moduleObject = Object.create(null);
        for (const fn of schema.functions || []) {
            const name = String(fn.name);
            if (!name) {
                continue;
            }
            moduleObject[name] = registryFunctionStub(`${namespace}.${importName}.${name}`, fn.returns, schema);
        }
        Object.defineProperty(moduleObject, "__schema", {
            value: schema,
            configurable: false,
            enumerable: false,
            writable: false,
        });
        namespaceObject[importName] = Object.freeze(moduleObject);
    };

    for (const module of modules) {
        installSchemaModule(module);
    }
    for (const [namespace, value] of Object.entries(namespaceRoot)) {
        defineHidden(namespace, Object.freeze(value));
    }

    const lookupPath = (path) => {
        const parts = String(path).split(".").filter(Boolean);
        let current = globalThis;
        for (const part of parts) {
            if (current == null || !Object.prototype.hasOwnProperty.call(current, part)) {
                return null;
            }
            current = current[part];
        }
        return current ?? null;
    };

    const registry = Object.freeze({
        modules() {
            return Object.freeze(registryModules());
        },
        has(name) {
            return this.module(name) !== null;
        },
        module(name) {
            return lookupPath(name);
        },
    });

    const oppw4 = Object.freeze({
        mod,
        events,
        registry,
        on: registerHandler,
        trace(message) {
            return globalThis.__oppw4_trace(String(message));
        },
    });

    defineHidden("__oppw4_handlers", handlers);
    defineHidden("__oppw4_register_handler", registerHandler);
    defineHidden("__oppw4_dispatch_handler", dispatchHandler);
    defineHidden("oppw4", oppw4);
})();
"#;
