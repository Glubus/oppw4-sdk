use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

use sdk_bridge::{
    analysis_warning_at, BridgeAnalysisWarning, BridgeModEffect, BridgeSourceSpan,
    RegistryModuleDescriptor,
};
use serde::Serialize;

use crate::parser::{
    matching_delimiter, object_property, parse_character_get, read_string_literal, receiver_before,
    skip_ws, skip_ws_after_comma, string_properties, string_property,
};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct JsAnalysisReport {
    pub effects: Vec<BridgeModEffect>,
    pub warnings: Vec<BridgeAnalysisWarning>,
}

pub fn analyze(source: &str, modules: &[RegistryModuleDescriptor]) -> JsAnalysisReport {
    let mut analyzer = Analyzer {
        source,
        declared_methods: declared_methods(modules),
        character_vars: HashMap::new(),
        report: JsAnalysisReport::default(),
    };
    analyzer.scan_character_bindings();
    analyzer.scan_replace_costume_calls();
    analyzer.report
}

struct Analyzer<'source, 'registry> {
    source: &'source str,
    declared_methods: HashSet<&'registry str>,
    character_vars: HashMap<String, String>,
    report: JsAnalysisReport,
}

impl Analyzer<'_, '_> {
    fn scan_character_bindings(&mut self) {
        for declaration in ["const ", "let ", "var "] {
            let mut offset = 0;
            while let Some(index) = self.source[offset..].find(declaration) {
                let start = offset + index + declaration.len();
                offset = start;
                let Some((name, after_name)) = crate::parser::read_identifier(self.source, start)
                else {
                    continue;
                };
                let after_equals = skip_ws(self.source, after_name);
                if !self.source[after_equals..].starts_with('=') {
                    continue;
                }
                let expr_start = skip_ws(self.source, after_equals + 1);
                if let Some((character, end)) = parse_character_get(self.source, expr_start) {
                    self.character_vars.insert(name.to_string(), character);
                    offset = end;
                }
            }
        }
    }

    fn scan_replace_costume_calls(&mut self) {
        for method in [".replace_costume", ".replaceCostume"] {
            let mut offset = 0;
            while let Some(index) = self.source[offset..].find(method) {
                let method_start = offset + index;
                offset = method_start + method.len();
                self.scan_replace_costume_call(method_start, method);
            }
        }
    }

    fn scan_replace_costume_call(&mut self, method_start: usize, method_name: &str) {
        self.warn_if_replace_costume_is_not_declared(
            method_name.trim_start_matches('.'),
            method_start,
            method_name.len(),
        );
        let receiver = receiver_before(self.source, method_start);
        let character = match character_from_receiver(
            &self.character_vars,
            receiver,
            method_start,
            method_name.len(),
        ) {
            CharacterReceiver::Static(character) => Some(character),
            CharacterReceiver::Dynamic {
                message,
                offset,
                length,
            } => {
                self.warning_at("dynamic_character", message, offset, length);
                None
            }
        };
        let args_start = skip_ws(self.source, method_start + method_name.len());
        if !self.source[args_start..].starts_with('(') {
            return;
        }
        let Some(args_end) = matching_delimiter(self.source, args_start, '(', ')') else {
            self.warning_at(
                "dynamic_replace_costume",
                "replace_costume arguments are not statically readable",
                args_start,
                1,
            );
            return;
        };
        let args = &self.source[args_start + 1..args_end];
        let Some((costume, after_costume)) = read_string_literal(args, skip_ws(args, 0)) else {
            self.warning_at(
                "dynamic_replace_costume",
                "replace_costume costume name is not a string literal",
                args_start + 1,
                args_end.saturating_sub(args_start + 1).max(1),
            );
            return;
        };
        let patch_start = skip_ws_after_comma(args, after_costume);
        if patch_start >= args.len() || !args[patch_start..].starts_with('{') {
            self.warning_at(
                "dynamic_replace_costume",
                "replace_costume patch is not a static object literal",
                args_start + 1 + patch_start.min(args.len()),
                1,
            );
            return;
        }
        let Some(patch_end) = matching_delimiter(args, patch_start, '{', '}') else {
            self.warning_at(
                "dynamic_replace_costume",
                "replace_costume patch object is incomplete",
                args_start + 1 + patch_start,
                1,
            );
            return;
        };
        let patch = &args[patch_start + 1..patch_end];
        collect_costume_patch_effects(&mut self.report, character.as_deref(), &costume, patch);
    }

