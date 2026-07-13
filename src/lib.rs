pub mod ast;
pub mod backend;
pub mod compiler;
pub mod frontend;
pub mod ir;
pub mod lexer;
pub mod package;
pub mod parser;
pub mod project;
pub mod semantic;
pub mod source;
pub mod token;

pub use backend::{generate_c, generate_c_from_ir, CompileError};
pub use compiler::{
    check_sources, generate_c_sources, lower_sources, CompilerError, CompilerStage,
};
pub use frontend::{parse_sources, FrontendError};
pub use ir::{lower, IrError, IrProgram};
pub use lexer::{lex, lex_with_source, LexError, Lexer};
pub use package::{
    build_package_graph, Package, PackageDeclaration, PackageDeclarationKind, PackageError,
    PackageGraph, PackageImport,
};
pub use parser::{parse, parse_with_source, ParseError, Parser};
pub use project::{
    discover_project, Project, ProjectError, ProjectManifest, ProjectMetadata, MANIFEST_FILE,
};
pub use semantic::{check, CheckedProgram, SemanticError};
pub use source::{
    load_source_files, SourceFile, SourceLoadError, SourceLocation, SourceMap, SourceSet,
};
pub use token::{Keyword, SourceId, Span, Token, TokenKind};
