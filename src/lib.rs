pub mod ast;
pub mod backend;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod source;
pub mod token;

pub use backend::{generate_c, generate_c_from_ir, CompileError};
pub use ir::{lower, IrError, IrProgram};
pub use lexer::{lex, lex_with_source, LexError, Lexer};
pub use parser::{parse, parse_with_source, ParseError, Parser};
pub use semantic::{check, CheckedProgram, SemanticError};
pub use source::{SourceFile, SourceLocation, SourceMap};
pub use token::{Keyword, SourceId, Span, Token, TokenKind};
