use crate::{ModId, RegistryModuleDescriptor};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BridgeAnalysisWarning {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum BridgeModEffect {
    ReplaceCostumeAsset {
        character: Option<String>,
        costume: String,
        slot: String,
        file: String,
    },
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct BridgeAnalysisReport {
    pub effects: Vec<BridgeModEffect>,
    pub warnings: Vec<BridgeAnalysisWarning>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectConflict {
    pub effect: BridgeModEffect,
    pub mod_ids: Vec<ModId>,
}

impl BridgeModEffect {
    pub fn replace_costume_asset(
        character: Option<impl Into<String>>,
        costume: impl Into<String>,
        slot: impl Into<String>,
        file: impl Into<String>,
    ) -> Self {
        Self::ReplaceCostumeAsset {
            character: character.map(Into::into),
            costume: costume.into(),
            slot: normalize_effect_slot(slot),
            file: file.into(),
        }
    }

    pub fn conflict_key(&self) -> String {
        match self {
            Self::ReplaceCostumeAsset {
                character,
                costume,
                slot,
                ..
            } => format!(
                "costume_asset:{}:{}:{}",
                character.as_deref().unwrap_or("unknown"),
                costume,
                slot
            )
            .to_ascii_lowercase(),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::ReplaceCostumeAsset {
                character,
                costume,
                slot,
                file,
            } => format!(
                "{} costume {costume} {slot} with {file}",
                character.as_deref().unwrap_or("unknown character")
            ),
        }
    }
}

pub fn analysis_warning(
    code: impl Into<String>,
    message: impl Into<String>,
) -> BridgeAnalysisWarning {
    BridgeAnalysisWarning {
        code: code.into(),
        message: message.into(),
    }
}

pub fn registry_declares_method(modules: &[RegistryModuleDescriptor], method_name: &str) -> bool {
    modules.iter().any(|module| {
        module.schema.as_ref().is_some_and(|schema| {
            schema.extensions.iter().any(|extension| {
                extension
                    .methods
                    .iter()
                    .any(|method| method.name == method_name)
            })
        })
    })
}

fn normalize_effect_slot(slot: impl Into<String>) -> String {
    slot.into().trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        RegistryMethodDescriptor, RegistryModuleDescriptor, RegistryModuleLoad,
        RegistryModuleSchema, RegistryTypeExtensionDescriptor, RegistryTypeRef,
    };

    #[test]
    fn normalizes_costume_asset_conflict_key() {
        let effect =
            BridgeModEffect::replace_costume_asset(Some("Zoro"), "Oni", "Texture.Body", "body.g1t");

        assert_eq!(effect.conflict_key(), "costume_asset:zoro:oni:texture.body");
    }

    #[test]
    fn finds_declared_registry_extension_method() {
        let module = RegistryModuleDescriptor {
            provider_id: "sdk_data".to_string(),
            module_name: "sdk.character".to_string(),
            module_context: 0,
            install: None,
            invoke: None,
            load: RegistryModuleLoad::Always,
            schema: Some(RegistryModuleSchema::new("sdk", "character").extension(
                RegistryTypeExtensionDescriptor::new("sdk.Character").method(
                    RegistryMethodDescriptor::new(
                        "replace_costume",
                        "replace_costume",
                        RegistryTypeRef::Json,
                    ),
                ),
            )),
        };

        assert!(registry_declares_method(&[module], "replace_costume"));
    }

    #[test]
    fn missing_registry_extension_method_is_not_declared() {
        let module = RegistryModuleDescriptor {
            provider_id: "sdk_data".to_string(),
            module_name: "sdk.character".to_string(),
            module_context: 0,
            install: None,
            invoke: None,
            load: RegistryModuleLoad::Always,
            schema: Some(RegistryModuleSchema::new("sdk", "character")),
        };

        assert!(!registry_declares_method(&[module], "replace_costume"));
    }

    #[test]
    fn analysis_report_serializes_effects_and_warnings() {
        let report = BridgeAnalysisReport {
            effects: vec![BridgeModEffect::replace_costume_asset(
                Some("zoro"),
                "oni",
                "Texture.Body",
                "body.g1t",
            )],
            warnings: vec![analysis_warning("dynamic_character", "unknown receiver")],
        };

        let json = serde_json::to_value(report).expect("json");

        assert_eq!(
            json["effects"][0]["ReplaceCostumeAsset"]["character"],
            "zoro"
        );
        assert_eq!(
            json["effects"][0]["ReplaceCostumeAsset"]["slot"],
            "texture.body"
        );
        assert_eq!(json["warnings"][0]["code"], "dynamic_character");
    }
}
