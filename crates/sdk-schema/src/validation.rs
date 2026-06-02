use crate::{RegistryModuleSchema, RegistryTypeRef};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistrySchemaError {
    DuplicateName { kind: &'static str, name: String },
    MissingNamedType { name: String },
}

impl std::fmt::Display for RegistrySchemaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateName { kind, name } => {
                write!(formatter, "duplicate registry {kind} name: {name}")
            }
            Self::MissingNamedType { name } => {
                write!(formatter, "registry named type is not declared: {name}")
            }
        }
    }
}

impl std::error::Error for RegistrySchemaError {}

pub(crate) fn validate_schema(schema: &RegistryModuleSchema) -> Result<(), RegistrySchemaError> {
    validate_unique_named_items(
        schema
            .types
            .iter()
            .map(|type_descriptor| type_descriptor.name.as_str()),
        "type",
    )?;
    validate_unique_named_items(
        schema
            .functions
            .iter()
            .map(|function| function.name.as_str()),
        "function",
    )?;
    validate_unique_named_items(
        schema.events.iter().map(|event| event.name.as_str()),
        "event",
    )?;
    validate_unique_named_items(
        schema
            .mutations
            .iter()
            .map(|mutation| mutation.name.as_str()),
        "mutation",
    )?;
    for function in &schema.functions {
        validate_type_ref(&function.returns, schema)?;
        for param in &function.params {
            validate_type_ref(&param.type_ref, schema)?;
        }
    }
    for type_descriptor in &schema.types {
        validate_unique_named_items(
            type_descriptor
                .fields
                .iter()
                .map(|field| field.name.as_str()),
            "field",
        )?;
        for field in &type_descriptor.fields {
            validate_type_ref(&field.type_ref, schema)?;
        }
    }
    for event in &schema.events {
        validate_type_ref(&event.payload, schema)?;
    }
    for mutation in &schema.mutations {
        validate_type_ref(&mutation.payload, schema)?;
    }
    for extension in &schema.extensions {
        for method in &extension.methods {
            validate_type_ref(&method.returns, schema)?;
        }
    }
    Ok(())
}

fn validate_unique_named_items<'a>(
    names: impl Iterator<Item = &'a str>,
    kind: &'static str,
) -> Result<(), RegistrySchemaError> {
    let mut seen = Vec::new();
    for name in names {
        if seen.iter().any(|known| known == &name) {
            return Err(RegistrySchemaError::DuplicateName {
                kind,
                name: name.to_string(),
            });
        }
        seen.push(name);
    }
    Ok(())
}

fn validate_type_ref(
    type_ref: &RegistryTypeRef,
    schema: &RegistryModuleSchema,
) -> Result<(), RegistrySchemaError> {
    match type_ref {
        RegistryTypeRef::Named { name } => {
            if name.contains('.') || schema.types.iter().any(|known| known.name == *name) {
                Ok(())
            } else {
                Err(RegistrySchemaError::MissingNamedType { name: name.clone() })
            }
        }
        RegistryTypeRef::Optional { inner } | RegistryTypeRef::Array { inner } => {
            validate_type_ref(inner, schema)
        }
        RegistryTypeRef::Void
        | RegistryTypeRef::Bool
        | RegistryTypeRef::I64
        | RegistryTypeRef::F64
        | RegistryTypeRef::String
        | RegistryTypeRef::Json => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        RegistryEventDescriptor, RegistryModuleSchema, RegistryMutationDescriptor,
        RegistrySchemaError, RegistryTypeDescriptor, RegistryTypeRef,
    };

    #[test]
    fn validates_named_event_payloads() {
        let schema = RegistryModuleSchema::new("sdk", "player")
            .type_descriptor(RegistryTypeDescriptor::new("CharacterChanged"))
            .event(RegistryEventDescriptor::new(
                "changed",
                "sdk.player.changed",
                RegistryTypeRef::Named {
                    name: "CharacterChanged".to_string(),
                },
            ));

        assert_eq!(schema.validate_contract(), Ok(()));
    }

    #[test]
    fn rejects_missing_named_payload_type() {
        let schema =
            RegistryModuleSchema::new("sdk", "player").event(RegistryEventDescriptor::new(
                "changed",
                "sdk.player.changed",
                RegistryTypeRef::Named {
                    name: "MissingPayload".to_string(),
                },
            ));

        assert_eq!(
            schema.validate_contract(),
            Err(RegistrySchemaError::MissingNamedType {
                name: "MissingPayload".to_string(),
            })
        );
    }

    #[test]
    fn rejects_duplicate_type_names() {
        let schema = RegistryModuleSchema::new("sdk", "player")
            .type_descriptor(RegistryTypeDescriptor::new("Payload"))
            .type_descriptor(RegistryTypeDescriptor::new("Payload"));

        assert_eq!(
            schema.validate_contract(),
            Err(RegistrySchemaError::DuplicateName {
                kind: "type",
                name: "Payload".to_string(),
            })
        );
    }

    #[test]
    fn validates_named_mutation_payloads() {
        let schema = RegistryModuleSchema::new("sdk", "runtime")
            .type_descriptor(RegistryTypeDescriptor::new("PatchRequest"))
            .mutation(RegistryMutationDescriptor::new(
                "patch",
                "sdk.runtime.patch",
                RegistryTypeRef::Named {
                    name: "PatchRequest".to_string(),
                },
            ));

        assert_eq!(schema.validate_contract(), Ok(()));
    }

    #[test]
    fn rejects_duplicate_mutation_names() {
        let schema = RegistryModuleSchema::new("sdk", "runtime")
            .mutation(RegistryMutationDescriptor::new(
                "patch",
                "sdk.runtime.patch",
                RegistryTypeRef::Json,
            ))
            .mutation(RegistryMutationDescriptor::new(
                "patch",
                "sdk.runtime.patch_again",
                RegistryTypeRef::Json,
            ));

        assert_eq!(
            schema.validate_contract(),
            Err(RegistrySchemaError::DuplicateName {
                kind: "mutation",
                name: "patch".to_string(),
            })
        );
    }
}
