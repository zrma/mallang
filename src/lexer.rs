use std::fmt;

use crate::token::{Keyword, SourceId, Span, Token, TokenKind};

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    lex_with_source(source, SourceId::default())
}

pub fn lex_with_source(source: &str, source_id: SourceId) -> Result<Vec<Token>, LexError> {
    Lexer::new_with_source(source, source_id).lex_all()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for LexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for LexError {}

pub struct Lexer<'a> {
    source: &'a str,
    source_id: SourceId,
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self::new_with_source(source, SourceId::default())
    }

    pub fn new_with_source(source: &'a str, source_id: SourceId) -> Self {
        Self {
            source,
            source_id,
            cursor: 0,
        }
    }

    pub fn lex_all(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_ignored();
            if self.is_at_end() {
                break;
            }

            tokens.push(self.next_token()?);
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.source_id, self.cursor, self.cursor),
        });

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        let start = self.cursor;
        let Some(ch) = self.advance_char() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::new(self.source_id, start, start),
            });
        };

        let kind = match ch {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            ';' => TokenKind::Semicolon,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '%' => TokenKind::Percent,
            ':' if self.match_char('=') => TokenKind::ColonEqual,
            ':' => TokenKind::Colon,
            '-' if self.match_char('>') => TokenKind::Arrow,
            '-' => TokenKind::Minus,
            '/' => TokenKind::Slash,
            '=' if self.match_char('=') => TokenKind::EqualEqual,
            '=' => TokenKind::Equal,
            '!' if self.match_char('=') => TokenKind::BangEqual,
            '!' => TokenKind::Bang,
            '&' if self.match_char('&') => TokenKind::AmpAmp,
            '<' if self.match_char('=') => TokenKind::LessEqual,
            '<' => TokenKind::Less,
            '>' if self.match_char('=') => TokenKind::GreaterEqual,
            '>' => TokenKind::Greater,
            '|' if self.match_char('|') => TokenKind::PipePipe,
            '|' if self.match_char('>') => TokenKind::PipeGreater,
            '"' => self.string_token(start)?,
            ch if ch.is_ascii_digit() => self.int_token(start),
            ch if is_ident_start(ch) => self.ident_or_keyword(start)?,
            _ => {
                return Err(self.error(start, self.cursor, format!("unexpected character `{ch}`")));
            }
        };

        Ok(Token {
            kind,
            span: Span::new(self.source_id, start, self.cursor),
        })
    }

    fn skip_ignored(&mut self) {
        loop {
            while matches!(self.peek_char(), Some(ch) if ch.is_whitespace()) {
                self.advance_char();
            }

            if self.peek_char() == Some('/') && self.peek_next_char() == Some('/') {
                while !matches!(self.peek_char(), None | Some('\n')) {
                    self.advance_char();
                }
                continue;
            }

            break;
        }
    }

    fn string_token(&mut self, start: usize) -> Result<TokenKind, LexError> {
        let mut value = String::new();

        loop {
            let Some(ch) = self.advance_char() else {
                return Err(self.error(start, self.cursor, "unterminated string literal"));
            };

            match ch {
                '"' => return Ok(TokenKind::String(value)),
                '\\' => {
                    let Some(escaped) = self.advance_char() else {
                        return Err(self.error(start, self.cursor, "unterminated escape sequence"));
                    };

                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        _ => {
                            return Err(self.error(
                                self.cursor - escaped.len_utf8(),
                                self.cursor,
                                format!("unsupported escape sequence `\\{escaped}`"),
                            ));
                        }
                    }
                }
                _ => value.push(ch),
            }
        }
    }

    fn int_token(&mut self, start: usize) -> TokenKind {
        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
            self.advance_char();
        }

        TokenKind::Int(self.source[start..self.cursor].to_string())
    }

    fn ident_or_keyword(&mut self, start: usize) -> Result<TokenKind, LexError> {
        while matches!(self.peek_char(), Some(ch) if is_ident_continue(ch)) {
            self.advance_char();
        }

        let ident = &self.source[start..self.cursor];
        if ident.starts_with("__mlg_") {
            return Err(self.error(
                start,
                self.cursor,
                "identifiers beginning with `__mlg_` are reserved for the compiler",
            ));
        }
        if let Some(keyword) = Keyword::from_ident(ident) {
            Ok(TokenKind::Keyword(keyword))
        } else {
            Ok(TokenKind::Ident(ident.to_string()))
        }
    }

    fn error(&self, start: usize, end: usize, message: impl Into<String>) -> LexError {
        LexError {
            message: message.into(),
            span: Span::new(self.source_id, start, end),
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.advance_char();
            true
        } else {
            false
        }
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut chars = self.source[self.cursor..].chars();
        chars.next()?;
        chars.next()
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.source.len()
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_function_and_borrow_call() {
        let tokens = lex(r#"func main() { msg := greet(con "kim") }"#).unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Keyword(Keyword::Func),
                TokenKind::Ident("main".to_string()),
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBrace,
                TokenKind::Ident("msg".to_string()),
                TokenKind::ColonEqual,
                TokenKind::Ident("greet".to_string()),
                TokenKind::LeftParen,
                TokenKind::Keyword(Keyword::Con),
                TokenKind::String("kim".to_string()),
                TokenKind::RightParen,
                TokenKind::RightBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_in_as_identifier_not_borrow_keyword() {
        let tokens = lex("in con mut").unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident("in".to_string()),
                TokenKind::Keyword(Keyword::Con),
                TokenKind::Keyword(Keyword::Mut),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_project_declaration_keywords() {
        let tokens = lex("package main import \"hello/greet\" pub func Print() {}").unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Keyword(Keyword::Package),
                TokenKind::Ident("main".to_string()),
                TokenKind::Keyword(Keyword::Import),
                TokenKind::String("hello/greet".to_string()),
                TokenKind::Keyword(Keyword::Pub),
                TokenKind::Keyword(Keyword::Func),
                TokenKind::Ident("Print".to_string()),
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_line_comments() {
        let tokens = lex("x := 1 // ignored\n y := 2").unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident("x".to_string()),
                TokenKind::ColonEqual,
                TokenKind::Int("1".to_string()),
                TokenKind::Ident("y".to_string()),
                TokenKind::ColonEqual,
                TokenKind::Int("2".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_logical_operators() {
        let tokens = lex("ready && enabled || fallback").unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|token| token.kind).collect();

        assert_eq!(
            kinds,
            vec![
                TokenKind::Ident("ready".to_string()),
                TokenKind::AmpAmp,
                TokenKind::Ident("enabled".to_string()),
                TokenKind::PipePipe,
                TokenKind::Ident("fallback".to_string()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn preserves_source_id_on_tokens_and_errors() {
        let source_id = SourceId::new(7);
        let tokens = lex_with_source("func main() {}", source_id).unwrap();
        assert!(tokens.iter().all(|token| token.span.source == source_id));

        let error = lex_with_source("@", source_id).unwrap_err();
        assert_eq!(error.span.source, source_id);
    }

    #[test]
    fn rejects_compiler_internal_identifier_prefix() {
        let error = lex("func __mlg_pkg_hidden() {}").unwrap_err();

        assert_eq!(
            error.message,
            "identifiers beginning with `__mlg_` are reserved for the compiler"
        );
        assert_eq!(error.span.start, 5);
    }
}
