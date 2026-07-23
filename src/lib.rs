pub mod ast;
pub mod backend;
pub mod compiler;
pub mod diagnostic;
pub mod formatter;
pub mod frontend;
pub mod ir;
pub mod lexer;
pub mod linker;
pub mod package;
pub mod parser;
pub mod project;
pub mod semantic;
pub mod source;
pub mod specialize;
pub mod standard;
pub mod token;

pub use backend::{generate_c, generate_c_from_ir, CompileError};
pub use compiler::{
    check_project_sources, check_project_sources_with_diagnostics, check_sources,
    check_sources_with_diagnostics, generate_c_project_sources,
    generate_c_project_sources_with_diagnostics, generate_c_sources,
    generate_c_sources_with_diagnostics, lower_project_sources,
    lower_project_sources_with_diagnostics, lower_sources, lower_sources_with_diagnostics,
    prepare_project_tests, prepare_project_tests_with_diagnostics, CompilerError, CompilerStage,
    ProjectTestSuite,
};
pub use diagnostic::{
    Diagnostic, DiagnosticPosition, DiagnosticSeverity, DiagnosticSource, DiagnosticSpan,
    DiagnosticStage, DIAGNOSTIC_SCHEMA,
};
pub use formatter::{format_source, FormatError};
pub use frontend::{parse_sources, parse_sources_with_diagnostics, FrontendError};
pub use ir::{lower, lower_test, IrError, IrProgram};
pub use lexer::{lex, lex_with_source, LexError, Lexer};
pub use linker::{display_linked_message, link_project, link_standalone, LinkError};
pub use package::{
    build_package_graph, build_standalone_package_graph, Package, PackageDeclaration,
    PackageDeclarationKind, PackageError, PackageGraph, PackageImport, PackageTest,
};
pub use parser::{
    parse, parse_with_diagnostics, parse_with_source, parse_with_source_diagnostics, ParseError,
    Parser, MAX_PARSE_ERRORS_PER_SOURCE,
};
pub use project::{
    discover_project, PathDependency, Project, ProjectError, ProjectManifest, ProjectMetadata,
    ProjectUnitRef, MANIFEST_FILE,
};
pub use semantic::{check, check_project, check_project_library, CheckedProgram, SemanticError};
pub use source::{
    load_source_files, SourceFile, SourceLoadError, SourceLocation, SourceMap, SourceSet,
};
pub use specialize::{specialize, specialize_for_validation, SpecializationError, SymbolicProgram};
pub use token::{Keyword, SourceId, Span, Token, TokenKind};
