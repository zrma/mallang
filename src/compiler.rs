use std::fmt;

use crate::{
    ast::Program,
    backend::generate_c_from_ir,
    frontend::parse_sources,
    ir::{lower, IrProgram},
    linker::{display_linked_message, link_project, link_standalone},
    package::{build_package_graph, build_standalone_package_graph, PackageGraph},
    project::Project,
    semantic::check_project,
    source::SourceMap,
    standard::augment_program,
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
    let (program, graph) = link_standalone_sources(sources, source_ids)?;
    check_project(&program, &graph).map_err(|error| {
        CompilerError::new(
            CompilerStage::Semantic,
            display_linked_message(&error.message),
            error.span,
        )
    })?;
    Ok(program)
}

pub fn lower_sources(
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<IrProgram, CompilerError> {
    let (program, graph) = link_standalone_sources(sources, source_ids)?;
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
    let mut program = link_project(project, &graph, &program)
        .map_err(|error| CompilerError::new(CompilerStage::Link, error.message, error.span))?;
    augment_program(&mut program, &graph);
    Ok((program, graph))
}

fn link_standalone_sources(
    sources: &SourceMap,
    source_ids: &[SourceId],
) -> Result<(Program, PackageGraph), CompilerError> {
    let program = parse_program(sources, source_ids)?;
    let graph = build_standalone_package_graph(sources, &program)
        .map_err(|error| CompilerError::new(CompilerStage::Package, error.message, error.span))?;
    let mut program = link_standalone(&graph, &program)
        .map_err(|error| CompilerError::new(CompilerStage::Link, error.message, error.span))?;
    augment_program(&mut program, &graph);
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
    use crate::{
        discover_project,
        ir::{IrExprKind, IrStmtKind},
        load_source_files,
        standard::{StandardIntrinsic, StandardType},
    };

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
    fn lowers_standalone_standard_calls_to_typed_intrinsics() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "main.mlg",
            "import \"std/strings\"\nfunc main() { text := \"hello\"; size := strings.byteLen(con text); print(size) }\n",
        );

        let ir = lower_sources(&sources, &[main]).unwrap();
        let IrStmtKind::Let { expr, .. } = &ir.functions[0].body[1].kind else {
            panic!("expected standard call initializer");
        };
        let IrExprKind::IntrinsicCall { intrinsic, args } = &expr.kind else {
            panic!("expected typed standard intrinsic call, got {expr:?}");
        };

        assert_eq!(*intrinsic, StandardIntrinsic::StringsByteLen);
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn rejects_standard_signature_mode_and_type_mismatches() {
        let mut mode_sources = SourceMap::new();
        let mode_main = mode_sources.add_file(
            "mode.mlg",
            "import \"std/strings\"\nfunc main() { text := \"hello\"; print(strings.byteLen(text)) }\n",
        );
        let mode_error = check_sources(&mode_sources, &[mode_main]).unwrap_err();
        assert_eq!(mode_error.stage, CompilerStage::Semantic);
        assert!(mode_error.message.contains("expects `con` argument"));

        let mut type_sources = SourceMap::new();
        let type_main = type_sources.add_file(
            "type.mlg",
            "import \"std/strings\"\nfunc main() { value := 1; print(strings.byteLen(con value)) }\n",
        );
        let type_error = check_sources(&type_sources, &[type_main]).unwrap_err();
        assert_eq!(type_error.stage, CompilerStage::Semantic);
        assert!(type_error.message.contains("expected `string`"));
    }

    #[test]
    fn rejects_unknown_standard_packages_and_unimplemented_runtime_calls() {
        let mut unknown_sources = SourceMap::new();
        let unknown_main =
            unknown_sources.add_file("unknown.mlg", "import \"std/unknown\"\nfunc main() {}\n");
        let unknown_error = check_sources(&unknown_sources, &[unknown_main]).unwrap_err();
        assert_eq!(unknown_error.stage, CompilerStage::Package);
        assert_eq!(
            unknown_error.message,
            "unknown standard package `std/unknown`"
        );

        let mut runtime_sources = SourceMap::new();
        let runtime_main = runtime_sources.add_file(
            "runtime.mlg",
            "import \"std/collections\"\nfunc main() { result := collections.newMap[int, string]() }\n",
        );
        let runtime_error = generate_c_sources(&runtime_sources, &[runtime_main]).unwrap_err();
        assert_eq!(runtime_error.stage, CompilerStage::Backend);
        assert_eq!(
            runtime_error.message,
            "standard intrinsic `std/collections.newMap` is not implemented in this compiler milestone"
        );
    }

    #[test]
    fn checks_and_lowers_explicit_map_specializations() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "map.mlg",
            "import \"std/collections\"\nfunc main() { values := collections.newMap[string, int](); print(collections.count[string, int](con values)) }\n",
        );

        check_sources(&sources, &[main]).unwrap();
        let ir = lower_sources(&sources, &[main]).unwrap();
        let debug = format!("{ir:?}");
        assert!(debug.contains("CollectionsNewMap"));
        assert!(debug.contains("CollectionsCount"));
        let map = ir
            .structs
            .iter()
            .find(|declaration| declaration.intrinsic == Some(StandardType::Map))
            .unwrap();
        assert_eq!(
            map.intrinsic_args,
            [crate::semantic::Type::String, crate::semantic::Type::Int]
        );
    }

    #[test]
    fn rejects_wrong_standard_generic_arity() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "map-arity.mlg",
            "import \"std/collections\"\nfunc main() { values := collections.newMap[string](); print(values) }\n",
        );

        let error = check_sources(&sources, &[main]).unwrap_err();

        assert_eq!(error.stage, CompilerStage::Semantic);
        assert!(
            error.message.contains("expects 2 type argument(s), got 1"),
            "{}",
            error.message
        );
    }

    #[test]
    fn rejects_unsupported_map_key_types() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "map-key.mlg",
            "import \"std/collections\"\nfunc main() { values := collections.newMap[[]int, string](); print(values) }\n",
        );

        let error = check_sources(&sources, &[main]).unwrap_err();

        assert_eq!(error.stage, CompilerStage::Semantic);
        assert!(error
            .message
            .contains("collections.Map key type must be `int`, `bool`, or `string`"));
    }

    #[test]
    fn compiles_unused_standard_imports_without_runtime_bodies() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "unused.mlg",
            "import \"std/strings\"\nfunc main() { print(1) }\n",
        );

        let c = generate_c_sources(&sources, &[main]).unwrap();

        assert!(c.contains("int main(void)"));
        assert!(!c.contains("mlg_byteLen"));
    }

    #[test]
    fn checks_project_standard_imports_through_the_shared_linker() {
        let temp = TempProject::new("standard-project");
        temp.write("mallang.toml", "[project]\nname = \"hello\"\n");
        temp.write(
            "src/main.mlg",
            "package main\nimport \"std/strings\"\nfunc main() { text := \"hello\"; print(strings.byteLen(con text)) }\n",
        );
        let (project, loaded) = temp.load();

        check_project_sources(&project, &loaded.sources, &loaded.source_ids).unwrap();
        let ir = lower_project_sources(&project, &loaded.sources, &loaded.source_ids).unwrap();

        assert!(format!("{ir:?}").contains("StringsByteLen"));
    }

    #[test]
    fn preserves_intrinsic_identity_for_standard_function_values() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "function-value.mlg",
            "import \"std/strings\"\nfunc main() { read := strings.byteLen; text := \"hello\"; print(read(con text)) }\n",
        );

        let ir = lower_sources(&sources, &[main]).unwrap();

        assert!(format!("{ir:?}").contains("IntrinsicFunctionValue"));

        let c = generate_c_sources(&sources, &[main]).unwrap();
        assert!(c.contains("mallang_std_strings_byte_len"));
        assert!(c.contains("mallang_callable_thunk_mlg___mlg_pkg_"));
    }

    #[test]
    fn rejects_direct_construction_of_opaque_standard_types() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "opaque.mlg",
            "import \"std/collections\"\nfunc main() { values := collections.Map[string, int]{}; print(values) }\n",
        );

        let error = check_sources(&sources, &[main]).unwrap_err();

        assert_eq!(error.stage, CompilerStage::Link);
        assert_eq!(
            error.message,
            "type `collections.Map` is opaque and cannot be constructed directly"
        );
    }

    #[test]
    fn rejects_printing_opaque_standard_maps() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "opaque-print.mlg",
            "import \"std/collections\"\nfunc main() { values := collections.newMap[string, int](); print(values) }\n",
        );

        let error = check_sources(&sources, &[main]).unwrap_err();

        assert_eq!(error.stage, CompilerStage::Semantic);
        assert!(error.message.contains("cannot print value of type"));
        assert!(
            error.message.contains("std/collections.Map[string,int]"),
            "{}",
            error.message
        );
    }

    #[test]
    fn prints_standard_errors_without_internal_linker_names() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "error-print.mlg",
            "import \"std/strings\"\nfunc main() { text := \"x\"; match strings.parseInt(con text) { case Ok(value) { print(value) } case Err(error) { print(error) } } }\n",
        );

        let c = generate_c_sources(&sources, &[main]).unwrap();

        assert!(c.contains("printf(\"Error{\");"));
        assert!(!c.contains("printf(\"__mlg_pkg_"));
    }

    #[test]
    fn generates_process_and_stream_standard_runtime() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "process.mlg",
            "import \"std/io\"\nimport \"std/os\"\nfunc main() { read := os.args; result := read(); text := \"\"; written := io.writeStdout(con text); os.exit(0) }\n",
        );

        let c = generate_c_sources(&sources, &[main]).unwrap();

        assert!(c.contains("int main(int argc, char **argv)"));
        assert!(c.contains("mallang_process_init(argc, argv);"));
        assert!(c.contains("mallang_std_os_args"));
        assert!(c.contains("mallang_std_io_write_stdout"));
        assert!(c.contains("mallang_std_os_exit"));
        assert!(c.contains("mallang_callable_thunk_mlg___mlg_pkg_"));
        assert!(!c.contains("void mlg_Ok;"));
    }

    #[test]
    fn generates_file_standard_runtime() {
        let mut sources = SourceMap::new();
        let main = sources.add_file(
            "file.mlg",
            "import \"std/fs\"\nfunc main() { path := \"file.txt\"; text := \"text\"; read := fs.readText; input := read(con path); written := fs.writeText(con path, con text) }\n",
        );

        let c = generate_c_sources(&sources, &[main]).unwrap();

        assert!(c.contains("int main(void)"));
        assert!(c.contains("mallang_std_file_path"));
        assert!(c.contains("mallang_std_fs_read_text"));
        assert!(c.contains("mallang_std_fs_write_text"));
        assert!(c.contains("mallang_callable_thunk_mlg___mlg_pkg_"));
        assert!(!c.contains("mallang_process_init"));
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
