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
        return freeze(
            value.map((item) =>
                wrapRegistryValue(registryModuleList, currentMod, typeRef.inner, item, schema)
            )
        );
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

    function namedType(schema, name) {
        const raw = String(name);
        return raw.includes(".") ? raw : `${String(schema.namespace)}.${raw}`;
    }

    function namedTypeDescriptor(schema, name) {
        const raw = String(name);
        return (
            (schema.types || []).find(
                (type) =>
                    String(type.name || "") === raw ||
                    `${String(schema.namespace)}.${String(type.name || "")}` === raw
            ) || null
        );
    }
