use super::*;

#[test]
fn detects_variable_replace_costume_assets() {
    let source = r#"
          const zoro = sdk.character.get("zoro");
          zoro.replace_costume("oni", {
            model: "zoro_oni.g1m",
            textures: { left_arm: "arm.g1t", body: "body.g1t" }
          });
        "#;

    let report = analyze(source, &[]);

    assert_eq!(report.effects.len(), 3);
    assert!(report
        .effects
        .contains(&BridgeModEffect::ReplaceCostumeAsset {
            character: Some("zoro".to_string()),
            costume: "oni".to_string(),
            slot: "model".to_string(),
            file: "zoro_oni.g1m".to_string(),
        }));
    assert!(report
        .effects
        .contains(&BridgeModEffect::ReplaceCostumeAsset {
            character: Some("zoro".to_string()),
            costume: "oni".to_string(),
            slot: "texture.body".to_string(),
            file: "body.g1t".to_string(),
        }));
}

#[test]
fn detects_direct_replace_costume_receiver() {
    let source = r#"sdk.character.get("zoro").replaceCostume("default", { model: "z.g1m" });"#;

    let report = analyze(source, &[]);

    assert_eq!(
        report.effects,
        [BridgeModEffect::ReplaceCostumeAsset {
            character: Some("zoro".to_string()),
            costume: "default".to_string(),
            slot: "model".to_string(),
            file: "z.g1m".to_string(),
        }]
    );
}

#[test]
fn warns_for_dynamic_patch_shape_with_span() {
    let source = r#"character.replace_costume(config.costume, buildPatch());"#;

    let report = analyze(source, &[]);

    let warning = report
        .warnings
        .iter()
        .find(|warning| warning.code == "dynamic_replace_costume")
        .expect("warning");
    assert!(report.effects.is_empty());
    assert_eq!(warning.span.as_ref().expect("span").line, 1);
}

#[test]
fn warns_when_replace_costume_method_is_not_declared_with_span() {
    let source = r#"sdk.character.get("zoro").replace_costume("oni", { model: "z.g1m" });"#;

    let report = analyze(source, &[]);

    let warning = report
        .warnings
        .iter()
        .find(|warning| warning.code == "registry_method_missing")
        .expect("warning");
    assert_eq!(warning.span.as_ref().expect("span").column, 26);
}

#[test]
fn declared_replace_costume_method_suppresses_registry_warning() {
    let source = r#"sdk.character.get("zoro").replace_costume("oni", { model: "z.g1m" });"#;
    let modules = vec![module_with_method("replace_costume")];

    let report = analyze(source, &modules);

    assert!(!report
        .warnings
        .iter()
        .any(|warning| warning.code == "registry_method_missing"));
    assert_eq!(report.effects.len(), 1);
}

fn module_with_method(method_name: &str) -> sdk_bridge::RegistryModuleDescriptor {
    sdk_bridge::RegistryModuleDescriptor::builder("test", "sdk.character")
        .schema(
            sdk_bridge::RegistryModuleSchema::new("sdk", "character").extension(
                sdk_bridge::RegistryTypeExtensionDescriptor::new("sdk.Character").method(
                    sdk_bridge::RegistryMethodDescriptor::new(
                        method_name,
                        method_name,
                        sdk_bridge::RegistryTypeRef::Json,
                    ),
                ),
            ),
        )
        .build()
}
