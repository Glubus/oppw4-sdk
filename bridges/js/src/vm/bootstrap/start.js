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
        queryHandler: createHandlerQueryDispatcher(handlers, mod),
    });
