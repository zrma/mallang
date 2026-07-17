use std::env;
use std::fs;
use std::process::ExitCode;

use mallang::{lex, Keyword, LexError, Token, TokenKind};

fn main() -> ExitCode {
    let mut args = env::args();
    let _program = args.next();
    let Some(path) = args.next() else {
        eprintln!("usage: bootstrap-frontend-oracle <source>");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("usage: bootstrap-frontend-oracle <source>");
        return ExitCode::from(2);
    }

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => {
            eprintln!("bootstrap frontend oracle could not read source");
            return ExitCode::from(2);
        }
    };

    match lex(&source) {
        Ok(tokens) => {
            for token in tokens {
                println!("{}", normalize_token(&token));
            }
        }
        Err(error) => println!("{}", normalize_error(&error)),
    }

    ExitCode::SUCCESS
}

fn normalize_token(token: &Token) -> String {
    let (kind, value) = match &token.kind {
        TokenKind::Ident(value) => ("Ident", value.as_str()),
        TokenKind::Int(value) => ("Int", value.as_str()),
        TokenKind::String(value) => ("String", value.as_str()),
        TokenKind::Keyword(keyword) => (keyword_kind(*keyword), ""),
        TokenKind::LeftParen => ("LeftParen", ""),
        TokenKind::RightParen => ("RightParen", ""),
        TokenKind::LeftBrace => ("LeftBrace", ""),
        TokenKind::RightBrace => ("RightBrace", ""),
        TokenKind::LeftBracket => ("LeftBracket", ""),
        TokenKind::RightBracket => ("RightBracket", ""),
        TokenKind::Comma => ("Comma", ""),
        TokenKind::Dot => ("Dot", ""),
        TokenKind::Colon => ("Colon", ""),
        TokenKind::Semicolon => ("Semicolon", ""),
        TokenKind::Plus => ("Plus", ""),
        TokenKind::Minus => ("Minus", ""),
        TokenKind::Star => ("Star", ""),
        TokenKind::Slash => ("Slash", ""),
        TokenKind::Percent => ("Percent", ""),
        TokenKind::Equal => ("Equal", ""),
        TokenKind::EqualEqual => ("EqualEqual", ""),
        TokenKind::Bang => ("Bang", ""),
        TokenKind::BangEqual => ("BangEqual", ""),
        TokenKind::AmpAmp => ("AmpAmp", ""),
        TokenKind::PipePipe => ("PipePipe", ""),
        TokenKind::Less => ("Less", ""),
        TokenKind::LessEqual => ("LessEqual", ""),
        TokenKind::Greater => ("Greater", ""),
        TokenKind::GreaterEqual => ("GreaterEqual", ""),
        TokenKind::ColonEqual => ("ColonEqual", ""),
        TokenKind::Arrow => ("Arrow", ""),
        TokenKind::PipeGreater => ("PipeGreater", ""),
        TokenKind::Eof => ("Eof", ""),
    };

    format!(
        "T|{kind}|{}|{}|{}",
        token.span.start,
        token.span.end,
        encode_bytes(value)
    )
}

fn normalize_error(error: &LexError) -> String {
    format!(
        "E|{}|{}|{}",
        error.span.start,
        error.span.end,
        encode_bytes(&error.message)
    )
}

fn encode_bytes(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn keyword_kind(keyword: Keyword) -> &'static str {
    match keyword {
        Keyword::Package => "Keyword.Package",
        Keyword::Import => "Keyword.Import",
        Keyword::Pub => "Keyword.Pub",
        Keyword::Func => "Keyword.Func",
        Keyword::Return => "Keyword.Return",
        Keyword::If => "Keyword.If",
        Keyword::Else => "Keyword.Else",
        Keyword::For => "Keyword.For",
        Keyword::Break => "Keyword.Break",
        Keyword::Continue => "Keyword.Continue",
        Keyword::Range => "Keyword.Range",
        Keyword::Match => "Keyword.Match",
        Keyword::Case => "Keyword.Case",
        Keyword::Mut => "Keyword.Mut",
        Keyword::Con => "Keyword.Con",
        Keyword::True => "Keyword.True",
        Keyword::False => "Keyword.False",
        Keyword::Struct => "Keyword.Struct",
        Keyword::Enum => "Keyword.Enum",
        Keyword::Type => "Keyword.Type",
        Keyword::Nil => "Keyword.Nil",
    }
}
