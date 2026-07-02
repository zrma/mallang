pub mod ast;
pub mod backend;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod token;

pub use backend::{generate_c, CompileError};
pub use lexer::{lex, LexError, Lexer};
pub use parser::{parse, ParseError, Parser};
pub use semantic::{check, CheckedProgram, SemanticError};
pub use token::{Keyword, Span, Token, TokenKind};
