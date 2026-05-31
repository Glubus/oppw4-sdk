use std::collections::BTreeSet;

use crate::{
    BridgeError, BridgeId, BridgeLoadReport, BridgeLoadRequest, BridgeModContext, ModId,
    ModLifecycle, ModRecord, RegistryModuleDescriptor, RegistryModuleLoad,
};

use super::BridgeRegistry;

impl BridgeRegistry {
    pub fn load_supported_mod(
        &mut self,
        request: BridgeLoadRequest,
    ) -> Result<ModLifecycle, BridgeError> {
        let bridge_id = self.bridge_for(&request)?;
        let modules = self.modules_for(&request.uses_plugins);
        self.load_mod(request.into_context(bridge_id, modules))
    }

    pub fn load_mod(&mut self, context: BridgeModContext) -> Result<ModLifecycle, BridgeError> {
        let Some(bridge) = self.bridges.get_mut(&context.bridge_id) else {
            return Err(BridgeError::MissingBridge {
                bridge_id: context.bridge_id.as_str().to_string(),
            });
        };
        let report = bridge.load_mod(&context);
        let BridgeModContext {
            mod_id, bridge_id, ..
        } = context;
        self.register_loaded_mod(mod_id, bridge_id, report)
    }

    pub fn register_loaded_mod(
        &mut self,
        mod_id: ModId,
        bridge_id: BridgeId,
        report: BridgeLoadReport,
    ) -> Result<ModLifecycle, BridgeError> {
        if !report.errors.is_empty() {
            return Err(BridgeError::LoadFailed {
                mod_id: mod_id.as_str().to_string(),
                errors: report.errors,
            });
        }

        let lifecycle = ModLifecycle::infer(&report);
        self.load_logs.extend(report.logs);
        self.load_logs.extend(
            report
                .analysis
                .warnings
                .iter()
                .map(|warning| format!("analysis warning {}: {}", warning.code, warning.message)),
        );
        for handler in report.handlers {
            if handler.mod_id != mod_id {
                return Err(BridgeError::MismatchedModId {
                    expected: mod_id.as_str().to_string(),
                    actual: handler.mod_id.as_str().to_string(),
                });
            }
            if handler.bridge_id != bridge_id {
                return Err(BridgeError::MismatchedBridgeId {
                    expected: bridge_id.as_str().to_string(),
                    actual: handler.bridge_id.as_str().to_string(),
                });
            }
            self.handlers_by_event
                .entry(handler.event_key.clone())
                .or_default()
                .push(handler);
        }

        self.boot_mutations.extend(report.boot_mutations);
        self.effects_by_mod
            .insert(mod_id.clone(), report.analysis.effects);
        self.mods.insert(
            mod_id.clone(),
            ModRecord {
                mod_id,
                bridge_id,
                lifecycle,
            },
        );
        Ok(lifecycle)
    }

    fn bridge_for(&self, request: &BridgeLoadRequest) -> Result<BridgeId, BridgeError> {
        let mut matches = self
            .bridges
            .iter()
            .filter(|(_, bridge)| bridge.supports(request))
            .map(|(id, _)| id.clone());
        let Some(first) = matches.next() else {
            return Err(BridgeError::NoBridgeForMod {
                mod_id: request.mod_id.as_str().to_string(),
                entry_file: request.entry_file.clone(),
            });
        };
        if matches.next().is_some() {
            return Err(BridgeError::AmbiguousBridgeForMod {
                mod_id: request.mod_id.as_str().to_string(),
                entry_file: request.entry_file.clone(),
            });
        }
        Ok(first)
    }

    fn modules_for(&self, uses_plugins: &[String]) -> Vec<RegistryModuleDescriptor> {
        let requested = uses_plugins
            .iter()
            .map(|plugin| plugin.to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        self.modules
            .iter()
            .filter(|module| {
                module.load == RegistryModuleLoad::Always
                    || requested.contains(&module.provider_id.to_ascii_lowercase())
            })
            .cloned()
            .collect()
    }
}
