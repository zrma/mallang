use std::fmt;

use crate::{
    ast::Program,
    parser::parse_with_source_diagnostics,
    source::SourceMap,
    token::{SourceId, Span},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontendError {
    pub message: String,
    pub span: Option<Span>,
}

impl FrontendError {
    fn without_span(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    fn with_span(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
        }
    }
}

impl fmt::Display for FrontendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(
                formatter,
                "{} at {}..{}",
                self.message, span.start, span.end
            )
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl std::error::Error for FrontendError {}

pub fn parse_sources(
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<Program, FrontendError> {
    match parse_sources_with_diagnostics(sources, source_ids) {
        Ok(program) => Ok(program),
        Err(errors) => Err(errors.into_iter().next().unwrap_or_else(|| {
            FrontendError::without_span("frontend failed without a diagnostic")
        })),
    }
}

pub fn parse_sources_with_diagnostics(
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<Program, Vec<FrontendError>> {
    if source_ids.is_empty() {
        return Err(vec![FrontendError::without_span(
            "cannot parse a program without source files",
        )]);
    }

    let mut programs = Vec::with_capacity(source_ids.len());
    let mut errors = Vec::new();
    for source_id in source_ids {
        let Some(source) = sources.file(*source_id) else {
            errors.push(FrontendError::without_span(format!(
                "source ID {} is not registered",
                source_id.index()
            )));
            continue;
        };
        match parse_with_source_diagnostics(source.text(), *source_id) {
            Ok(program) => programs.push(program),
            Err(parse_errors) => {
                errors.extend(
                    parse_errors
                        .into_iter()
                        .map(|error| FrontendError::with_span(error.message, error.span)),
                );
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut programs = programs.into_iter();
    let Some(mut merged) = programs.next() else {
        return Err(vec![FrontendError::without_span(
            "frontend produced no program for a non-empty source set",
        )]);
    };
    for mut program in programs {
        merged.source_units.append(&mut program.source_units);
        merged.structs.append(&mut program.structs);
        merged.enums.append(&mut program.enums);
        merged.functions.append(&mut program.functions);
        merged.tests.append(&mut program.tests);
        merged.source_spans.append(&mut program.source_spans);
    }

    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check, generate_c};

    #[test]
    fn merges_multiple_sources_for_semantic_and_backend_phases() {
        let mut sources = SourceMap::new();
        let main = sources.add_file("src/main.mlg", "func main() { print(double(21)) }\n");
        let math = sources.add_file(
            "src/math.mlg",
            "func double(value int) int { return value * 2 }\n",
        );

        let program = parse_sources(&sources, &[main, math]).unwrap();
        assert_eq!(program.source_units.len(), 2);
        assert_eq!(program.source_spans.len(), 2);
        assert_eq!(program.functions.len(), 2);
        assert_eq!(program.functions[0].span.source, main);
        assert_eq!(program.functions[1].span.source, math);

        check(&program).unwrap();
        let c_source = generate_c(&program).unwrap();
        assert!(c_source.contains("int64_t mlg_double(int64_t mlg_value);"));
        assert!(c_source.contains("mlg_double(21)"));
    }

    #[test]
    fn preserves_package_and_import_metadata_for_each_source() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { greet.Print() }\n",
        );
        let greet = sources.add_file(
            "src/greet/greet.mlg",
            "package greet\npub func Print() {}\n",
        );

        let program = parse_sources(&sources, &[main, greet]).unwrap();

        assert_eq!(program.source_units.len(), 2);
        assert_eq!(
            program.source_units[0]
                .package
                .as_ref()
                .map(|package| package.name.as_str()),
            Some("main")
        );
        assert_eq!(program.source_units[0].imports[0].path, "hello/greet");
        assert_eq!(
            program.source_units[1]
                .package
                .as_ref()
                .map(|package| package.name.as_str()),
            Some("greet")
        );
        assert_eq!(
            program.functions[1].visibility,
            crate::ast::Visibility::Public
        );
        assert_eq!(program.source_units[0].span.source, main);
        assert_eq!(program.source_units[1].span.source, greet);
    }

    #[test]
    fn cross_file_semantic_error_points_to_the_declaring_source() {
        let mut sources = SourceMap::new();
        let first = sources.add_file("src/main.mlg", "func main() {}\n");
        let second = sources.add_file("src/other.mlg", "func main() {}\n");

        let program = parse_sources(&sources, &[first, second]).unwrap();
        let error = check(&program).unwrap_err();

        assert_eq!(error.message, "duplicate function `main`");
        assert_eq!(error.span.source, second);
    }

    #[test]
    fn parse_error_points_to_its_source() {
        let mut sources = SourceMap::new();
        let first = sources.add_file("src/main.mlg", "func main() {}\n");
        let second = sources.add_file("src/broken.mlg", "func broken( {}\n");

        let error = parse_sources(&sources, &[first, second]).unwrap_err();

        assert_eq!(error.span.map(|span| span.source), Some(second));
    }

    #[test]
    fn aggregates_parse_errors_across_sources_in_input_order() {
        let mut sources = SourceMap::new();
        let first = sources.add_file(
            "src/a.mlg",
            "func brokenA(value int {}\nfunc brokenB(value bool {}\n",
        );
        let second = sources.add_file(
            "src/b.mlg",
            "func brokenC(value string {}\nfunc main() {}\n",
        );

        let errors = parse_sources_with_diagnostics(&sources, &[first, second]).unwrap_err();

        assert_eq!(errors.len(), 3);
        assert_eq!(
            errors
                .iter()
                .map(|error| error.span.map(|span| span.source))
                .collect::<Vec<_>>(),
            vec![Some(first), Some(first), Some(second)]
        );
        assert_eq!(
            parse_sources(&sources, &[first, second]).unwrap_err(),
            errors[0]
        );
    }

    #[test]
    fn rejects_empty_or_unknown_source_sets() {
        let sources = SourceMap::new();
        assert_eq!(
            parse_sources(&sources, &[]).unwrap_err().message,
            "cannot parse a program without source files"
        );
        assert_eq!(
            parse_sources(&sources, &[SourceId::new(7)])
                .unwrap_err()
                .message,
            "source ID 7 is not registered"
        );
    }
}
