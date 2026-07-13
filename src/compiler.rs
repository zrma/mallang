use std::fmt;

use crate::{
    ast::Program,
    backend::generate_c_from_ir,
    frontend::parse_sources,
    ir::{lower, IrProgram},
    semantic::check,
    source::SourceMap,
    token::{SourceId, Span},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerStage {
    Frontend,
    Semantic,
    Ir,
    Backend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerError {
    pub stage: CompilerStage,
    pub message: String,
    pub span: Option<Span>,
}

impl CompilerError {
    fn new(
        stage: CompilerStage,
        message: impl Into<String>,
        span: impl Into<Option<Span>>,
    ) -> Self {
        Self {
            stage,
            message: message.into(),
            span: span.into(),
        }
    }
}

impl fmt::Display for CompilerError {
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

impl std::error::Error for CompilerError {}

pub fn check_sources(
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<Program, CompilerError> {
    let program = parse_program(sources, source_ids)?;
    check(&program)
        .map_err(|error| CompilerError::new(CompilerStage::Semantic, error.message, error.span))?;
    Ok(program)
}

pub fn lower_sources(
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<IrProgram, CompilerError> {
    let program = parse_program(sources, source_ids)?;
    let checked = check(&program)
        .map_err(|error| CompilerError::new(CompilerStage::Semantic, error.message, error.span))?;
    lower(&checked)
        .map_err(|error| CompilerError::new(CompilerStage::Ir, error.message, error.span))
}

pub fn generate_c_sources(
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<String, CompilerError> {
    let ir = lower_sources(sources, source_ids)?;
    generate_c_from_ir(&ir)
        .map_err(|error| CompilerError::new(CompilerStage::Backend, error.message, None))
}

fn parse_program(sources: &SourceMap, source_ids: &[SourceId]) -> Result<Program, CompilerError> {
    parse_sources(sources, source_ids)
        .map_err(|error| CompilerError::new(CompilerStage::Frontend, error.message, error.span))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_source_program() -> (SourceMap, Vec<SourceId>) {
        let mut sources = SourceMap::new();
        let main = sources.add_file("src/main.mlg", "func main() { print(double(21)) }\n");
        let math = sources.add_file(
            "src/math.mlg",
            "func double(value int) int { return value * 2 }\n",
        );
        (sources, vec![main, math])
    }

    #[test]
    fn runs_multi_source_program_through_semantic_ir_and_backend() {
        let (sources, source_ids) = two_source_program();

        let program = check_sources(&sources, &source_ids).unwrap();
        assert_eq!(program.functions.len(), 2);

        let ir = lower_sources(&sources, &source_ids).unwrap();
        assert_eq!(ir.functions.len(), 2);

        let c_source = generate_c_sources(&sources, &source_ids).unwrap();
        assert!(c_source.contains("int64_t mlg_double(int64_t mlg_value);"));
        assert!(c_source.contains("mlg_double(21)"));
    }

    #[test]
    fn preserves_frontend_error_stage_and_source() {
        let mut sources = SourceMap::new();
        let main = sources.add_file("src/main.mlg", "func main() {}\n");
        let broken = sources.add_file("src/broken.mlg", "func broken( {}\n");

        let error = check_sources(&sources, &[main, broken]).unwrap_err();

        assert_eq!(error.stage, CompilerStage::Frontend);
        assert_eq!(error.span.map(|span| span.source), Some(broken));
    }

    #[test]
    fn preserves_semantic_error_stage_and_source() {
        let mut sources = SourceMap::new();
        let main = sources.add_file("src/main.mlg", "func main() {}\n");
        let duplicate = sources.add_file("src/other.mlg", "func main() {}\n");

        let error = lower_sources(&sources, &[main, duplicate]).unwrap_err();

        assert_eq!(error.stage, CompilerStage::Semantic);
        assert_eq!(error.span.map(|span| span.source), Some(duplicate));
    }
}
