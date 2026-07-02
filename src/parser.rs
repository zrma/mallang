use std::fmt;

use crate::{
    ast::{
        Arg, ArgMode, BinaryOp, Block, Expr, ExprKind, FieldDecl, FieldInit, Function, MatchArm,
        MatchPattern, Param, ParamMode, Program, Stmt, StmtKind, StructDecl, TypeRef, UnaryOp,
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
    allow_struct_literals: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            cursor: 0,
            allow_struct_literals: true,
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let start = self.peek().span;
        let mut structs = Vec::new();
        let mut functions = Vec::new();

        while !self.at(TokenTag::Eof) {
            if self.at_keyword(Keyword::Type) {
                structs.push(self.parse_type_decl()?);
            } else if self.at_keyword(Keyword::Func) {
                functions.push(self.parse_function()?);
            } else {
                return Err(ParseError::new(
                    "expected `type` or `func` declaration",
                    self.peek().span,
                ));
            }
        }

        let end = self.peek().span;
        Ok(Program {
            structs,
            functions,
            span: start.join(end),
        })
    }

    fn parse_type_decl(&mut self) -> Result<StructDecl, ParseError> {
        let start = self.expect_keyword(Keyword::Type, "expected `type` declaration")?;
        let (name, _) = self.expect_ident("expected type name")?;
        self.expect_keyword(Keyword::Struct, "expected `struct` after type name")?;
        self.expect(TokenTag::LeftBrace, "expected `{` before struct fields")?;

        let mut fields = Vec::new();
        while !self.at(TokenTag::RightBrace) && !self.at(TokenTag::Eof) {
            fields.push(self.parse_field_decl()?);
            while self.eat(TokenTag::Comma).is_some() || self.eat(TokenTag::Semicolon).is_some() {}
        }

        let end = self.expect(TokenTag::RightBrace, "expected `}` after struct fields")?;
        Ok(StructDecl {
            name,
            fields,
            span: start.join(end),
        })
    }

    fn parse_field_decl(&mut self) -> Result<FieldDecl, ParseError> {
        let (name, start) = self.expect_ident("expected struct field name")?;
        let ty = self.parse_type_ref()?;
        let span = start.join(ty.span);

        Ok(FieldDecl { name, ty, span })
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        let start = self.expect_keyword(Keyword::Func, "expected `func` declaration")?;
        let receiver = if self.at(TokenTag::LeftParen) {
            Some(self.parse_receiver()?)
        } else {
            None
        };
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
            receiver,
            params,
            return_type,
            body,
            span,
        })
    }

    fn parse_receiver(&mut self) -> Result<Param, ParseError> {
        self.expect(TokenTag::LeftParen, "expected `(` before method receiver")?;
        let receiver = self.parse_param()?;
        self.expect(TokenTag::RightParen, "expected `)` after method receiver")?;
        Ok(receiver)
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
        let (name, start) = self.expect_ident("expected type name")?;
        let mut args = Vec::new();
        let mut span = start;

        if self.eat(TokenTag::LeftBracket).is_some() {
            if self.at(TokenTag::RightBracket) {
                return Err(ParseError::new("expected type argument", self.peek().span));
            }

            loop {
                args.push(self.parse_type_ref()?);
                if self.eat(TokenTag::Comma).is_none() {
                    break;
                }
            }

            let end = self.expect(TokenTag::RightBracket, "expected `]` after type arguments")?;
            span = start.join(end);
        }

        Ok(TypeRef { name, args, span })
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

        if self.at_keyword(Keyword::If) {
            return self.parse_if_statement();
        }

        if self.at_keyword(Keyword::Mut)
            || (self.at(TokenTag::Ident) && self.peek_next_is(TokenTag::ColonEqual))
        {
            return self.parse_let_statement();
        }

        if self.at(TokenTag::Ident) && self.peek_next_is(TokenTag::Equal) {
            return self.parse_assign_statement();
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

    fn parse_assign_statement(&mut self) -> Result<Stmt, ParseError> {
        let (name, start) = self.expect_ident("expected assignment target")?;
        self.expect(TokenTag::Equal, "expected `=` in assignment")?;
        let expr = self.parse_expression()?;
        let span = start.join(expr.span);

        Ok(Stmt {
            kind: StmtKind::Assign { name, expr },
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

    fn parse_if_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::If, "expected `if`")?;
        let condition = self.parse_expression_without_struct_literals()?;
        let then_block = self.parse_block()?;
        let mut span = start.join(then_block.span);
        let else_block = if self.eat_keyword(Keyword::Else).is_some() {
            let block = self.parse_block()?;
            span = start.join(block.span);
            Some(block)
        } else {
            None
        };

        Ok(Stmt {
            kind: StmtKind::If {
                condition,
                then_block,
                else_block,
            },
            span,
        })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_precedence(0)
    }

    fn parse_expression_without_struct_literals(&mut self) -> Result<Expr, ParseError> {
        let previous = self.allow_struct_literals;
        self.allow_struct_literals = false;
        let result = self.parse_expression();
        self.allow_struct_literals = previous;
        result
    }

    fn parse_precedence(&mut self, min_precedence: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            if self.at(TokenTag::LeftParen) {
                left = self.finish_call(left)?;
                continue;
            }

            if self.at(TokenTag::Dot) {
                left = self.finish_field_access(left)?;
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
            TokenKind::Ident(name) => {
                if self.allow_struct_literals && self.at(TokenTag::LeftBrace) {
                    self.finish_struct_literal(name, token.span)
                } else {
                    Ok(Expr {
                        kind: ExprKind::Var(name),
                        span: token.span,
                    })
                }
            }
            TokenKind::Keyword(Keyword::True) => Ok(Expr {
                kind: ExprKind::Bool(true),
                span: token.span,
            }),
            TokenKind::Keyword(Keyword::False) => Ok(Expr {
                kind: ExprKind::Bool(false),
                span: token.span,
            }),
            TokenKind::Keyword(Keyword::Nil) => Ok(Expr {
                kind: ExprKind::Nil,
                span: token.span,
            }),
            TokenKind::Keyword(Keyword::If) => self.finish_if_expr(token.span),
            TokenKind::Keyword(Keyword::Match) => self.finish_match_expr(token.span),
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

    fn finish_if_expr(&mut self, start: Span) -> Result<Expr, ParseError> {
        let condition = self.parse_expression_without_struct_literals()?;
        let (then_branch, _) = self.parse_if_branch_expr()?;
        self.expect_keyword(Keyword::Else, "expected `else` in if expression")?;
        let (else_branch, end) = self.parse_if_branch_expr()?;
        let span = start.join(end);

        Ok(Expr {
            kind: ExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            span,
        })
    }

    fn parse_if_branch_expr(&mut self) -> Result<(Expr, Span), ParseError> {
        self.expect(TokenTag::LeftBrace, "expected `{` before if branch")?;
        if self.at(TokenTag::RightBrace) {
            return Err(ParseError::new(
                "expected expression in if branch",
                self.peek().span,
            ));
        }

        let expr = self.parse_expression()?;
        while self.eat(TokenTag::Semicolon).is_some() {}
        let end = self.expect(TokenTag::RightBrace, "expected `}` after if branch")?;

        Ok((expr, end))
    }

    fn finish_match_expr(&mut self, start: Span) -> Result<Expr, ParseError> {
        let scrutinee = self.parse_expression_without_struct_literals()?;
        self.expect(TokenTag::LeftBrace, "expected `{` before match arms")?;
        let mut arms = Vec::new();

        while !self.at(TokenTag::RightBrace) && !self.at(TokenTag::Eof) {
            arms.push(self.parse_match_arm()?);
        }

        let end = self.expect(TokenTag::RightBrace, "expected `}` after match arms")?;
        Ok(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: start.join(end),
        })
    }

    fn finish_struct_literal(
        &mut self,
        type_name: String,
        start: Span,
    ) -> Result<Expr, ParseError> {
        self.expect(
            TokenTag::LeftBrace,
            "expected `{` before struct literal fields",
        )?;
        let mut fields = Vec::new();

        while !self.at(TokenTag::RightBrace) && !self.at(TokenTag::Eof) {
            fields.push(self.parse_field_init()?);
            if self.eat(TokenTag::Comma).is_none() {
                break;
            }
        }

        let end = self.expect(
            TokenTag::RightBrace,
            "expected `}` after struct literal fields",
        )?;
        Ok(Expr {
            kind: ExprKind::StructLiteral { type_name, fields },
            span: start.join(end),
        })
    }

    fn parse_field_init(&mut self) -> Result<FieldInit, ParseError> {
        let (name, start) = self.expect_ident("expected struct literal field name")?;
        self.expect(
            TokenTag::Colon,
            "expected `:` after struct literal field name",
        )?;
        let expr = self.parse_expression()?;
        let span = start.join(expr.span);

        Ok(FieldInit { name, expr, span })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, ParseError> {
        let start = self.expect_keyword(Keyword::Case, "expected `case` in match")?;
        let pattern = self.parse_match_pattern()?;
        let (expr, end) = self.parse_match_branch_expr()?;

        Ok(MatchArm {
            pattern,
            expr,
            span: start.join(end),
        })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, ParseError> {
        let (name, span) = self.expect_ident("expected match pattern")?;
        match name.as_str() {
            "None" => Ok(MatchPattern::None),
            "Some" => {
                let binding = self.parse_payload_pattern("Some")?;
                Ok(MatchPattern::Some(binding))
            }
            "Ok" => {
                let binding = self.parse_payload_pattern("Ok")?;
                Ok(MatchPattern::Ok(binding))
            }
            "Err" => {
                let binding = self.parse_payload_pattern("Err")?;
                Ok(MatchPattern::Err(binding))
            }
            _ => Err(ParseError::new(
                "expected `Some`, `None`, `Ok`, or `Err` pattern",
                span,
            )),
        }
    }

    fn parse_payload_pattern(&mut self, constructor: &str) -> Result<String, ParseError> {
        self.expect(TokenTag::LeftParen, "expected `(` in payload pattern")?;
        let (binding, _) = self.expect_ident("expected payload binding")?;
        self.expect(TokenTag::RightParen, "expected `)` after payload pattern")?;
        if matches!(binding.as_str(), "Some" | "None" | "Ok" | "Err") {
            return Err(ParseError::new(
                format!("`{binding}` cannot be used as a `{constructor}` payload binding"),
                self.peek().span,
            ));
        }
        Ok(binding)
    }

    fn parse_match_branch_expr(&mut self) -> Result<(Expr, Span), ParseError> {
        self.expect(TokenTag::LeftBrace, "expected `{` before match branch")?;
        if self.at(TokenTag::RightBrace) {
            return Err(ParseError::new(
                "expected expression in match branch",
                self.peek().span,
            ));
        }

        let expr = self.parse_expression()?;
        while self.eat(TokenTag::Semicolon).is_some() {}
        let end = self.expect(TokenTag::RightBrace, "expected `}` after match branch")?;

        Ok((expr, end))
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

    fn finish_field_access(&mut self, base: Expr) -> Result<Expr, ParseError> {
        self.expect(TokenTag::Dot, "expected `.` in field access")?;
        let (field, end) = self.expect_ident("expected field name after `.`")?;
        let span = base.span.join(end);

        Ok(Expr {
            kind: ExprKind::FieldAccess {
                base: Box::new(base),
                field,
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
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    ColonEqual,
    Equal,
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
                | (Self::LeftBracket, TokenKind::LeftBracket)
                | (Self::RightBracket, TokenKind::RightBracket)
                | (Self::Comma, TokenKind::Comma)
                | (Self::Dot, TokenKind::Dot)
                | (Self::Colon, TokenKind::Colon)
                | (Self::Semicolon, TokenKind::Semicolon)
                | (Self::ColonEqual, TokenKind::ColonEqual)
                | (Self::Equal, TokenKind::Equal)
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

    #[test]
    fn parses_assignment_and_nil_expression() {
        let program = parse("func main() { mut x := nil x = 2 }").unwrap();
        assert!(matches!(
            program.functions[0].body.statements[0].kind,
            StmtKind::Let { .. }
        ));
        assert!(matches!(
            program.functions[0].body.statements[1].kind,
            StmtKind::Assign { .. }
        ));
    }

    #[test]
    fn parses_if_expression() {
        let program = parse(
            r#"
func main() {
    label := if 1 < 2 { "yes" } else { "no" }
}
"#,
        )
        .unwrap();

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } = &expr.kind
        else {
            panic!("expected if expression");
        };
        assert!(matches!(condition.kind, ExprKind::Binary { .. }));
        assert!(matches!(then_branch.kind, ExprKind::String(_)));
        assert!(matches!(else_branch.kind, ExprKind::String(_)));
    }

    #[test]
    fn parses_if_statement() {
        let program = parse(
            r#"
func main() {
    if 1 < 2 {
        print("yes")
    } else {
        print("no")
    }
}
"#,
        )
        .unwrap();

        let StmtKind::If {
            condition,
            then_block,
            else_block,
        } = &program.functions[0].body.statements[0].kind
        else {
            panic!("expected if statement");
        };
        assert!(matches!(condition.kind, ExprKind::Binary { .. }));
        assert_eq!(then_block.statements.len(), 1);
        assert_eq!(else_block.as_ref().unwrap().statements.len(), 1);
    }

    #[test]
    fn parses_struct_declaration_literal_and_field_access() {
        let program = parse(
            r#"
type User struct {
    name string
    age int
}

func main() {
    user := User{name: "kim", age: 30}
    print(user.age)
}
"#,
        )
        .unwrap();

        assert_eq!(program.structs.len(), 1);
        assert_eq!(program.structs[0].name, "User");
        assert_eq!(program.structs[0].fields.len(), 2);

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::StructLiteral { type_name, fields } = &expr.kind else {
            panic!("expected struct literal");
        };
        assert_eq!(type_name, "User");
        assert_eq!(fields.len(), 2);

        let StmtKind::Expr { expr } = &program.functions[0].body.statements[1].kind else {
            panic!("expected expression statement");
        };
        let ExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected print call");
        };
        assert!(matches!(args[0].expr.kind, ExprKind::FieldAccess { .. }));
    }

    #[test]
    fn parses_method_declaration_and_call() {
        let program = parse(
            r#"
type User struct {
    name string
    age int
}

func (self in User) age() int {
    return self.age
}

func main() {
    user := User{name: "kim", age: 30}
    print(user.age())
}
"#,
        )
        .unwrap();

        assert_eq!(program.functions.len(), 2);
        let receiver = program.functions[0].receiver.as_ref().unwrap();
        assert_eq!(receiver.name, "self");
        assert_eq!(receiver.mode, ParamMode::In);
        assert_eq!(receiver.ty.name, "User");

        let StmtKind::Expr { expr } = &program.functions[1].body.statements[1].kind else {
            panic!("expected expression statement");
        };
        let ExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected print call");
        };
        let ExprKind::Call { callee, .. } = &args[0].expr.kind else {
            panic!("expected method call");
        };
        assert!(matches!(callee.kind, ExprKind::FieldAccess { .. }));
    }

    #[test]
    fn parses_generic_type_refs() {
        let program = parse(
            r#"
func find() Option[int] {
    return None
}

func read() Result[string, int] {
    return Err(1)
}

func main() {}
"#,
        )
        .unwrap();

        let option_ty = program.functions[0].return_type.as_ref().unwrap();
        assert_eq!(option_ty.name, "Option");
        assert_eq!(option_ty.args.len(), 1);
        assert_eq!(option_ty.args[0].name, "int");

        let result_ty = program.functions[1].return_type.as_ref().unwrap();
        assert_eq!(result_ty.name, "Result");
        assert_eq!(result_ty.args.len(), 2);
        assert_eq!(result_ty.args[0].name, "string");
        assert_eq!(result_ty.args[1].name, "int");
    }

    #[test]
    fn parses_match_expression() {
        let program = parse(
            r#"
func main() {
    value := Some(1)
    out := match value {
        case Some(x) { x }
        case None { 0 }
    }
}
"#,
        )
        .unwrap();

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[1].kind else {
            panic!("expected let statement");
        };
        let ExprKind::Match { scrutinee, arms } = &expr.kind else {
            panic!("expected match expression");
        };
        assert!(matches!(scrutinee.kind, ExprKind::Var(_)));
        assert_eq!(arms.len(), 2);
        assert!(matches!(arms[0].pattern, MatchPattern::Some(_)));
        assert!(matches!(arms[1].pattern, MatchPattern::None));
    }
}
