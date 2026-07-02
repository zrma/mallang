use std::fmt;

use crate::token::{Keyword, Span, Token, TokenKind};

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex_all()
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
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, cursor: 0 }
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
            span: Span {
                start: self.cursor,
                end: self.cursor,
            },
        });

        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        let start = self.cursor;
        let Some(ch) = self.advance_char() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span { start, end: start },
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
            ch if is_ident_start(ch) => self.ident_or_keyword(start),
            _ => {
                return Err(self.error(start, self.cursor, format!("unexpected character `{ch}`")));
            }
        };

        Ok(Token {
            kind,
            span: Span {
                start,
                end: self.cursor,
            },
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

    fn ident_or_keyword(&mut self, start: usize) -> TokenKind {
        while matches!(self.peek_char(), Some(ch) if is_ident_continue(ch)) {
            self.advance_char();
        }

        let ident = &self.source[start..self.cursor];
        if let Some(keyword) = Keyword::from_ident(ident) {
            TokenKind::Keyword(keyword)
        } else {
            TokenKind::Ident(ident.to_string())
        }
    }

    fn error(&self, start: usize, end: usize, message: impl Into<String>) -> LexError {
        LexError {
            message: message.into(),
            span: Span { start, end },
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
        let tokens = lex(r#"func main() { msg := greet(in "kim") }"#).unwrap();
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
                TokenKind::Keyword(Keyword::In),
                TokenKind::String("kim".to_string()),
                TokenKind::RightParen,
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
}
