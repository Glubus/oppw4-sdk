use sdk_schema::{
    schema, schema_module, RegistryFunctionDescriptor, RegistryMethodDescriptor,
    RegistryModuleSchema, RegistryMutationDescriptor, RegistryTypeDescriptor,
    RegistryTypeExtensionDescriptor, RegistryTypeRef, SchemaAccessorDescriptor, SchemaAccessorKind,
    SchemaEntity,
};

#[allow(dead_code)]
#[schema(constructible = true)]
struct DemoEntity {
    id: String,
    flags: Vec<bool>,
    maybe_count: Option<u32>,
}

#[allow(dead_code)]
#[schema(name = "Character", constructible = false)]
struct RenamedEntity {
    #[getter]
    #[setter]
    total: u64,
    #[schema(name = "movesetLinkdataEntry")]
    moveset_linkdata_entry: u64,
}

#[allow(dead_code)]
#[derive(SchemaEntity)]
struct DerivedEntity {
    id: String,
}

#[allow(dead_code)]
#[schema(name = "GetterOnly", constructible = false)]
struct GetterOnlyEntity {
    #[getter]
    total: u64,
}

#[allow(dead_code)]
#[schema(name = "SetterOnly", constructible = false)]
struct SetterOnlyEntity {
    #[setter]
    total: u64,
}

#[allow(dead_code)]
#[schema(name = "MovesetCharacter", constructible = false)]
struct RenamedAccessorEntity {
    #[schema(name = "movesetLinkdataEntry")]
    #[getter]
    #[setter]
    moveset_linkdata_entry: u64,
}

#[allow(dead_code)]
#[schema_module(namespace = "sdk", import_name = "character", entity = crate::RenamedEntity)]
mod character_schema {}

#[test]
fn derive_schema_entity_generates_descriptor() {
    assert_eq!(
        DemoEntity::schema_type_descriptor(),
        RegistryTypeDescriptor::new("DemoEntity")
            .constructible(true)
            .field("id", RegistryTypeRef::String)
            .field(
                "flags",
                RegistryTypeRef::Array {
                    inner: Box::new(RegistryTypeRef::Bool),
                },
            )
            .field(
                "maybe_count",
                RegistryTypeRef::Optional {
                    inner: Box::new(RegistryTypeRef::I64),
                },
            )
    );
}

#[test]
fn derive_schema_entity_respects_renames() {
    assert_eq!(RenamedEntity::schema_entity_name(), "Character");
    assert_eq!(
        RenamedEntity::schema_type_descriptor(),
        RegistryTypeDescriptor::new("Character")
            .field("total", RegistryTypeRef::I64)
            .field("movesetLinkdataEntry", RegistryTypeRef::I64)
    );
}

#[test]
fn derive_schema_entity_defaults_to_struct_name() {
    assert_eq!(DerivedEntity::schema_entity_name(), "DerivedEntity");
    assert_eq!(
        DerivedEntity::schema_type_descriptor(),
        RegistryTypeDescriptor::new("DerivedEntity").field("id", RegistryTypeRef::String)
    );
}

#[test]
fn derive_schema_entity_generates_getter_and_setter_accessors() {
    assert_eq!(
        RenamedEntity::schema_accessors(),
        vec![
            SchemaAccessorDescriptor {
                kind: SchemaAccessorKind::Getter,
                method_name: "total".to_string(),
                value_field_name: "value".to_string(),
                value_type: RegistryTypeRef::I64,
                payload_type_name: None,
            },
            SchemaAccessorDescriptor {
                kind: SchemaAccessorKind::Setter,
                method_name: "set_total".to_string(),
                value_field_name: "value".to_string(),
                value_type: RegistryTypeRef::I64,
                payload_type_name: Some("CharacterSetTotalPayload".to_string()),
            },
        ]
    );
}

#[test]
fn derive_schema_entity_supports_getter_only_and_setter_only_fields() {
    assert_eq!(
        GetterOnlyEntity::schema_accessors(),
        vec![SchemaAccessorDescriptor {
            kind: SchemaAccessorKind::Getter,
            method_name: "total".to_string(),
            value_field_name: "value".to_string(),
            value_type: RegistryTypeRef::I64,
            payload_type_name: None,
        }]
    );
    assert_eq!(
        SetterOnlyEntity::schema_accessors(),
        vec![SchemaAccessorDescriptor {
            kind: SchemaAccessorKind::Setter,
            method_name: "set_total".to_string(),
            value_field_name: "value".to_string(),
            value_type: RegistryTypeRef::I64,
            payload_type_name: Some("SetterOnlySetTotalPayload".to_string()),
        }]
    );
}

#[test]
fn derive_schema_entity_uses_schema_field_name_for_accessors() {
    assert_eq!(
        RenamedAccessorEntity::schema_accessors(),
        vec![
            SchemaAccessorDescriptor {
                kind: SchemaAccessorKind::Getter,
                method_name: "movesetLinkdataEntry".to_string(),
                value_field_name: "value".to_string(),
                value_type: RegistryTypeRef::I64,
                payload_type_name: None,
            },
            SchemaAccessorDescriptor {
                kind: SchemaAccessorKind::Setter,
                method_name: "set_movesetLinkdataEntry".to_string(),
                value_field_name: "value".to_string(),
                value_type: RegistryTypeRef::I64,
                payload_type_name: Some(
                    "MovesetCharacterSetMovesetLinkdataEntryPayload".to_string(),
                ),
            },
        ]
    );
}

#[test]
fn schema_module_generates_functions_mutations_and_types() {
    let schema = character_schema::schema_module();
    assert_eq!(
        schema,
        RegistryModuleSchema::new("sdk", "character")
            .type_descriptor(RenamedEntity::schema_type_descriptor())
            .function(
                RegistryFunctionDescriptor::new("total", RegistryTypeRef::I64).param(
                    "target",
                    RegistryTypeRef::Named {
                        name: "Character".to_string(),
                    },
                ),
            )
            .mutation(RegistryMutationDescriptor::new(
                "set_total",
                "sdk.character.set_total",
                RegistryTypeRef::Named {
                    name: "CharacterSetTotalPayload".to_string(),
                },
            ))
            .type_descriptor(
                RegistryTypeDescriptor::new("CharacterSetTotalPayload")
                    .field(
                        "target",
                        RegistryTypeRef::Named {
                            name: "Character".to_string(),
                        },
                    )
                    .field("value", RegistryTypeRef::I64),
            )
            .extension(
                RegistryTypeExtensionDescriptor::new("sdk.Character")
                    .method(RegistryMethodDescriptor::new(
                        "total",
                        "total",
                        RegistryTypeRef::I64,
                    ))
                    .method(RegistryMethodDescriptor::mutation(
                        "set_total",
                        "set_total",
                        RegistryTypeRef::Void,
                    )),
            )
    );
    assert_eq!(schema.validate_contract(), Ok(()));
}
