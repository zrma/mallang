pub mod ast;
pub mod backend;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod token;

pub use backend::{generate_c, generate_c_from_ir, CompileError};
pub use ir::{lower, IrError, IrProgram};
pub use lexer::{lex, LexError, Lexer};
pub use parser::{parse, ParseError, Parser};
pub use semantic::{check, CheckedProgram, SemanticError};
pub use token::{Keyword, Span, Token, TokenKind};