    fn warn_if_replace_costume_is_not_declared(
        &mut self,
        method_name: &str,
        offset: usize,
        length: usize,
    ) {
        if !self.declared_methods.contains(method_name) {
            self.warning_at(
                "registry_method_missing",
                format!(
                    "replace_costume call found but registry does not declare method {method_name}"
                ),
                offset,
                length,
            );
        }
    }

    fn warning_at(
        &mut self,
        code: impl Into<String>,
        message: impl Into<String>,
        offset: usize,
        length: usize,
    ) {
        self.report.warnings.push(analysis_warning_at(
            code,
            message,
            source_span_at(self.source, offset, length),
        ));
    }
}

enum CharacterReceiver<'a> {
    Static(Cow<'a, str>),
    Dynamic {
        message: &'static str,
        offset: usize,
        length: usize,
    },
}

fn character_from_receiver<'a>(
    character_vars: &'a HashMap<String, String>,
    receiver: Option<&str>,
    method_start: usize,
    method_len: usize,
) -> CharacterReceiver<'a> {
    let Some(receiver) = receiver.map(str::trim).filter(|value| !value.is_empty()) else {
        return CharacterReceiver::Dynamic {
            message: "replace_costume receiver is not statically readable",
            offset: method_start,
            length: method_len,
        };
    };
    if let Some(character) = character_vars.get(receiver) {
        return CharacterReceiver::Static(Cow::Borrowed(character));
    }
    if let Some((character, _)) = parse_character_get(receiver, 0) {
        return CharacterReceiver::Static(Cow::Owned(character));
    }
    CharacterReceiver::Dynamic {
        message: "replace_costume character is dynamic or unknown",
        offset: method_start.saturating_sub(receiver.len()),
        length: receiver.len().max(1),
    }
}

fn collect_costume_patch_effects(
    report: &mut JsAnalysisReport,
    character: Option<&str>,
    costume: &str,
    patch: &str,
) {
    if let Some(model) = string_property(patch, "model") {
        report.effects.push(BridgeModEffect::replace_costume_asset(
            character, costume, "model", model,
        ));
    }
    if let Some(textures) = object_property(patch, "textures") {
        for (name, file) in string_properties(textures) {
            report.effects.push(BridgeModEffect::replace_costume_asset(
                character,
                costume,
                format!("texture.{name}"),
                file,
            ));
        }
    }
}

fn declared_methods(modules: &[RegistryModuleDescriptor]) -> HashSet<&str> {
    modules
        .iter()
        .filter_map(|module| module.schema.as_ref())
        .flat_map(|schema| schema.extensions.iter())
        .flat_map(|extension| extension.methods.iter())
        .map(|method| method.name.as_str())
        .collect()
}

fn source_span_at(source: &str, offset: usize, length: usize) -> BridgeSourceSpan {
    let offset = offset.min(source.len());
    let offset = if source.is_char_boundary(offset) {
        offset
    } else {
        source
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index < offset)
            .last()
            .unwrap_or(0)
    };
    let before = &source[..offset];
    let line = before.lines().count().max(1);
    let line_start = before.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let column = source[line_start..offset].chars().count() + 1;
    let line_end = source[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(source.len());
    BridgeSourceSpan {
        line,
        column,
        length: length.max(1),
        source_line: source[line_start..line_end].to_string(),
    }
}

#[cfg(test)]
mod tests;
