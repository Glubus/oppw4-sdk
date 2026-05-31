use std::path::Path;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Serialize)]
pub(crate) struct AppReport {
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) files: Vec<FileReport>,
    pub(crate) summary: ReportSummary,
}

#[derive(Debug, Serialize)]
pub(crate) struct Diagnostic {
    pub(crate) path: String,
    pub(crate) severity: DiagnosticSeverity,
    pub(crate) code: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) span: Option<DiagnosticSpan>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DiagnosticSpan {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) length: usize,
    pub(crate) source_line: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Serialize)]
pub(crate) struct FileReport {
    pub(crate) path: String,
    pub(crate) effects: Vec<sdk_bridge::BridgeModEffect>,
    pub(crate) warnings: Vec<sdk_bridge::BridgeAnalysisWarning>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReportSummary {
    pub(crate) diagnostics: usize,
    pub(crate) errors: usize,
    pub(crate) files: usize,
    pub(crate) effects: usize,
    pub(crate) warnings: usize,
}

pub(crate) fn print_report(report: &AppReport, output: OutputFormat) -> Result<(), String> {
    match output {
        OutputFormat::Human => print!("{}", format_human_report(report)),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(report)
                .map_err(|error| format!("failed to serialize analysis report: {error}"))?;
            println!("{json}");
        }
    }
    Ok(())
}

pub(crate) fn check_exit_code(report: &AppReport) -> i32 {
    if report.summary.errors == 0 {
        0
    } else {
        1
    }
}

fn format_human_report(report: &AppReport) -> String {
    let mut output = String::new();
    output.push_str("    Checking bridge-js mods\n");
    for diagnostic in &report.diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
        };
        output.push_str(&format_diagnostic(severity, diagnostic));
    }
    for file in &report.files {
        for warning in &file.warnings {
            output.push_str(&format!(
                "warning[{}]: {}\n  --> {}\n\n",
                warning.code, warning.message, file.path
            ));
        }
        for effect in &file.effects {
            output.push_str(&format!(
                "note[effect]: {}\n  --> {}\n\n",
                effect.describe(),
                file.path
            ));
        }
    }

    let status = if report.summary.errors == 0 {
        "Finished"
    } else {
        "Failed"
    };
    output.push_str(&format!(
        "{status} sdk-analyzer: {} file(s), {} effect(s), {} warning(s), {} error(s)\n",
        report.summary.files,
        report.summary.effects,
        report.summary.warnings + report.summary.diagnostics - report.summary.errors,
        report.summary.errors,
    ));
    output
}

pub(crate) fn format_diagnostic(severity: &str, diagnostic: &Diagnostic) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "{severity}[{}]: {}\n",
        diagnostic.code, diagnostic.message
    ));
    if let Some(span) = &diagnostic.span {
        output.push_str(&format!(
            "  --> {}:{}:{}\n",
            diagnostic.path, span.line, span.column
        ));
        let width = span.line.to_string().len();
        output.push_str(&format!("{:>width$} |\n", "", width = width));
        output.push_str(&format!(
            "{:>width$} | {}\n",
            span.line,
            span.source_line,
            width = width
        ));
        output.push_str(&format!(
            "{:>width$} | {}{}\n\n",
            "",
            " ".repeat(span.column.saturating_sub(1)),
            "^".repeat(span.length.max(1)),
            width = width
        ));
    } else {
        output.push_str(&format!("  --> {}\n\n", diagnostic.path));
    }
    output
}

pub(crate) fn find_source_span(source: &str, needle: &str) -> Option<DiagnosticSpan> {
    let offset = source.find(needle)?;
    source_span_at(source, offset, needle.chars().count())
}

pub(crate) fn source_span_at(source: &str, offset: usize, length: usize) -> Option<DiagnosticSpan> {
    if offset > source.len() || !source.is_char_boundary(offset) {
        return None;
    }
    let before = &source[..offset];
    let line = before.lines().count().max(1);
    let line_start = before.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let column = source[line_start..offset].chars().count() + 1;
    let line_end = source[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(source.len());
    Some(DiagnosticSpan {
        line,
        column,
        length,
        source_line: source[line_start..line_end].to_string(),
    })
}

impl Diagnostic {
    pub(crate) fn error(path: &Path, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.display().to_string(),
            severity: DiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            span: None,
        }
    }

    pub(crate) fn warning(
        path: &Path,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.display().to_string(),
            severity: DiagnosticSeverity::Warning,
            code: code.into(),
            message: message.into(),
            span: None,
        }
    }

    pub(crate) fn with_span(mut self, span: Option<DiagnosticSpan>) -> Self {
        self.span = span;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_human_report_like_check_output() {
        let report = AppReport {
            diagnostics: vec![Diagnostic::warning(
                Path::new("mod.toml"),
                "uses_plugins_empty",
                "mod.toml does not declare [uses].plugins",
            )],
            files: vec![FileReport {
                path: "main.js".to_string(),
                effects: Vec::new(),
                warnings: vec![sdk_bridge::analysis_warning(
                    "dynamic_patch",
                    "dynamic patch shape cannot be fully analyzed",
                )],
            }],
            summary: ReportSummary {
                diagnostics: 1,
                errors: 0,
                files: 1,
                effects: 0,
                warnings: 1,
            },
        };

        let formatted = format_human_report(&report);

        assert!(formatted.contains("    Checking bridge-js mods"));
        assert!(formatted.contains("warning[uses_plugins_empty]"));
        assert!(formatted.contains("  --> mod.toml"));
        assert!(formatted.contains("warning[dynamic_patch]"));
        assert!(formatted.contains("Finished sdk-analyzer"));
    }

    #[test]
    fn formats_diagnostic_source_spans() {
        let diagnostic = Diagnostic::error(
            Path::new("main.js"),
            "asset_missing",
            "referenced asset does not exist: missing.g1t",
        )
        .with_span(find_source_span(
            r#"asset.replace("missing.g1t");"#,
            "missing.g1t",
        ));

        let formatted = format_diagnostic("error", &diagnostic);

        assert!(formatted.contains("--> main.js:1:16"));
        assert!(formatted.contains("1 | asset.replace(\"missing.g1t\");"));
        assert!(formatted.contains("^^^^^^^^^^^"));
    }
}
