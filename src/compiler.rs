use std::fmt;

use crate::{
    ast::Program,
    backend::generate_c_from_ir,
    frontend::parse_sources,
    ir::{lower, IrProgram},
    linker::{display_linked_message, link_project},
    package::{build_package_graph, PackageGraph},
    project::Project,
    semantic::{check, check_project},
    source::SourceMap,
    token::{SourceId, Span},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerStage {
    Frontend,
    Package,
    Link,
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

pub fn check_project_sources(
    project: &Project,
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<Program, CompilerError> {
    let (program, graph) = link_project_sources(project, sources, source_ids)?;
    check_project(&program, &graph).map_err(|error| {
        CompilerError::new(
            CompilerStage::Semantic,
            display_linked_message(&error.message),
            error.span,
        )
    })?;
    Ok(program)
}

pub fn lower_project_sources(
    project: &Project,
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<IrProgram, CompilerError> {
    let (program, graph) = link_project_sources(project, sources, source_ids)?;
    let checked = check_project(&program, &graph).map_err(|error| {
        CompilerError::new(
            CompilerStage::Semantic,
            display_linked_message(&error.message),
            error.span,
        )
    })?;
    lower(&checked).map_err(|error| {
        CompilerError::new(
            CompilerStage::Ir,
            display_linked_message(&error.message),
            error.span,
        )
    })
}

pub fn generate_c_project_sources(
    project: &Project,
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<String, CompilerError> {
    let ir = lower_project_sources(project, sources, source_ids)?;
    generate_c_from_ir(&ir).map_err(|error| {
        CompilerError::new(
            CompilerStage::Backend,
            display_linked_message(&error.message),
            None,
        )
    })
}

fn link_project_sources(
    project: &Project,
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<(Program, PackageGraph), CompilerError> {
    let program = parse_program(sources, source_ids)?;
    let graph = build_package_graph(project, sources, &program)
        .map_err(|error| CompilerError::new(CompilerStage::Package, error.message, error.span))?;
    let program = link_project(project, &graph, &program)
        .map_err(|error| CompilerError::new(CompilerStage::Link, error.message, error.span))?;
    Ok((program, graph))
}

fn parse_program(sources: &SourceMap, source_ids: &[SourceId]) -> Result<Program, CompilerError> {
    parse_sources(sources, source_ids)
        .map_err(|error| CompilerError::new(CompilerStage::Frontend, error.message, error.span))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use super::*;
    use crate::{discover_project, load_source_files};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(label: &str) -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "mallang-compiler-project-test-{}-{label}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        fn load(&self) -> (Project, crate::SourceSet) {
            let project = discover_project(&self.root).unwrap();
            let sources = load_source_files(project.source_files().iter()).unwrap();
            (project, sources)
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

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

    #[test]
    fn runs_project_sources_through_package_link_ir_and_backend() {
        let temp = TempProject::new("pipeline");
        temp.write("mallang.toml", "[project]\nname = \"hello\"\n");
        temp.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { print(greet.Double(21)) }\n",
        );
        temp.write(
            "src/greet/greet.mlg",
            "package greet\npub func Double(value int) int { return value * 2 }\n",
        );
        let (project, loaded) = temp.load();

        let program = check_project_sources(&project, &loaded.sources, &loaded.source_ids).unwrap();
        assert_eq!(program.functions.len(), 2);

        let ir = lower_project_sources(&project, &loaded.sources, &loaded.source_ids).unwrap();
        assert_eq!(ir.functions.len(), 2);

        let c_source =
            generate_c_project_sources(&project, &loaded.sources, &loaded.source_ids).unwrap();
        assert!(c_source.contains("int main(void)"));
        assert!(c_source.contains("(21)"));
    }

    #[test]
    fn classifies_package_errors_and_restores_semantic_symbol_names() {
        let temp = TempProject::new("errors");
        temp.write("mallang.toml", "[project]\nname = \"hello\"\n");
        temp.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { greet.Read(1) }\n",
        );
        temp.write(
            "src/greet/greet.mlg",
            "package greet\npub type Message struct { value int }\npub func Read(message Message) { print(message.value) }\n",
        );
        let (project, loaded) = temp.load();

        let error =
            check_project_sources(&project, &loaded.sources, &loaded.source_ids).unwrap_err();
        assert_eq!(error.stage, CompilerStage::Semantic);
        assert!(
            error.message.contains("hello/greet.Message"),
            "{}",
            error.message
        );
        assert!(!error.message.contains("__mlg_pkg_"));

        temp.write(
            "src/main.mlg",
            "package main\nimport \"hello/missing\"\nfunc main() {}\n",
        );
        let (project, loaded) = temp.load();
        let error =
            check_project_sources(&project, &loaded.sources, &loaded.source_ids).unwrap_err();
        assert_eq!(error.stage, CompilerStage::Package);
    }
}
