use std::fmt;

use crate::{
    ast::{
        Arg, ArgMode, BinaryOp, Block, Expr, ExprKind, Function, Param, ParamMode, Program, Stmt,
        StmtKind, TypeRef, UnaryOp,
    },
    lexer::{lex, LexError},
    token::{Keyword, Span, Token, TokenKind},
};

pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = lex(source).map_err(ParseError::from_lex)?;
    Parser::new(tokens).parse_program()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    fn from_lex(error: LexError) -> Self {
        Self {
            message: error.message,
            span: error.span,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let start = self.peek().span;
        let mut functions = Vec::new();

        while !self.at(TokenTag::Eof) {
            functions.push(self.parse_function()?);
        }

        let end = self.peek().span;
        Ok(Program {
            functions,
            span: start.join(end),
        })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let start = self.expect_keyword(Keyword::Func, "expected `func` declaration")?;
        let (name, _) = self.expect_ident("expected function name")?;
        self.expect(TokenTag::LeftParen, "expected `(` after function name")?;

        let mut params = Vec::new();
        if !self.at(TokenTag::RightParen) {
            loop {
                params.push(self.parse_param()?);
                if self.eat(TokenTag::Comma).is_none() {
                    break;
                }
            }
        }

        self.expect(
            TokenTag::RightParen,
            "expected `)` after function parameters",
        )?;
        let return_type = if self.at(TokenTag::Ident) {
            Some(self.parse_type_ref()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = start.join(body.span);

        Ok(Function {
            name,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let (name, start) = self.expect_ident("expected parameter name")?;
        let mode = if self.eat_keyword(Keyword::In).is_some() {
            ParamMode::In
        } else if self.eat_keyword(Keyword::Mut).is_some() {
            ParamMode::Mut
        } else {
            ParamMode::Owned
        };
        let ty = self.parse_type_ref()?;
        let span = start.join(ty.span);

        Ok(Param {
            name,
            mode,
            ty,
            span,
        })
    }

    fn parse_type_ref(&mut self) -> Result<TypeRef, ParseError> {
        let (name, span) = self.expect_ident("expected type name")?;
        Ok(TypeRef { name, span })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(TokenTag::LeftBrace, "expected `{`")?;
        let mut statements = Vec::new();

        while !self.at(TokenTag::RightBrace) && !self.at(TokenTag::Eof) {
            statements.push(self.parse_statement()?);
            while self.eat(TokenTag::Semicolon).is_some() {}
        }

        let end = self.expect(TokenTag::RightBrace, "expected `}` after block")?;
        Ok(Block {
            statements,
            span: start.join(end),
        })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        if self.at_keyword(Keyword::Return) {
            return self.parse_return_statement();
        }

        if self.at_keyword(Keyword::Mut)
            || (self.at(TokenTag::Ident) && self.peek_next_is(TokenTag::ColonEqual))
        {
            return self.parse_let_statement();
        }

        let expr = self.parse_expression()?;
        let span = expr.span;
        Ok(Stmt {
            kind: StmtKind::Expr { expr },
            span,
        })
    }

    fn parse_let_statement(&mut self) -> Result<Stmt, ParseError> {
        let mut mutable = false;
        let start = if let Some(span) = self.eat_keyword(Keyword::Mut) {
            mutable = true;
            span
        } else {
            self.peek().span
        };
        let (name, _) = self.expect_ident("expected binding name")?;
        self.expect(TokenTag::ColonEqual, "expected `:=` in binding")?;
        let expr = self.parse_expression()?;
        let span = start.join(expr.span);

        Ok(Stmt {
            kind: StmtKind::Let {
                mutable,
                name,
                expr,
            },
            span,
        })
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::Return, "expected `return`")?;
        let expr = self.parse_expression()?;
        let span = start.join(expr.span);

        Ok(Stmt {
            kind: StmtKind::Return { expr },
            span,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_precedence(0)
    }

    fn parse_precedence(&mut self, min_precedence: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            if self.at(TokenTag::LeftParen) {
                left = self.finish_call(left)?;
                continue;
            }

            let Some((op, precedence)) = self.peek_binary_op() else {
                break;
            };
            if precedence < min_precedence {
                break;
            }

            self.advance();
            let right = self.parse_precedence(precedence + 1)?;
            let span = left.span.join(right.span);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Int(value) => {
                let value = value.parse::<i64>().map_err(|_| {
                    ParseError::new("integer literal is out of range for `int`", token.span)
                })?;
                Ok(Expr {
                    kind: ExprKind::Int(value),
                    span: token.span,
                })
            }
            TokenKind::String(value) => Ok(Expr {
                kind: ExprKind::String(value),
                span: token.span,
            }),
            TokenKind::Ident(name) => Ok(Expr {
                kind: ExprKind::Var(name),
                span: token.span,
            }),
            TokenKind::Keyword(Keyword::True) => Ok(Expr {
                kind: ExprKind::Bool(true),
                span: token.span,
            }),
            TokenKind::Keyword(Keyword::False) => Ok(Expr {
                kind: ExprKind::Bool(false),
                span: token.span,
            }),
            TokenKind::Minus => {
                let expr = self.parse_precedence(6)?;
                let span = token.span.join(expr.span);
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Negate,
                        expr: Box::new(expr),
                    },
                    span,
                })
            }
            TokenKind::Bang => {
                let expr = self.parse_precedence(6)?;
                let span = token.span.join(expr.span);
                Ok(Expr {
                    kind: ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                    span,
                })
            }
            TokenKind::LeftParen => {
                let expr = self.parse_expression()?;
                self.expect(TokenTag::RightParen, "expected `)` after expression")?;
                Ok(expr)
            }
            _ => Err(ParseError::new("expected expression", token.span)),
        }
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        self.expect(TokenTag::LeftParen, "expected `(` in call")?;
        let mut args = Vec::new();

        if !self.at(TokenTag::RightParen) {
            loop {
                args.push(self.parse_arg()?);
                if self.eat(TokenTag::Comma).is_none() {
                    break;
                }
            }
        }

        let end = self.expect(TokenTag::RightParen, "expected `)` after call arguments")?;
        let span = callee.span.join(end);
        Ok(Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                args,
            },
            span,
        })
    }

    fn parse_arg(&mut self) -> Result<Arg, ParseError> {
        let (mode, start) = if let Some(span) = self.eat_keyword(Keyword::In) {
            (ArgMode::In, span)
        } else if let Some(span) = self.eat_keyword(Keyword::Mut) {
            (ArgMode::Mut, span)
        } else {
            (ArgMode::Owned, self.peek().span)
        };
        let expr = self.parse_expression()?;
        let span = start.join(expr.span);

        Ok(Arg { mode, expr, span })
    }

    fn peek_binary_op(&self) -> Option<(BinaryOp, u8)> {
        let op = match &self.peek().kind {
            TokenKind::EqualEqual => BinaryOp::Equal,
            TokenKind::BangEqual => BinaryOp::NotEqual,
            TokenKind::Less => BinaryOp::Less,
            TokenKind::LessEqual => BinaryOp::LessEqual,
            TokenKind::Greater => BinaryOp::Greater,
            TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Subtract,
            TokenKind::Star => BinaryOp::Multiply,
            TokenKind::Slash => BinaryOp::Divide,
            TokenKind::Percent => BinaryOp::Remainder,
            _ => return None,
        };
        let precedence = match op {
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => 2,
            BinaryOp::Add | BinaryOp::Subtract => 3,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 4,
        };

        Some((op, precedence))
    }

    fn expect_keyword(
        &mut self,
        keyword: Keyword,
        message: &'static str,
    ) -> Result<Span, ParseError> {
        if self.at_keyword(keyword) {
            Ok(self.advance().span)
        } else {
            Err(ParseError::new(message, self.peek().span))
        }
    }

    fn eat_keyword(&mut self, keyword: Keyword) -> Option<Span> {
        if self.at_keyword(keyword) {
            Some(self.advance().span)
        } else {
            None
        }
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(found) if found == keyword)
    }

    fn expect(&mut self, tag: TokenTag, message: &'static str) -> Result<Span, ParseError> {
        if self.at(tag) {
            Ok(self.advance().span)
        } else {
            Err(ParseError::new(message, self.peek().span))
        }
    }

    fn eat(&mut self, tag: TokenTag) -> Option<Span> {
        if self.at(tag) {
            Some(self.advance().span)
        } else {
            None
        }
    }

    fn expect_ident(&mut self, message: &'static str) -> Result<(String, Span), ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Ident(name) => Ok((name, token.span)),
            _ => Err(ParseError::new(message, token.span)),
        }
    }

    fn at(&self, tag: TokenTag) -> bool {
        tag.matches(&self.peek().kind)
    }

    fn peek_next_is(&self, tag: TokenTag) -> bool {
        self.tokens
            .get(self.cursor + 1)
            .is_some_and(|token| tag.matches(&token.kind))
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.cursor];
        if !matches!(token.kind, TokenKind::Eof) {
            self.cursor += 1;
        }
        token
    }
}

