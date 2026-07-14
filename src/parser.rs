use std::fmt;

use crate::{
    ast::{
        Arg, ArgMode, BinaryOp, Block, Expr, ExprKind, FieldDecl, FieldInit, ForInit, ForPost,
        Function, FunctionLiteral, FunctionTypeParam, FunctionTypeRef, ImportDecl, MatchArm,
        MatchBlockArm, MatchPattern, PackageDecl, Param, ParamMode, Program, SourceUnit, Stmt,
        StmtKind, StructDecl, TypeRef, UnaryOp, Visibility,
    },
    lexer::{lex, lex_with_source, LexError},
    token::{Keyword, SourceId, Span, Token, TokenKind},
};

pub fn parse(source: &str) -> Result<Program, ParseError> {
    let tokens = lex(source).map_err(ParseError::from_lex)?;
    Parser::new(tokens).parse_program()
}

pub fn parse_with_source(source: &str, source_id: SourceId) -> Result<Program, ParseError> {
    let tokens = lex_with_source(source, source_id).map_err(ParseError::from_lex)?;
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
        let package = if self.at_keyword(Keyword::Package) {
            Some(self.parse_package_decl()?)
        } else {
            None
        };
        if package.is_none() && self.at_keyword(Keyword::Import) {
            return Err(ParseError::new(
                "import declarations require a package declaration",
                self.peek().span,
            ));
        }

        let mut imports = Vec::new();
        while self.at_keyword(Keyword::Import) {
            imports.push(self.parse_import_decl()?);
        }

        let mut structs = Vec::new();
        let mut functions = Vec::new();

        while !self.at(TokenTag::Eof) {
            let public_span = self.eat_keyword(Keyword::Pub);
            let visibility = if public_span.is_some() {
                Visibility::Public
            } else {
                Visibility::Package
            };

            if self.at_keyword(Keyword::Type) {
                structs.push(self.parse_type_decl(visibility, public_span)?);
            } else if self.at_keyword(Keyword::Func) {
                functions.push(self.parse_function(visibility, public_span)?);
            } else if public_span.is_some() {
                return Err(ParseError::new(
                    "expected `type` or `func` declaration after `pub`",
                    self.peek().span,
                ));
            } else if self.at_keyword(Keyword::Package) {
                return Err(ParseError::new(
                    "package declaration must appear first",
                    self.peek().span,
                ));
            } else if self.at_keyword(Keyword::Import) {
                return Err(ParseError::new(
                    "import declarations must appear before top-level declarations",
                    self.peek().span,
                ));
            } else {
                return Err(ParseError::new(
                    "expected `type`, `func`, or `pub` declaration",
                    self.peek().span,
                ));
            }
        }

        let end = self.peek().span;
        let span = start.join(end);
        Ok(Program {
            source_units: vec![SourceUnit {
                package,
                imports,
                span,
            }],
            structs,
            functions,
            source_spans: vec![span],
            span,
        })
    }

    fn parse_package_decl(&mut self) -> Result<PackageDecl, ParseError> {
        let start = self.expect_keyword(Keyword::Package, "expected `package` declaration")?;
        let (name, end) = self.expect_ident("expected package name")?;

        Ok(PackageDecl {
            name,
            span: start.join(end),
        })
    }

    fn parse_import_decl(&mut self) -> Result<ImportDecl, ParseError> {
        let start = self.expect_keyword(Keyword::Import, "expected `import` declaration")?;
        let (path, end) = self.expect_string("expected import path string")?;

        Ok(ImportDecl {
            path,
            span: start.join(end),
        })
    }

    fn parse_type_decl(
        &mut self,
        visibility: Visibility,
        public_span: Option<Span>,
    ) -> Result<StructDecl, ParseError> {
        let type_span = self.expect_keyword(Keyword::Type, "expected `type` declaration")?;
        let start = public_span.unwrap_or(type_span);
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
            visibility,
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

    fn parse_function(
        &mut self,
        visibility: Visibility,
        public_span: Option<Span>,
    ) -> Result<Function, ParseError> {
        let func_span = self.expect_keyword(Keyword::Func, "expected `func` declaration")?;
        let start = public_span.unwrap_or(func_span);
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
        let return_type = if self.starts_type_ref() {
            Some(self.parse_type_ref()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = start.join(body.span);

        Ok(Function {
            visibility,
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
        let prefix_mode = self.eat_param_mode();
        let (name, name_span) = self.expect_ident("expected parameter name")?;
        let (mode, start) = prefix_mode.unwrap_or((ParamMode::Owned, name_span));
        let ty = self.parse_type_ref()?;
        let span = start.join(ty.span);

        Ok(Param {
            name,
            mode,
            ty,
            span,
        })
    }

    fn eat_param_mode(&mut self) -> Option<(ParamMode, Span)> {
        if let Some(span) = self.eat_keyword(Keyword::Con) {
            Some((ParamMode::Con, span))
        } else {
            self.eat_keyword(Keyword::Mut)
                .map(|span| (ParamMode::Mut, span))
        }
    }

    fn parse_type_ref(&mut self) -> Result<TypeRef, ParseError> {
        if self.at_keyword(Keyword::Func) {
            return self.parse_function_type_ref();
        }
        if self.at(TokenTag::LeftBracket) {
            return self.parse_array_type_ref();
        }

        let (mut name, start) = self.expect_ident("expected type name")?;
        let mut args = Vec::new();
        let mut span = start;

        if self.eat(TokenTag::Dot).is_some() {
            let (selected, end) = self.expect_ident("expected type name after `.`")?;
            name.push('.');
            name.push_str(&selected);
            span = start.join(end);
        }

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

        Ok(TypeRef {
            name,
            args,
            array_len: None,
            slice: false,
            function: None,
            span,
        })
    }

    fn parse_function_type_ref(&mut self) -> Result<TypeRef, ParseError> {
        let start = self.expect_keyword(Keyword::Func, "expected `func` in function type")?;
        let mutable = self.eat_keyword(Keyword::Mut).is_some();
        self.expect(TokenTag::LeftParen, "expected `(` after function type")?;

        let mut params = Vec::new();
        if !self.at(TokenTag::RightParen) {
            loop {
                let prefix_mode = self.eat_param_mode();
                let ty = self.parse_type_ref()?;
                let (mode, param_start) = prefix_mode.unwrap_or((ParamMode::Owned, ty.span));
                params.push(FunctionTypeParam {
                    mode,
                    span: param_start.join(ty.span),
                    ty,
                });
                if self.eat(TokenTag::Comma).is_none() {
                    break;
                }
            }
        }

        let params_end = self.expect(
            TokenTag::RightParen,
            "expected `)` after function type parameters",
        )?;
        if !self.starts_type_ref() {
            return Err(ParseError::new(
                "expected function return type; use `unit` for no value",
                params_end,
            ));
        }
        let return_type = Box::new(self.parse_type_ref()?);
        let end = return_type.span;
        let span = start.join(end);

        Ok(TypeRef {
            name: "func".to_string(),
            args: Vec::new(),
            array_len: None,
            slice: false,
            function: Some(FunctionTypeRef {
                mutable,
                params,
                return_type,
                span,
            }),
            span,
        })
    }

    fn parse_array_type_ref(&mut self) -> Result<TypeRef, ParseError> {
        let start = self.expect(TokenTag::LeftBracket, "expected `[` before array length")?;
        if self.eat(TokenTag::RightBracket).is_some() {
            let element = self.parse_type_ref()?;
            let span = start.join(element.span);
            return Ok(TypeRef {
                name: "Slice".to_string(),
                args: vec![element],
                array_len: None,
                slice: true,
                function: None,
                span,
            });
        }

        let token = self.advance().clone();
        let TokenKind::Int(length) = token.kind else {
            return Err(ParseError::new(
                "expected array length or `]` for slice type",
                token.span,
            ));
        };
        let length = length
            .parse::<usize>()
            .map_err(|_| ParseError::new("array length is out of range", token.span))?;
        self.expect(TokenTag::RightBracket, "expected `]` after array length")?;
        let element = self.parse_type_ref()?;
        let span = start.join(element.span);

        Ok(TypeRef {
            name: "Array".to_string(),
            args: vec![element],
            array_len: Some(length),
            slice: false,
            function: None,
            span,
        })
    }

    fn starts_type_ref(&self) -> bool {
        self.at(TokenTag::Ident) || self.at(TokenTag::LeftBracket) || self.at_keyword(Keyword::Func)
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

        if self.at_keyword(Keyword::For) {
            return self.parse_for_statement();
        }

        if self.at_keyword(Keyword::Break) {
            return self.parse_break_statement();
        }

        if self.at_keyword(Keyword::Continue) {
            return self.parse_continue_statement();
        }

        if self.at_keyword(Keyword::Match) {
            return self.parse_match_statement();
        }

        if self.at_keyword(Keyword::Mut)
            || (self.at(TokenTag::Ident) && self.peek_next_is(TokenTag::ColonEqual))
        {
            return self.parse_let_statement();
        }

        let expr = self.parse_expression()?;
        if self.at(TokenTag::Equal) {
            return self.finish_assign_statement(expr);
        }
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

    fn finish_assign_statement(&mut self, target: Expr) -> Result<Stmt, ParseError> {
        let start = target.span;
        self.expect(TokenTag::Equal, "expected `=` in assignment")?;
        let expr = self.parse_expression()?;
        let span = start.join(expr.span);

        let kind = match target.kind {
            ExprKind::Var(name) => StmtKind::Assign { name, expr },
            ExprKind::FieldAccess { base, field } => StmtKind::FieldAssign {
                base: *base,
                field,
                expr,
            },
            ExprKind::Index { base, index } => StmtKind::IndexAssign {
                base: *base,
                index: *index,
                expr,
            },
            _ => {
                return Err(ParseError::new(
                    "assignment target must be a variable, field access, or index expression",
                    start,
                ));
            }
        };

        Ok(Stmt { kind, span })
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
        let else_block = if let Some(else_start) = self.eat_keyword(Keyword::Else) {
            if self.at_keyword(Keyword::If) {
                let nested = self.parse_if_statement()?;
                span = start.join(nested.span);
                Some(Block {
                    span: else_start.join(nested.span),
                    statements: vec![nested],
                })
            } else {
                let block = self.parse_block()?;
                span = start.join(block.span);
                Some(block)
            }
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

    fn parse_for_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::For, "expected `for`")?;

        if self.at(TokenTag::LeftBrace) {
            let body = self.parse_block()?;
            let span = start.join(body.span);

            return Ok(Stmt {
                kind: StmtKind::For {
                    init: None,
                    condition: None,
                    post: None,
                    body,
                },
                span,
            });
        }

        if self.starts_range_header() {
            let (index_name, _) = self.expect_ident("expected range index binding name")?;
            let value_name = if self.eat(TokenTag::Comma).is_some() {
                let (value_name, _) = self.expect_ident("expected range value binding name")?;
                value_name
            } else {
                "_".to_string()
            };
            self.expect(TokenTag::ColonEqual, "expected `:=` in range loop")?;
            self.expect_keyword(Keyword::Range, "expected `range` in range loop")?;
            let source = self.parse_expression_without_struct_literals()?;
            let body = self.parse_block()?;
            let span = start.join(body.span);

            return Ok(Stmt {
                kind: StmtKind::RangeFor {
                    index_name,
                    value_name,
                    source,
                    body,
                },
                span,
            });
        }

        if self.at(TokenTag::Semicolon) || self.starts_for_clause_header() {
            let init = if self.eat(TokenTag::Semicolon).is_some() {
                None
            } else {
                let init = self.parse_for_init()?;
                self.expect(TokenTag::Semicolon, "expected `;` after for init")?;
                Some(init)
            };
            let condition = if self.at(TokenTag::Semicolon) {
                None
            } else {
                Some(self.parse_expression_without_struct_literals()?)
            };
            self.expect(TokenTag::Semicolon, "expected `;` after for condition")?;
            let post = if self.at(TokenTag::LeftBrace) {
                None
            } else {
                Some(self.parse_for_post()?)
            };
            let body = self.parse_block()?;
            let span = start.join(body.span);

            return Ok(Stmt {
                kind: StmtKind::For {
                    init,
                    condition,
                    post,
                    body,
                },
                span,
            });
        }

        let condition = Some(self.parse_expression_without_struct_literals()?);
        let body = self.parse_block()?;
        let span = start.join(body.span);

        Ok(Stmt {
            kind: StmtKind::For {
                init: None,
                condition,
                post: None,
                body,
            },
            span,
        })
    }

    fn starts_range_header(&self) -> bool {
        self.at(TokenTag::Ident)
            && (self.peek_next_is(TokenTag::Comma)
                || (self.peek_next_is(TokenTag::ColonEqual)
                    && self.peek_second_is_keyword(Keyword::Range)))
    }

    fn starts_for_clause_header(&self) -> bool {
        self.at_keyword(Keyword::Mut)
            || (self.at(TokenTag::Ident) && self.peek_next_is(TokenTag::ColonEqual))
    }

    fn parse_for_init(&mut self) -> Result<ForInit, ParseError> {
        let mutable = self.eat_keyword(Keyword::Mut).is_some();
        let (name, _) = self.expect_ident("expected for init binding name")?;
        self.expect(TokenTag::ColonEqual, "expected `:=` in for init")?;
        let expr = self.parse_expression()?;

        Ok(ForInit::Let {
            mutable,
            name,
            expr,
        })
    }

    fn parse_for_post(&mut self) -> Result<ForPost, ParseError> {
        let target = self.parse_expression()?;
        let target_span = target.span;
        self.expect(TokenTag::Equal, "expected `=` in for post")?;
        let expr = self.parse_expression_without_struct_literals()?;

        match target.kind {
            ExprKind::Var(_) | ExprKind::FieldAccess { .. } | ExprKind::Index { .. } => {
                Ok(ForPost::Assign { target, expr })
            }
            _ => Err(ParseError::new(
                "for post target must be a variable, field access, or index expression",
                target_span,
            )),
        }
    }

    fn parse_break_statement(&mut self) -> Result<Stmt, ParseError> {
        let span = self.expect_keyword(Keyword::Break, "expected `break`")?;
        Ok(Stmt {
            kind: StmtKind::Break,
            span,
        })
    }

    fn parse_continue_statement(&mut self) -> Result<Stmt, ParseError> {
        let span = self.expect_keyword(Keyword::Continue, "expected `continue`")?;
        Ok(Stmt {
            kind: StmtKind::Continue,
            span,
        })
    }

    fn parse_match_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::Match, "expected `match`")?;
        let scrutinee = self.parse_expression_without_struct_literals()?;
        self.expect(TokenTag::LeftBrace, "expected `{` before match arms")?;
        let mut arms = Vec::new();

        while !self.at(TokenTag::RightBrace) && !self.at(TokenTag::Eof) {
            arms.push(self.parse_match_block_arm()?);
        }

        let end = self.expect(TokenTag::RightBrace, "expected `}` after match arms")?;
        Ok(Stmt {
            kind: StmtKind::Match { scrutinee, arms },
            span: start.join(end),
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

            if self.at(TokenTag::LeftBracket) {
                left = self.finish_index(left)?;
                continue;
            }

            if self.at(TokenTag::Dot) {
                left = self.finish_field_access(left)?;
                continue;
            }

            if self.at(TokenTag::PipeGreater) {
                const PIPELINE_PRECEDENCE: u8 = 0;
                if PIPELINE_PRECEDENCE < min_precedence {
                    break;
                }
                left = self.finish_pipeline(left)?;
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
        if self.at(TokenTag::LeftBracket) {
            return self.parse_array_literal();
        }

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
                if self.allow_struct_literals
                    && self.at(TokenTag::Dot)
                    && self.peek_next_is(TokenTag::Ident)
                    && self.peek_n_is(2, TokenTag::LeftBrace)
                {
                    self.advance();
                    let (selected, _) = self.expect_ident("expected struct type name after `.`")?;
                    self.finish_struct_literal(format!("{name}.{selected}"), token.span)
                } else if self.allow_struct_literals && self.at(TokenTag::LeftBrace) {
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
            TokenKind::Keyword(Keyword::Func) => self.finish_function_literal(token.span),
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

    fn finish_function_literal(&mut self, start: Span) -> Result<Expr, ParseError> {
        let mutable = self.eat_keyword(Keyword::Mut).is_some();
        self.expect(TokenTag::LeftParen, "expected `(` after `func`")?;

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
            "expected `)` after closure parameters",
        )?;
        let return_type = if self.starts_type_ref() {
            Some(self.parse_type_ref()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = start.join(body.span);

        Ok(Expr {
            kind: ExprKind::FunctionLiteral(Box::new(FunctionLiteral {
                mutable,
                params,
                return_type,
                body,
            })),
            span,
        })
    }

    fn parse_array_literal(&mut self) -> Result<Expr, ParseError> {
        let ty = self.parse_type_ref()?;
        let start = ty.span;
        self.expect(
            TokenTag::LeftBrace,
            "expected `{` before array literal elements",
        )?;
        let mut elements = Vec::new();

        while !self.at(TokenTag::RightBrace) && !self.at(TokenTag::Eof) {
            elements.push(self.parse_expression()?);
            if self.eat(TokenTag::Comma).is_none() {
                break;
            }
        }

        let end = self.expect(
            TokenTag::RightBrace,
            "expected `}` after array literal elements",
        )?;

        Ok(Expr {
            kind: ExprKind::ArrayLiteral {
                ty: Box::new(ty),
                elements,
            },
            span: start.join(end),
        })
    }

    fn finish_if_expr(&mut self, start: Span) -> Result<Expr, ParseError> {
        let condition = self.parse_expression_without_struct_literals()?;
        let (then_branch, _) = self.parse_if_branch_expr()?;
        self.expect_keyword(Keyword::Else, "expected `else` in if expression")?;
        let (else_branch, end) = if self.at_keyword(Keyword::If) {
            let if_start = self.expect_keyword(Keyword::If, "expected `if` after `else`")?;
            let expr = self.finish_if_expr(if_start)?;
            let end = expr.span;
            (expr, end)
        } else {
            self.parse_if_branch_expr()?
        };
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

    fn parse_match_block_arm(&mut self) -> Result<MatchBlockArm, ParseError> {
        let start = self.expect_keyword(Keyword::Case, "expected `case` in match")?;
        let pattern = self.parse_match_pattern()?;
        let block = self.parse_block()?;

        Ok(MatchBlockArm {
            pattern,
            span: start.join(block.span),
            block,
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

    fn finish_pipeline(&mut self, input: Expr) -> Result<Expr, ParseError> {
        self.expect(
            TokenTag::PipeGreater,
            "expected `|>` in pipeline expression",
        )?;
        let input_span = input.span;
        let callee = self.parse_prefix()?;
        if !self.at(TokenTag::LeftParen) {
            return Err(ParseError::new(
                "pipeline target must be a call like `value |> f(args...)`",
                callee.span,
            ));
        }

        let mut call = self.finish_call(callee)?;
        let span = input_span.join(call.span);
        let ExprKind::Call { args, .. } = &mut call.kind else {
            unreachable!("finish_call always returns a call expression");
        };
        args.insert(
            0,
            Arg {
                mode: ArgMode::Owned,
                span: input_span,
                expr: input,
            },
        );
        call.span = span;

        Ok(call)
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

    fn finish_index(&mut self, base: Expr) -> Result<Expr, ParseError> {
        self.expect(TokenTag::LeftBracket, "expected `[` in index expression")?;
        let index = self.parse_expression()?;
        let end = self.expect(
            TokenTag::RightBracket,
            "expected `]` after index expression",
        )?;
        let span = base.span.join(end);

        Ok(Expr {
            kind: ExprKind::Index {
                base: Box::new(base),
                index: Box::new(index),
            },
            span,
        })
    }

    fn parse_arg(&mut self) -> Result<Arg, ParseError> {
        let (mode, start) = if let Some(span) = self.eat_keyword(Keyword::Con) {
            (ArgMode::Con, span)
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
            TokenKind::AmpAmp => BinaryOp::LogicalAnd,
            TokenKind::PipePipe => BinaryOp::LogicalOr,
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
            BinaryOp::LogicalOr => 1,
            BinaryOp::LogicalAnd => 2,
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => 3,
            BinaryOp::Add | BinaryOp::Subtract => 4,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 5,
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

    fn expect_string(&mut self, message: &'static str) -> Result<(String, Span), ParseError> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(value) => Ok((value, token.span)),
            _ => Err(ParseError::new(message, token.span)),
        }
    }

    fn at(&self, tag: TokenTag) -> bool {
        tag.matches(&self.peek().kind)
    }

    fn peek_next_is(&self, tag: TokenTag) -> bool {
        self.peek_n_is(1, tag)
    }

    fn peek_n_is(&self, offset: usize, tag: TokenTag) -> bool {
        self.tokens
            .get(self.cursor + offset)
            .is_some_and(|token| tag.matches(&token.kind))
    }

    fn peek_second_is_keyword(&self, keyword: Keyword) -> bool {
        matches!(
            self.tokens.get(self.cursor + 2).map(|token| &token.kind),
            Some(TokenKind::Keyword(found)) if *found == keyword
        )
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
    PipeGreater,
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
                | (Self::PipeGreater, TokenKind::PipeGreater)
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
    fn parses_package_import_and_public_declarations() {
        let source = r#"
package main

import "hello/greet"

pub type Message struct {
    text string
}

pub func (con self Message) Print() {
    print(self.text)
}

func main() {
    greet.Print()
}
"#;
        let program = parse(source).unwrap();
        let unit = &program.source_units[0];

        assert_eq!(
            unit.package.as_ref().map(|package| package.name.as_str()),
            Some("main")
        );
        assert_eq!(unit.imports.len(), 1);
        assert_eq!(unit.imports[0].path, "hello/greet");
        assert_eq!(program.structs[0].visibility, Visibility::Public);
        assert_eq!(
            program.structs[0].span.start,
            source.find("pub type").unwrap()
        );
        assert_eq!(program.functions[0].visibility, Visibility::Public);
        assert!(program.functions[0].receiver.is_some());
        assert_eq!(program.functions[1].visibility, Visibility::Package);
    }

    #[test]
    fn parses_qualified_types_and_struct_literals() {
        let program = parse(
            r#"
package main
import "hello/greet"

func echo(message greet.Message) greet.Message {
    return greet.Message{text: message.text}
}

func main() {}
"#,
        )
        .unwrap();

        assert_eq!(program.functions[0].params[0].ty.name, "greet.Message");
        assert_eq!(
            program.functions[0].return_type.as_ref().unwrap().name,
            "greet.Message"
        );
        let StmtKind::Return { expr } = &program.functions[0].body.statements[0].kind else {
            panic!("expected return statement");
        };
        assert!(matches!(
            &expr.kind,
            ExprKind::StructLiteral { type_name, .. } if type_name == "greet.Message"
        ));
    }

    #[test]
    fn keeps_standalone_declarations_package_private() {
        let program = parse("type Message struct {}\nfunc main() {}\n").unwrap();

        assert!(program.source_units[0].package.is_none());
        assert!(program.source_units[0].imports.is_empty());
        assert_eq!(program.structs[0].visibility, Visibility::Package);
        assert_eq!(program.functions[0].visibility, Visibility::Package);
    }

    #[test]
    fn rejects_invalid_project_header_order() {
        let missing_package = parse("import \"hello/greet\"\nfunc main() {}\n").unwrap_err();
        assert_eq!(
            missing_package.message,
            "import declarations require a package declaration"
        );

        let late_package = parse("func main() {}\npackage main\n").unwrap_err();
        assert_eq!(
            late_package.message,
            "package declaration must appear first"
        );

        let late_import =
            parse("package main\nfunc main() {}\nimport \"hello/greet\"\n").unwrap_err();
        assert_eq!(
            late_import.message,
            "import declarations must appear before top-level declarations"
        );
    }

    #[test]
    fn restricts_pub_to_top_level_type_and_function_declarations() {
        let error = parse("package main\npub package other\n").unwrap_err();

        assert_eq!(
            error.message,
            "expected `type` or `func` declaration after `pub`"
        );
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
    fn respects_logical_operator_precedence() {
        let program = parse("func main() { x := a || b && c == d }").unwrap();
        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::Binary { op, right, .. } = &expr.kind else {
            panic!("expected binary expression");
        };
        assert_eq!(*op, BinaryOp::LogicalOr);
        let ExprKind::Binary {
            op: right_op,
            right: and_right,
            ..
        } = &right.kind
        else {
            panic!("expected logical and expression");
        };
        assert_eq!(*right_op, BinaryOp::LogicalAnd);
        assert!(matches!(
            and_right.kind,
            ExprKind::Binary {
                op: BinaryOp::Equal,
                ..
            }
        ));
    }

    #[test]
    fn parses_unary_not_before_logical_and() {
        let program = parse("func main() { x := !a && b }").unwrap();
        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::Binary { op, left, .. } = &expr.kind else {
            panic!("expected logical and expression");
        };
        assert_eq!(*op, BinaryOp::LogicalAnd);
        assert!(matches!(
            left.kind,
            ExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn parses_pipeline_expression_as_call_sugar() {
        let program = parse("func main() { x := 1 + 2 |> double() |> add(3) }").unwrap();
        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::Call { callee, args } = &expr.kind else {
            panic!("expected outer call");
        };
        assert!(matches!(&callee.kind, ExprKind::Var(name) if name == "add"));
        assert_eq!(args.len(), 2);
        assert!(matches!(args[1].expr.kind, ExprKind::Int(3)));

        let ExprKind::Call {
            callee: inner_callee,
            args: inner_args,
        } = &args[0].expr.kind
        else {
            panic!("expected piped inner call");
        };
        assert!(matches!(&inner_callee.kind, ExprKind::Var(name) if name == "double"));
        assert_eq!(inner_args.len(), 1);
        assert!(matches!(
            inner_args[0].expr.kind,
            ExprKind::Binary {
                op: BinaryOp::Add,
                ..
            }
        ));
    }

    #[test]
    fn rejects_pipeline_target_without_call() {
        let error = parse("func main() { x := 1 |> double }").unwrap_err();
        assert!(error.message.contains("pipeline target must be a call"));
    }

    #[test]
    fn parses_for_statement() {
        let program = parse("func main() { for keepGoing { tick() } }").unwrap();
        let StmtKind::For {
            init,
            condition,
            post,
            body,
        } = &program.functions[0].body.statements[0].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        assert!(post.is_none());
        let condition = condition.as_ref().expect("expected for condition");
        assert!(matches!(&condition.kind, ExprKind::Var(name) if name == "keepGoing"));
        assert_eq!(body.statements.len(), 1);
        assert!(matches!(body.statements[0].kind, StmtKind::Expr { .. }));
    }

    #[test]
    fn parses_infinite_for_statement() {
        let program = parse("func main() { for { tick() } }").unwrap();
        let StmtKind::For {
            init,
            condition,
            post,
            body,
        } = &program.functions[0].body.statements[0].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        assert!(condition.is_none());
        assert!(post.is_none());
        assert_eq!(body.statements.len(), 1);
    }

    #[test]
    fn parses_for_clause_statement() {
        let program =
            parse("func main() { for mut i := 0; i < 3; i = i + 1 { print(i) } }").unwrap();
        let StmtKind::For {
            init,
            condition,
            post,
            body,
        } = &program.functions[0].body.statements[0].kind
        else {
            panic!("expected for statement");
        };
        assert!(matches!(
            init,
            Some(ForInit::Let {
                mutable: true,
                name,
                ..
            }) if name == "i"
        ));
        let condition = condition.as_ref().expect("expected for condition");
        assert!(matches!(
            condition.kind,
            ExprKind::Binary {
                op: BinaryOp::Less,
                ..
            }
        ));
        assert!(matches!(
            post,
            Some(ForPost::Assign {
                target,
                ..
            }) if matches!(&target.kind, ExprKind::Var(name) if name == "i")
        ));
        assert_eq!(body.statements.len(), 1);
    }

    #[test]
    fn parses_initless_for_clause_statement() {
        let program =
            parse("func main() { mut i := 0 for ; i < 3; i = i + 1 { print(i) } }").unwrap();
        let StmtKind::For {
            init,
            condition,
            post,
            ..
        } = &program.functions[0].body.statements[1].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        let condition = condition.as_ref().expect("expected for condition");
        assert!(matches!(
            condition.kind,
            ExprKind::Binary {
                op: BinaryOp::Less,
                ..
            }
        ));
        assert!(matches!(post, Some(ForPost::Assign { .. })));
    }

    #[test]
    fn parses_for_clause_index_assignment_post() {
        let program = parse(
            r#"
func main() {
    mut values := [3]int{0, 0, 0}
    mut slot := 0
    mut i := 0
    for ; i < 3; values[slot] = i {
        slot = i
        i = i + 1
    }
}
"#,
        )
        .unwrap();
        let StmtKind::For { post, .. } = &program.functions[0].body.statements[3].kind else {
            panic!("expected for statement");
        };
        let Some(ForPost::Assign { target, .. }) = post else {
            panic!("expected for post assignment");
        };
        let ExprKind::Index { base, index } = &target.kind else {
            panic!("expected index assignment target");
        };
        assert!(matches!(&base.kind, ExprKind::Var(name) if name == "values"));
        assert!(matches!(&index.kind, ExprKind::Var(name) if name == "slot"));
    }

    #[test]
    fn parses_for_clause_with_empty_condition() {
        let program = parse("func main() { mut i := 0 for ; ; i = i + 1 { print(i) } }").unwrap();
        let StmtKind::For {
            init,
            condition,
            post,
            ..
        } = &program.functions[0].body.statements[1].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        assert!(condition.is_none());
        assert!(matches!(post, Some(ForPost::Assign { .. })));
    }

    #[test]
    fn parses_loop_control_statements() {
        let program = parse("func main() { for true { continue break } }").unwrap();
        let StmtKind::For { body, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected for statement");
        };
        assert!(matches!(body.statements[0].kind, StmtKind::Continue));
        assert!(matches!(body.statements[1].kind, StmtKind::Break));
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
    fn parses_prefix_parameter_modes() {
        let program = parse(
            r#"
func read(con name string) {
    print(name)
}

func rename(mut name string) {
    name = "lee"
}
"#,
        )
        .unwrap();

        assert_eq!(program.functions[0].params[0].name, "name");
        assert_eq!(program.functions[0].params[0].mode, ParamMode::Con);
        assert_eq!(program.functions[1].params[0].name, "name");
        assert_eq!(program.functions[1].params[0].mode, ParamMode::Mut);
    }

    #[test]
    fn rejects_legacy_suffix_read_borrow_parameter_mode() {
        let error = parse(
            r#"
func read(name in string) {
    print(name)
}
"#,
        )
        .unwrap_err();

        assert!(error
            .message
            .contains("expected `)` after function parameters"));
    }

    #[test]
    fn rejects_legacy_suffix_mut_borrow_parameter_mode() {
        let error = parse(
            r#"
func rename(name mut string) {
    print(name)
}
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("expected type name"));
    }

    #[test]
    fn rejects_legacy_in_call_borrow_mode() {
        let error = parse(
            r#"
func read(con name string) {
    print(name)
}

func main() {
    name := "kim"
    read(in name)
}
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("expected `)` after call arguments"));
    }

    #[test]
    fn rejects_con_borrow_marker_in_let_value_position() {
        let error = parse(
            r#"
type User struct {
    name string
}

func main() {
    user := User{name: "kim"}
    borrowed := con user.name
    print(borrowed)
}
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("expected expression"));
    }

    #[test]
    fn rejects_mut_borrow_marker_in_return_value_position() {
        let error = parse(
            r#"
func identity(mut name string) string {
    return mut name
}
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("expected expression"));
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
    fn parses_else_if_expression_as_nested_if() {
        let program = parse(
            r#"
func main() {
    label := if false { "no" } else if true { "yes" } else { "fallback" }
}
"#,
        )
        .unwrap();

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::If { else_branch, .. } = &expr.kind else {
            panic!("expected outer if expression");
        };
        let ExprKind::If {
            then_branch,
            else_branch,
            ..
        } = &else_branch.kind
        else {
            panic!("expected nested if expression");
        };
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
    fn parses_else_if_statement_as_nested_if() {
        let program = parse(
            r#"
func main() {
    if false {
        print("no")
    } else if true {
        print("yes")
    } else {
        print("fallback")
    }
}
"#,
        )
        .unwrap();

        let StmtKind::If { else_block, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected outer if statement");
        };
        let nested_block = else_block.as_ref().expect("expected synthetic else block");
        assert_eq!(nested_block.statements.len(), 1);
        let StmtKind::If {
            then_block,
            else_block,
            ..
        } = &nested_block.statements[0].kind
        else {
            panic!("expected nested if statement");
        };
        assert_eq!(then_block.statements.len(), 1);
        assert_eq!(else_block.as_ref().unwrap().statements.len(), 1);
    }

    #[test]
    fn parses_match_statement_with_block_arms() {
        let program = parse(
            r#"
func main() {
    value := Some(1)
    match value {
        case Some(inner) {
            print(inner)
        }
        case None {
            print(0)
        }
    }
}
"#,
        )
        .unwrap();

        let StmtKind::Match { arms, .. } = &program.functions[0].body.statements[1].kind else {
            panic!("expected match statement");
        };
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].block.statements.len(), 1);
        assert!(matches!(arms[0].pattern, MatchPattern::Some(_)));
        assert!(matches!(arms[1].pattern, MatchPattern::None));
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

func (con self User) age() int {
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
        assert_eq!(receiver.mode, ParamMode::Con);
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
    fn parses_field_assignment() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    mut user := User{age: 30}
    user.age = 31
}
"#,
        )
        .unwrap();

        let StmtKind::FieldAssign { base, field, expr } =
            &program.functions[0].body.statements[1].kind
        else {
            panic!("expected field assignment");
        };
        assert!(matches!(base.kind, ExprKind::Var(_)));
        assert_eq!(field, "age");
        assert!(matches!(expr.kind, ExprKind::Int(31)));
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
    fn parses_function_type_refs_with_modes_and_mutability() {
        let program = parse(
            r#"
type Handler struct {
    apply func(int, con string) bool
}

func Wrap(transform func(int) int) func mut(mut Counter) int {
    return transform
}
"#,
        )
        .unwrap();

        let function = program.structs[0].fields[0]
            .ty
            .function
            .as_ref()
            .expect("expected function type");
        assert!(!function.mutable);
        assert_eq!(function.params.len(), 2);
        assert_eq!(function.params[0].mode, ParamMode::Owned);
        assert_eq!(function.params[0].ty.name, "int");
        assert_eq!(function.params[1].mode, ParamMode::Con);
        assert_eq!(function.params[1].ty.name, "string");
        assert_eq!(function.return_type.name, "bool");

        let return_type = program.functions[0]
            .return_type
            .as_ref()
            .and_then(|ty| ty.function.as_ref())
            .expect("expected mutable function return type");
        assert!(return_type.mutable);
        assert_eq!(return_type.params[0].mode, ParamMode::Mut);
        assert_eq!(return_type.params[0].ty.name, "Counter");
    }

    #[test]
    fn requires_explicit_function_type_return_type() {
        let error = parse("func Use(callback func(int)) {}\n").unwrap_err();

        assert!(error
            .message
            .contains("expected function return type; use `unit` for no value"));
    }

    #[test]
    fn parses_plain_and_mutable_function_literals() {
        let program = parse(
            r#"
func main() {
    plain := func(value int) int {
        return value * 2
    }
    mut count := 0
    mut counter := func mut(delta int) int {
        count = count + delta
        return count
    }
}
"#,
        )
        .unwrap();

        let StmtKind::Let { expr: plain, .. } = &program.functions[0].body.statements[0].kind
        else {
            panic!("expected plain closure binding");
        };
        let ExprKind::FunctionLiteral(function) = &plain.kind else {
            panic!("expected plain function literal");
        };
        assert!(!function.mutable);
        assert_eq!(function.params[0].name, "value");
        assert_eq!(function.params[0].ty.name, "int");
        assert_eq!(
            function.return_type.as_ref().map(|ty| ty.name.as_str()),
            Some("int")
        );

        let StmtKind::Let { expr: counter, .. } = &program.functions[0].body.statements[2].kind
        else {
            panic!("expected mutable closure binding");
        };
        let ExprKind::FunctionLiteral(function) = &counter.kind else {
            panic!("expected mutable function literal");
        };
        assert!(function.mutable);
        assert!(matches!(
            function.body.statements[0].kind,
            StmtKind::Assign { .. }
        ));
    }

    #[test]
    fn parses_fixed_size_array_type_refs() {
        let program = parse(
            r#"
type Bag struct {
    values [3]int
}

func make() [3]int {
    return values
}

func wrap(values Option[[2]string]) {
}
"#,
        )
        .unwrap();

        let field_ty = &program.structs[0].fields[0].ty;
        assert_eq!(field_ty.name, "Array");
        assert_eq!(field_ty.array_len, Some(3));
        assert_eq!(field_ty.args.len(), 1);
        assert_eq!(field_ty.args[0].name, "int");

        let return_ty = program.functions[0].return_type.as_ref().unwrap();
        assert_eq!(return_ty.name, "Array");
        assert_eq!(return_ty.array_len, Some(3));
        assert_eq!(return_ty.args[0].name, "int");

        let option_ty = &program.functions[1].params[0].ty;
        assert_eq!(option_ty.name, "Option");
        let array_ty = &option_ty.args[0];
        assert_eq!(array_ty.name, "Array");
        assert_eq!(array_ty.array_len, Some(2));
        assert_eq!(array_ty.args[0].name, "string");
    }

    #[test]
    fn parses_slice_type_refs() {
        let program = parse(
            r#"
type Bag struct {
    values []int
    nested []Option[[2]string]
}

func take(values []string) []int {
    return values
}
"#,
        )
        .unwrap();

        let values_ty = &program.structs[0].fields[0].ty;
        assert_eq!(values_ty.name, "Slice");
        assert!(values_ty.slice);
        assert_eq!(values_ty.array_len, None);
        assert_eq!(values_ty.args.len(), 1);
        assert_eq!(values_ty.args[0].name, "int");

        let nested_ty = &program.structs[0].fields[1].ty;
        assert_eq!(nested_ty.name, "Slice");
        assert!(nested_ty.slice);
        let option_ty = &nested_ty.args[0];
        assert_eq!(option_ty.name, "Option");
        let array_ty = &option_ty.args[0];
        assert_eq!(array_ty.name, "Array");
        assert_eq!(array_ty.array_len, Some(2));
        assert!(!array_ty.slice);
        assert_eq!(array_ty.args[0].name, "string");

        let param_ty = &program.functions[0].params[0].ty;
        assert_eq!(param_ty.name, "Slice");
        assert!(param_ty.slice);
        assert_eq!(param_ty.args[0].name, "string");

        let return_ty = program.functions[0].return_type.as_ref().unwrap();
        assert_eq!(return_ty.name, "Slice");
        assert!(return_ty.slice);
        assert_eq!(return_ty.args[0].name, "int");
    }

    #[test]
    fn parses_fixed_size_array_literals() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    empty := [0]string{}
    slice := []int{1, 2}
}
"#,
        )
        .unwrap();

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
            panic!("expected let statement");
        };
        let ExprKind::ArrayLiteral { ty, elements } = &expr.kind else {
            panic!("expected array literal");
        };
        assert_eq!(ty.name, "Array");
        assert_eq!(ty.array_len, Some(3));
        assert_eq!(ty.args[0].name, "int");
        assert_eq!(elements.len(), 3);
        assert!(matches!(elements[0].kind, ExprKind::Int(1)));

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[1].kind else {
            panic!("expected let statement");
        };
        let ExprKind::ArrayLiteral { ty, elements } = &expr.kind else {
            panic!("expected array literal");
        };
        assert_eq!(ty.array_len, Some(0));
        assert_eq!(ty.args[0].name, "string");
        assert!(elements.is_empty());

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[2].kind else {
            panic!("expected let statement");
        };
        let ExprKind::ArrayLiteral { ty, elements } = &expr.kind else {
            panic!("expected slice literal");
        };
        assert_eq!(ty.name, "Slice");
        assert!(ty.slice);
        assert_eq!(ty.array_len, None);
        assert_eq!(ty.args[0].name, "int");
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn parses_array_range_loop() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    for i, value := range values {
        print(i)
        print(value)
    }
}
"#,
        )
        .unwrap();

        let StmtKind::RangeFor {
            index_name,
            value_name,
            source,
            body,
        } = &program.functions[0].body.statements[1].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "i");
        assert_eq!(value_name, "value");
        assert!(matches!(source.kind, ExprKind::Var(_)));
        assert_eq!(body.statements.len(), 2);
    }

    #[test]
    fn parses_array_range_loop_blank_identifiers() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    for _, value := range values {
        print(value)
    }
    for i, _ := range values {
        print(i)
    }
}
"#,
        )
        .unwrap();

        let StmtKind::RangeFor {
            index_name,
            value_name,
            ..
        } = &program.functions[0].body.statements[1].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "_");
        assert_eq!(value_name, "value");

        let StmtKind::RangeFor {
            index_name,
            value_name,
            ..
        } = &program.functions[0].body.statements[2].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "i");
        assert_eq!(value_name, "_");
    }

    #[test]
    fn parses_one_variable_array_range_loop() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    for i := range values {
        print(i)
    }
    for _ := range values {
        print(1)
    }
}
"#,
        )
        .unwrap();

        let StmtKind::RangeFor {
            index_name,
            value_name,
            ..
        } = &program.functions[0].body.statements[1].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "i");
        assert_eq!(value_name, "_");

        let StmtKind::RangeFor {
            index_name,
            value_name,
            ..
        } = &program.functions[0].body.statements[2].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "_");
        assert_eq!(value_name, "_");
    }

    #[test]
    fn rejects_mutable_range_value_binding_syntax() {
        let error = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    for i, mut value := range values {
        print(value)
    }
}
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("expected range value binding name"));
    }

    #[test]
    fn rejects_by_reference_range_value_binding_syntax() {
        let error = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    for i, con value := range values {
        print(value)
    }
}
"#,
        )
        .unwrap_err();

        assert!(error.message.contains("expected range value binding name"));
    }

    #[test]
    fn parses_array_index_and_len_call() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    first := values[0]
    count := len(values)
}
"#,
        )
        .unwrap();

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[1].kind else {
            panic!("expected let statement");
        };
        let ExprKind::Index { base, index } = &expr.kind else {
            panic!("expected index expression");
        };
        assert!(matches!(base.kind, ExprKind::Var(_)));
        assert!(matches!(index.kind, ExprKind::Int(0)));

        let StmtKind::Let { expr, .. } = &program.functions[0].body.statements[2].kind else {
            panic!("expected let statement");
        };
        let ExprKind::Call { callee, args } = &expr.kind else {
            panic!("expected len call");
        };
        assert!(matches!(&callee.kind, ExprKind::Var(name) if name == "len"));
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn parses_array_index_assignment() {
        let program = parse(
            r#"
func main() {
    mut values := [3]int{1, 2, 3}
    index := 1
    values[index] = 5
}
"#,
        )
        .unwrap();

        let StmtKind::IndexAssign { base, index, expr } =
            &program.functions[0].body.statements[2].kind
        else {
            panic!("expected index assignment");
        };
        assert!(matches!(base.kind, ExprKind::Var(_)));
        assert!(matches!(index.kind, ExprKind::Var(_)));
        assert!(matches!(expr.kind, ExprKind::Int(5)));
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

    #[test]
    fn preserves_source_id_on_programs_and_errors() {
        let source_id = SourceId::new(11);
        let program = parse_with_source("func main() {}", source_id).unwrap();
        assert_eq!(program.span.source, source_id);
        assert_eq!(program.functions[0].span.source, source_id);

        let error = parse_with_source("func main( {}", source_id).unwrap_err();
        assert_eq!(error.span.source, source_id);
    }
}
