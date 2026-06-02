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
        const generatedModule = generatedCreateSchemaModule(
            registryModuleList,
            currentMod,
            schema
        );
        if (generatedModule) {
            return generatedModule;
        }
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
                moduleObject[name] = registryFunctionStub(
                    registryModuleList,
                    currentMod,
                    schema,
                    fn
                );
            }
        }
    }

    function installSchemaEvents(moduleObject, registryModuleList, currentMod, schema) {
        for (const event of schema.events || []) {
            const name = String(event.name || "");
            const key = String(event.key || "");
            if (name && key) {
                moduleObject[`on_${name}`] = registryEventStub(
                    registryModuleList,
                    schema,
                    key,
                    event
                );
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
