use std::path::Path;

use serde::Serialize;

use crate::{compiler::CompilerStage, source::SourceMap, token::Span};

pub const DIAGNOSTIC_SCHEMA: &str = "mallang.diagnostic.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticStage {
    Cli,
    Input,
    Frontend,
    Package,
    Link,
    Lint,
    Semantic,
    Ir,
    Backend,
    Native,
}

impl From<CompilerStage> for DiagnosticStage {
    fn from(stage: CompilerStage) -> Self {
        match stage {
            CompilerStage::Frontend => Self::Frontend,
            CompilerStage::Package => Self::Package,
            CompilerStage::Link => Self::Link,
            CompilerStage::Semantic => Self::Semantic,
            CompilerStage::Ir => Self::Ir,
            CompilerStage::Backend => Self::Backend,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DiagnosticPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DiagnosticSpan {
    pub byte_start: usize,
    pub byte_end: usize,
    pub start: DiagnosticPosition,
    pub end: DiagnosticPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticSource {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<DiagnosticSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub schema: &'static str,
    pub severity: DiagnosticSeverity,
    pub stage: DiagnosticStage,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<DiagnosticSource>,
}

impl Diagnostic {
    pub fn error(stage: DiagnosticStage, message: impl Into<String>) -> Self {
        Self {
            schema: DIAGNOSTIC_SCHEMA,
            severity: DiagnosticSeverity::Error,
            stage,
            message: message.into(),
            code: None,
            source: None,
        }
    }

    pub fn warning(stage: DiagnosticStage, message: impl Into<String>) -> Self {
        Self {
            schema: DIAGNOSTIC_SCHEMA,
            severity: DiagnosticSeverity::Warning,
            stage,
            message: message.into(),
            code: None,
            source: None,
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.source = Some(DiagnosticSource {
            path: path.as_ref().to_string_lossy().into_owned(),
            span: None,
        });
        self
    }

    pub fn with_span(
        mut self,
        sources: &SourceMap,
        span: Span,
        display_path: Option<&Path>,
    ) -> Self {
        let Some(file) = sources.file(span.source) else {
            return self;
        };
        let Some(start) = file.location(span.start) else {
            return self.with_path(display_path.unwrap_or_else(|| file.path()));
        };
        let Some(end) = file.location(span.end) else {
            return self.with_path(display_path.unwrap_or_else(|| file.path()));
        };

        self.source = Some(DiagnosticSource {
            path: display_path
                .unwrap_or_else(|| file.path())
                .to_string_lossy()
                .into_owned(),
            span: Some(DiagnosticSpan {
                byte_start: span.start,
                byte_end: span.end,
                start: DiagnosticPosition {
                    line: start.line,
                    column: start.column,
                },
                end: DiagnosticPosition {
                    line: end.line,
                    column: end.column,
                },
            }),
        });
        self
    }

    pub fn render_human(&self) -> String {
        let message = match (&self.code, self.severity) {
            (Some(code), DiagnosticSeverity::Warning) => {
                format!("warning[{code}]: {}", self.message)
            }
            (Some(code), DiagnosticSeverity::Error) => format!("[{code}] {}", self.message),
            (None, DiagnosticSeverity::Warning) => format!("warning: {}", self.message),
            (None, DiagnosticSeverity::Error) => self.message.clone(),
        };
        match &self.source {
            Some(source) => match source.span {
                Some(span) => format!(
                    "{}:{}:{}: {}",
                    source.path, span.start.line, span.start.column, message
                ),
                None => format!("{}: {}", source.path, message),
            },
            None => message,
        }
    }

    pub fn render_json(&self) -> String {
        serde_json::to_string(self)
            .expect("Diagnostic only contains JSON-serializable string and integer fields")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SourceMap;

    #[test]
    fn renders_human_and_json_from_the_same_source_span() {
        let mut sources = SourceMap::new();
        let source = sources.add_file("src/main.mlg", "// a\u{d55c}\nfunc main() {}\n");
        let start = "// a\u{d55c}\nfunc ".len();
        let diagnostic = Diagnostic::error(DiagnosticStage::Semantic, "unknown name").with_span(
            &sources,
            Span::new(source, start, start + 4),
            None,
        );

        assert_eq!(diagnostic.render_human(), "src/main.mlg:2:6: unknown name");
        assert_eq!(
            diagnostic.render_json(),
            r#"{"schema":"mallang.diagnostic.v1","severity":"error","stage":"semantic","message":"unknown name","source":{"path":"src/main.mlg","span":{"byte_start":13,"byte_end":17,"start":{"line":2,"column":6},"end":{"line":2,"column":10}}}}"#
        );
    }

    #[test]
    fn renders_path_only_diagnostics_without_a_span() {
        let diagnostic =
            Diagnostic::error(DiagnosticStage::Input, "not formatted").with_path("src/main.mlg");

        assert_eq!(diagnostic.render_human(), "src/main.mlg: not formatted");
        assert_eq!(
            diagnostic.render_json(),
            r#"{"schema":"mallang.diagnostic.v1","severity":"error","stage":"input","message":"not formatted","source":{"path":"src/main.mlg"}}"#
        );
    }

    #[test]
    fn renders_warning_codes_in_human_and_json_diagnostics() {
        let diagnostic =
            Diagnostic::warning(DiagnosticStage::Lint, "type name should use PascalCase")
                .with_code("MLG-NAME-001")
                .with_path("src/main.mlg");

        assert_eq!(
            diagnostic.render_human(),
            "src/main.mlg: warning[MLG-NAME-001]: type name should use PascalCase"
        );
        assert_eq!(
            diagnostic.render_json(),
            r#"{"schema":"mallang.diagnostic.v1","severity":"warning","stage":"lint","message":"type name should use PascalCase","code":"MLG-NAME-001","source":{"path":"src/main.mlg"}}"#
        );
    }

    #[test]
    fn serializes_the_stable_stage_vocabulary() {
        let stages = [
            (DiagnosticStage::Cli, "cli"),
            (DiagnosticStage::Input, "input"),
            (DiagnosticStage::Frontend, "frontend"),
            (DiagnosticStage::Package, "package"),
            (DiagnosticStage::Link, "link"),
            (DiagnosticStage::Semantic, "semantic"),
            (DiagnosticStage::Ir, "ir"),
            (DiagnosticStage::Backend, "backend"),
            (DiagnosticStage::Native, "native"),
        ];

        for (stage, expected) in stages {
            let value = serde_json::to_value(Diagnostic::error(stage, "message")).unwrap();
            assert_eq!(value["stage"], expected);
        }
    }
}
