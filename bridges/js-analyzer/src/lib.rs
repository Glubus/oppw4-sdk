use std::collections::BTreeMap;

use sdk_bridge::{
    analysis_warning, registry_declares_method, BridgeModEffect, RegistryModuleDescriptor,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JsAnalysisReport {
    pub effects: Vec<BridgeModEffect>,
    pub warnings: Vec<sdk_bridge::BridgeAnalysisWarning>,
}

pub fn analyze(source: &str, modules: &[RegistryModuleDescriptor]) -> JsAnalysisReport {
    let mut analyzer = Analyzer {
        source,
        modules,
        character_vars: BTreeMap::new(),
        report: JsAnalysisReport::default(),
    };
    analyzer.scan_character_bindings();
    analyzer.scan_replace_costume_calls();
    analyzer.report
}

struct Analyzer<'a> {
    source: &'a str,
    modules: &'a [RegistryModuleDescriptor],
    character_vars: BTreeMap<String, String>,
    report: JsAnalysisReport,
}

impl Analyzer<'_> {
    fn scan_character_bindings(&mut self) {
        for declaration in ["const ", "let ", "var "] {
            let mut offset = 0;
            while let Some(index) = self.source[offset..].find(declaration) {
                let start = offset + index + declaration.len();
                offset = start;
                let Some((name, after_name)) = read_identifier(self.source, start) else {
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
        let mut offset = 0;
        while let Some(index) = self.source[offset..].find(".replace_costume") {
            let method_start = offset + index;
            offset = method_start + ".replace_costume".len();
            self.scan_replace_costume_call(method_start, ".replace_costume");
        }
        let mut offset = 0;
        while let Some(index) = self.source[offset..].find(".replaceCostume") {
            let method_start = offset + index;
            offset = method_start + ".replaceCostume".len();
            self.scan_replace_costume_call(method_start, ".replaceCostume");
        }
    }

    fn scan_replace_costume_call(&mut self, method_start: usize, method_name: &str) {
        self.warn_if_replace_costume_is_not_declared(method_name.trim_start_matches('.'));
        let receiver = receiver_before(self.source, method_start);
        let character = self.character_from_receiver(receiver);
        let args_start = skip_ws(self.source, method_start + method_name.len());
        if !self.source[args_start..].starts_with('(') {
            return;
        }
        let Some(args_end) = matching_delimiter(self.source, args_start, '(', ')') else {
            self.warning(
                "dynamic_replace_costume",
                "replace_costume arguments are not statically readable",
            );
            return;
        };
        let args = &self.source[args_start + 1..args_end];
        let Some((costume, after_costume)) = read_string_literal(args, skip_ws(args, 0)) else {
            self.warning(
                "dynamic_replace_costume",
                "replace_costume costume name is not a string literal",
            );
            return;
        };
        let patch_start = skip_ws_after_comma(args, after_costume);
        if patch_start >= args.len() || !args[patch_start..].starts_with('{') {
            self.warning(
                "dynamic_replace_costume",
                "replace_costume patch is not a static object literal",
            );
            return;
        }
        let Some(patch_end) = matching_delimiter(args, patch_start, '{', '}') else {
            self.warning(
                "dynamic_replace_costume",
                "replace_costume patch object is incomplete",
            );
            return;
        };
        let patch = &args[patch_start + 1..patch_end];
        self.collect_costume_patch_effects(character, costume, patch);
    }

    fn character_from_receiver(&mut self, receiver: Option<&str>) -> Option<String> {
        let Some(receiver) = receiver.map(str::trim).filter(|value| !value.is_empty()) else {
            self.warning(
                "dynamic_character",
                "replace_costume receiver is not statically readable",
            );
            return None;
        };
        if let Some(character) = self.character_vars.get(receiver) {
            return Some(character.clone());
        }
        if let Some((character, _)) = parse_character_get(receiver, 0) {
            return Some(character);
        }
        self.warning(
            "dynamic_character",
            "replace_costume character is dynamic or unknown",
        );
        None
    }

    fn collect_costume_patch_effects(
        &mut self,
        character: Option<String>,
        costume: String,
        patch: &str,
    ) {
        if let Some(model) = string_property(patch, "model") {
            self.report
                .effects
                .push(BridgeModEffect::replace_costume_asset(
                    character.clone(),
                    costume.clone(),
                    "model",
                    model,
                ));
        }
        if let Some(textures) = object_property(patch, "textures") {
            for (name, file) in string_properties(textures) {
                self.report
                    .effects
                    .push(BridgeModEffect::replace_costume_asset(
                        character.clone(),
                        costume.clone(),
                        format!("texture.{name}"),
                        file,
                    ));
            }
        }
    }

    fn warn_if_replace_costume_is_not_declared(&mut self, method_name: &str) {
        if !registry_declares_method(self.modules, method_name) {
            self.warning(
                "registry_method_missing",
                format!(
                    "replace_costume call found but registry does not declare method {method_name}"
                ),
            );
        }
    }

    fn warning(&mut self, code: impl Into<String>, message: impl Into<String>) {
        self.report.warnings.push(analysis_warning(code, message));
    }
}

fn parse_character_get(source: &str, start: usize) -> Option<(String, usize)> {
    let text = &source[start..];
    let prefixes = [
        "sdk.character.get",
        "sdk.character.find",
        "sdk.character.findById",
        "sdk.character.find_by_id",
    ];
    let prefix = prefixes.iter().find(|prefix| text.starts_with(**prefix))?;
    let args_start = skip_ws(source, start + prefix.len());
    if !source[args_start..].starts_with('(') {
        return None;
    }
    let args_end = matching_delimiter(source, args_start, '(', ')')?;
    let args = &source[args_start + 1..args_end];
    let (character, _) = read_string_literal(args, skip_ws(args, 0))?;
    Some((character, args_end + 1))
}

fn receiver_before(source: &str, method_start: usize) -> Option<&str> {
    let mut start = method_start;
    let bytes = source.as_bytes();
    while start > 0 && bytes[start - 1].is_ascii_whitespace() {
        start -= 1;
    }
    if start == 0 {
        return None;
    }
    if bytes[start - 1] == b')' {
        let open = matching_open_delimiter(source, start - 1, '(', ')')?;
        let expr_start = expression_start_before(source, open)?;
        return Some(&source[expr_start..start]);
    }
    let expr_start = expression_start_before(source, start)?;
    Some(&source[expr_start..start])
}

fn expression_start_before(source: &str, end: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut start = end;
    while start > 0 {
        let char = bytes[start - 1] as char;
        if char.is_ascii_alphanumeric() || matches!(char, '_' | '$' | '.' | ')' | '(') {
            start -= 1;
        } else {
            break;
        }
    }
    (start < end).then_some(start)
}

fn string_property(source: &str, name: &str) -> Option<String> {
    let value_start = property_value_start(source, name)?;
    let (value, _) = read_string_literal(source, value_start)?;
    Some(value)
}

fn object_property<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let value_start = property_value_start(source, name)?;
    if !source[value_start..].starts_with('{') {
        return None;
    }
    let end = matching_delimiter(source, value_start, '{', '}')?;
    Some(&source[value_start + 1..end])
}

fn property_value_start(source: &str, name: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(index) = source[offset..].find(name) {
        let key_start = offset + index;
        let key_end = key_start + name.len();
        offset = key_end;
        if !is_key_boundary(source, key_start, key_end) {
            continue;
        }
        let colon = skip_ws(source, key_end);
        if source[colon..].starts_with(':') {
            return Some(skip_ws(source, colon + 1));
        }
    }
    None
}

fn string_properties(source: &str) -> Vec<(String, String)> {
    let mut properties = Vec::new();
    let mut offset = 0;
    while let Some((name, after_name)) = read_property_key(source, offset) {
        let colon = skip_ws(source, after_name);
        offset = after_name;
        if !source[colon..].starts_with(':') {
            continue;
        }
        let value_start = skip_ws(source, colon + 1);
        if let Some((value, value_end)) = read_string_literal(source, value_start) {
            properties.push((name.to_string(), value));
            offset = value_end;
        }
    }
    properties
}

fn read_property_key(source: &str, start: usize) -> Option<(&str, usize)> {
    let start = skip_until_identifier(source, start)?;
    read_identifier(source, start)
}

fn read_identifier(source: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let first = *bytes.get(start)? as char;
    if !(first == '_' || first == '$' || first.is_ascii_alphabetic()) {
        return None;
    }
    let mut end = start + 1;
    while let Some(byte) = bytes.get(end) {
        let char = *byte as char;
        if char == '_' || char == '$' || char.is_ascii_alphanumeric() {
            end += 1;
        } else {
            break;
        }
    }
    Some((&source[start..end], end))
}

fn read_string_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let quote = source.as_bytes().get(start).copied()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut value = String::new();
    let mut index = start + 1;
    let bytes = source.as_bytes();
    while let Some(byte) = bytes.get(index).copied() {
        if byte == quote {
            return Some((value, index + 1));
        }
        if byte == b'\\' {
            index += 1;
            value.push(*bytes.get(index)? as char);
        } else {
            value.push(byte as char);
        }
        index += 1;
    }
    None
}

