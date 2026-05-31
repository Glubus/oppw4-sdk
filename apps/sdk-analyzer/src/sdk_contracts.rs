use sdk_bridge::{
    RegistryMethodDescriptor, RegistryModuleDescriptor, RegistryModuleSchema,
    RegistryTypeExtensionDescriptor, RegistryTypeRef,
};

pub(crate) fn method_modules(methods: &[String]) -> Vec<RegistryModuleDescriptor> {
    if methods.is_empty() {
        return Vec::new();
    }
    let mut extension = RegistryTypeExtensionDescriptor::new("sdk.Character");
    for method in methods {
        extension = extension.method(RegistryMethodDescriptor::new(
            method,
            method,
            RegistryTypeRef::Json,
        ));
    }
    vec![
        RegistryModuleDescriptor::builder("standalone", "sdk.character")
            .schema(RegistryModuleSchema::new("sdk", "character").extension(extension))
            .build(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_modules_declare_requested_methods() {
        let modules = method_modules(&["replace_costume".to_string()]);

        assert!(sdk_bridge::registry_declares_method(
            &modules,
            "replace_costume"
        ));
        assert!(!sdk_bridge::registry_declares_method(
            &modules,
            "replace_movesets"
        ));
    }
}
