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
        Object.defineProperty(
            value,
            methodName,
            extensionProperty(registryModuleList, currentMod, value, extension)
        );
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
                return wrapRegistryValue(
                    registryModuleList,
                    currentMod,
                    extension.method.returns,
                    result,
                    extension.schema
                );
            }
            if (extension.method.mutation) {
                const collector = currentMutationCollector();
                if (!collector) {
                    throw new Error(
                        `mutation method ${String(extension.method.name)} requires an active event dispatch`
                    );
                }
                collector.push(extensionMutationEnvelope(value, extension, args));
                return undefined;
            }
            throw new Error(
                `extension method ${String(extension.method.name)} is missing function or mutation binding`
            );
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
            throw new Error(
                `missing mutation contract for extension method ${String(extension.method.name)}`
            );
        }
        return freeze({
            key: String(mutation.key),
            payload: buildMutationPayload(target, extension, mutation, args),
        });
    }

    function mutationContractFor(extension) {
        const name = String(extension.method.mutation || "");
        return (
            (extension.schema.mutations || []).find(
                (mutation) => String(mutation.name || "") === name
            ) || null
        );
    }

    function buildMutationPayload(target, extension, mutation, args) {
        const payloadType = mutation.payload;
        if (payloadType && String(payloadType.kind || "") === "named") {
            const payloadDescriptor = namedTypeDescriptor(extension.schema, String(payloadType.name || ""));
            if (payloadDescriptor) {
                const valueFields = (payloadDescriptor.fields || []).filter(
                    (field) => String(field.name || "") !== "target"
                );
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
        return (module.schema.extensions || []).flatMap((extension) =>
            extensionMethods(extension, module.schema, targetType)
        );
    }

    function extensionMethods(extension, schema, targetType) {
        if (String(extension.targetType) !== targetType) {
            return [];
        }
        return (extension.methods || []).map((method) => ({ schema, method }));
    }