#[derive(Debug, Clone, Copy)]
enum TokenTag {
    Ident,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
    ColonEqual,
    Eof,
}

impl TokenTag {
    fn matches(self, kind: &TokenKind) -> bool {
        matches!(
            (self, kind),
            (Self::Ident, TokenKind::Ident(_))
                | (Self::LeftParen, TokenKind::LeftParen)
                | (Self::RightParen, TokenKind::RightParen)
                | (Self::LeftBrace, TokenKind::LeftBrace)
                | (Self::RightBrace, TokenKind::RightBrace)
                | (Self::Comma, TokenKind::Comma)
                | (Self::Semicolon, TokenKind::Semicolon)
                | (Self::ColonEqual, TokenKind::ColonEqual)
                | (Self::Eof, TokenKind::Eof)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::StmtKind;

    #[test]
    fn parses_first_target_program() {
        let program = parse(
            r#"
func main() {
    x := 10
    y := add(x, 20)
    print(y)
}

func add(a int, b int) int {
    return a + b
}
"#,
        )
        .unwrap();

        assert_eq!(program.functions.len(), 2);
        assert_eq!(program.functions[0].name, "main");
        assert_eq!(program.functions[0].body.statements.len(), 3);
        assert_eq!(program.functions[1].name, "add");
        assert_eq!(program.functions[1].params.len(), 2);
        assert_eq!(
            program.functions[1].return_type.as_ref().unwrap().name,
            "int"
        );
        assert!(matches!(
            program.functions[1].body.statements[0].kind,
            StmtKind::Return { .. }
        ));
    }

    #[test]
    fn respects_operator_precedence() {
        let program = parse("func main() { x := 1 + 2 * 3 }").unwrap();
        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::Binary { op, right, .. } = &expr.kind else {
            panic!("expected binary expression");
        };
        assert_eq!(*op, BinaryOp::Add);
        assert!(matches!(
            right.kind,
            ExprKind::Binary {
                op: BinaryOp::Multiply,
                ..
            }
        ));
    }
}