fn matching_delimiter(
    source: &str,
    open: usize,
    open_char: char,
    close_char: char,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut string_quote = None;
    let bytes = source.as_bytes();
    let mut index = open;
    while let Some(byte) = bytes.get(index).copied() {
        let char = byte as char;
        if let Some(quote) = string_quote {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if char == quote {
                string_quote = None;
            }
        } else if char == '"' || char == '\'' {
            string_quote = Some(char);
        } else if char == open_char {
            depth += 1;
        } else if char == close_char {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
        index += 1;
    }
    None
}

fn matching_open_delimiter(
    source: &str,
    close: usize,
    open_char: char,
    close_char: char,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, char) in source[..=close].char_indices().rev() {
        if char == close_char {
            depth += 1;
        } else if char == open_char {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn skip_ws(source: &str, mut index: usize) -> usize {
    while source
        .as_bytes()
        .get(index)
        .is_some_and(u8::is_ascii_whitespace)
    {
        index += 1;
    }
    index
}

fn skip_ws_after_comma(source: &str, index: usize) -> usize {
    let index = skip_ws(source, index);
    if source[index..].starts_with(',') {
        skip_ws(source, index + 1)
    } else {
        index
    }
}

fn skip_until_identifier(source: &str, mut index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    while let Some(byte) = bytes.get(index) {
        let char = *byte as char;
        if char == '_' || char == '$' || char.is_ascii_alphabetic() {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_key_boundary(source: &str, start: usize, end: usize) -> bool {
    let before = start == 0
        || !source.as_bytes()[start - 1].is_ascii_alphanumeric()
            && source.as_bytes()[start - 1] != b'_';
    let after = source
        .as_bytes()
        .get(end)
        .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_');
    before && after
}

#[cfg(test)]
mod tests {
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
    fn warns_for_dynamic_patch_shape() {
        let source = r#"character.replace_costume(config.costume, buildPatch());"#;

        let report = analyze(source, &[]);

        assert!(report.effects.is_empty());
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.code == "dynamic_replace_costume"));
    }
}
