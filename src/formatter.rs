use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    ast::{
        Arg, Block, EnumDecl, Expr, ExprKind, FieldDecl, ForInit, ForPost, Function,
        FunctionLiteral, MatchArm, MatchBlockArm, Param, Program, Stmt, StmtKind, StructDecl,
        TestDecl, TypeRef,
    },
    lex, parse,
    token::{Keyword, Span, Token, TokenKind},
};

const INDENT_WIDTH: usize = 4;

pub fn format_source(source: &str) -> Result<String, FormatError> {
    let program = parse(source).map_err(|error| FormatError {
        message: error.message,
        span: error.span,
    })?;
    let tokens = lex(source).map_err(|error| FormatError {
        message: error.message,
        span: error.span,
    })?;
    let hints = LayoutHints::from_program(&program);

    Ok(Formatter::new(source, &tokens, hints).format())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    pub message: String,
    pub span: Span,
}

impl fmt::Display for FormatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for FormatError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LineBreak {
    None,
    Line,
    BlankLine,
}

impl LineBreak {
    fn count(self) -> usize {
        match self {
            Self::None => 0,
            Self::Line => 1,
            Self::BlankLine => 2,
        }
    }
}

#[derive(Debug, Default)]
struct LayoutHints {
    breaks: HashMap<usize, LineBreak>,
    type_starts: HashSet<usize>,
    receiver_starts: HashSet<usize>,
    receiver_opens: HashSet<usize>,
    contextual_block_opens: HashSet<usize>,
}

impl LayoutHints {
    fn from_program(program: &Program) -> Self {
        let mut hints = Self::default();

        for unit in &program.source_units {
            if let Some(package) = &unit.package {
                hints.break_after(package.span.end, LineBreak::BlankLine);
            }
            for (index, import) in unit.imports.iter().enumerate() {
                let line_break = if index + 1 == unit.imports.len() {
                    LineBreak::BlankLine
                } else {
                    LineBreak::Line
                };
                hints.break_after(import.span.end, line_break);
            }
        }

        for declaration in &program.structs {
            hints.visit_struct(declaration);
            hints.break_after(declaration.span.end, LineBreak::BlankLine);
        }
        for declaration in &program.enums {
            hints.visit_enum(declaration);
            hints.break_after(declaration.span.end, LineBreak::BlankLine);
        }
        for function in &program.functions {
            hints.visit_function(function);
            hints.break_after(function.span.end, LineBreak::BlankLine);
        }
        for test in &program.tests {
            hints.visit_test(test);
            hints.break_after(test.span.end, LineBreak::BlankLine);
        }

        hints
    }

    fn break_after(&mut self, offset: usize, line_break: LineBreak) {
        self.breaks
            .entry(offset)
            .and_modify(|current| *current = (*current).max(line_break))
            .or_insert(line_break);
    }

    fn visit_struct(&mut self, declaration: &StructDecl) {
        for field in &declaration.fields {
            self.visit_field(field);
            self.break_after(field.span.end, LineBreak::Line);
        }
    }

    fn visit_enum(&mut self, declaration: &EnumDecl) {
        for variant in &declaration.variants {
            for payload in &variant.payloads {
                self.visit_type(payload);
            }
            self.break_after(variant.span.end, LineBreak::Line);
        }
    }

    fn visit_field(&mut self, field: &FieldDecl) {
        self.visit_type(&field.ty);
    }

    fn visit_function(&mut self, function: &Function) {
        if let Some(receiver) = &function.receiver {
            self.receiver_starts.insert(receiver.span.start);
            self.visit_param(receiver);
        }
        for param in &function.params {
            self.visit_param(param);
        }
        if let Some(return_type) = &function.return_type {
            self.visit_type(return_type);
        }
        self.visit_block(&function.body);
    }

    fn visit_test(&mut self, test: &TestDecl) {
        self.contextual_block_opens.insert(test.body.span.start);
        self.visit_block(&test.body);
    }

    fn visit_param(&mut self, param: &Param) {
        self.visit_type(&param.ty);
    }

    fn visit_type(&mut self, ty: &TypeRef) {
        self.type_starts.insert(ty.span.start);
        for argument in &ty.args {
            self.visit_type(argument);
        }
        if let Some(function) = &ty.function {
            for param in &function.params {
                self.visit_type(&param.ty);
            }
            self.visit_type(&function.return_type);
        }
    }

    fn visit_block(&mut self, block: &Block) {
        for statement in &block.statements {
            self.visit_statement(statement);
            self.break_after(statement.span.end, LineBreak::Line);
        }
    }

    fn visit_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let { expr, .. }
            | StmtKind::Assign { expr, .. }
            | StmtKind::Return { expr }
            | StmtKind::Expr { expr } => self.visit_expr(expr),
            StmtKind::FieldAssign { base, expr, .. } => {
                self.visit_expr(base);
                self.visit_expr(expr);
            }
            StmtKind::IndexAssign {
                base, index, expr, ..
            } => {
                self.visit_expr(base);
                self.visit_expr(index);
                self.visit_expr(expr);
            }
            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.visit_expr(condition);
                self.visit_block(then_block);
                if let Some(else_block) = else_block {
                    self.visit_block(else_block);
                }
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
            } => {
                if let Some(ForInit::Let { expr, .. }) = init {
                    self.visit_expr(expr);
                }
                if let Some(condition) = condition {
                    self.visit_expr(condition);
                }
                if let Some(ForPost::Assign { target, expr }) = post {
                    self.visit_expr(target);
                    self.visit_expr(expr);
                }
                self.visit_block(body);
            }
            StmtKind::RangeFor { source, body, .. } => {
                self.visit_expr(source);
                self.visit_block(body);
            }
            StmtKind::Match { scrutinee, arms } => {
                self.visit_expr(scrutinee);
                for arm in arms {
                    self.visit_block_arm(arm);
                }
            }
            StmtKind::Assert { condition } => self.visit_expr(condition),
            StmtKind::Break | StmtKind::Continue => {}
        }
    }

    fn visit_block_arm(&mut self, arm: &MatchBlockArm) {
        self.visit_block(&arm.block);
        self.break_after(arm.span.end, LineBreak::Line);
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::FunctionLiteral(function) => self.visit_function_literal(function),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.visit_expr(condition);
                self.visit_expr(then_branch);
                self.visit_expr(else_branch);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.visit_expr(scrutinee);
                for arm in arms {
                    self.visit_arm(arm);
                }
            }
            ExprKind::StructLiteral {
                type_args, fields, ..
            } => {
                for argument in type_args {
                    self.visit_type(argument);
                }
                for field in fields {
                    self.visit_expr(&field.expr);
                }
            }
            ExprKind::ArrayLiteral { ty, elements } => {
                self.visit_type(ty);
                for element in elements {
                    self.visit_expr(element);
                }
            }
            ExprKind::FieldAccess { base, .. } => self.visit_expr(base),
            ExprKind::Index { base, index } => {
                self.visit_expr(base);
                self.visit_expr(index);
            }
            ExprKind::TypeApply { base, args } => {
                self.visit_expr(base);
                for argument in args {
                    self.visit_type(argument);
                }
            }
            ExprKind::EnumConstructor { args, .. } => {
                if let Some(args) = args {
                    for argument in args {
                        self.visit_arg(argument);
                    }
                }
            }
            ExprKind::Call { callee, args } => {
                self.visit_expr(callee);
                for argument in args {
                    self.visit_arg(argument);
                }
            }
            ExprKind::Unary { expr, .. } => self.visit_expr(expr),
            ExprKind::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            ExprKind::Int(_)
            | ExprKind::String(_)
            | ExprKind::Bool(_)
            | ExprKind::Nil
            | ExprKind::Var(_) => {}
        }
    }

    fn visit_function_literal(&mut self, function: &FunctionLiteral) {
        for param in &function.params {
            self.visit_param(param);
        }
        if let Some(return_type) = &function.return_type {
            self.visit_type(return_type);
        }
        self.visit_block(&function.body);
    }

    fn visit_arm(&mut self, arm: &MatchArm) {
        self.visit_expr(&arm.expr);
        self.break_after(arm.span.end, LineBreak::Line);
    }

    fn visit_arg(&mut self, argument: &Arg) {
        self.visit_expr(&argument.expr);
    }
}

struct Formatter<'a> {
    source: &'a str,
    tokens: &'a [Token],
    hints: LayoutHints,
    output: Output,
    brace_stack: Vec<BraceFrame>,
    pending_block: bool,
    previous_unary: bool,
    previous_right_brace_was_block: bool,
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str, tokens: &'a [Token], mut hints: LayoutHints) -> Self {
        for pair in tokens.windows(2) {
            if pair[1].kind == TokenKind::Semicolon {
                if let Some(line_break) = hints.breaks.remove(&pair[0].span.end) {
                    hints.break_after(pair[1].span.end, line_break);
                }
            }
        }
        for receiver_start in hints.receiver_starts.iter().copied() {
            if let Some(open) = tokens.iter().rfind(|token| {
                token.span.start < receiver_start && token.kind == TokenKind::LeftParen
            }) {
                hints.receiver_opens.insert(open.span.start);
            }
        }

        Self {
            source,
            tokens,
            hints,
            output: Output::default(),
            brace_stack: Vec::new(),
            pending_block: false,
            previous_unary: false,
            previous_right_brace_was_block: false,
        }
    }

    fn format(mut self) -> String {
        let syntax_tokens: Vec<&Token> = self
            .tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Eof)
            .collect();
        let mut previous: Option<&Token> = None;
        let mut cursor = 0;

        for token in syntax_tokens {
            let gap = Gap::parse(&self.source[cursor..token.span.start], previous.is_some());
            if !gap.comments.is_empty() {
                if let Some(frame) = self.brace_stack.last_mut() {
                    frame.multiline = true;
                }
            }
            let current_is_block_close = token.kind == TokenKind::RightBrace
                && self.brace_stack.last().is_some_and(|frame| frame.block);
            let current_is_multiline_literal_close = token.kind == TokenKind::RightBrace
                && self
                    .brace_stack
                    .last()
                    .is_some_and(|frame| !frame.block && frame.multiline);
            let current_is_block_open = token.kind == TokenKind::LeftBrace
                && (self.pending_block
                    || self
                        .hints
                        .contextual_block_opens
                        .contains(&token.span.start));
            let empty_block = current_is_block_close
                && previous.is_some_and(|previous| previous.kind == TokenKind::LeftBrace)
                && gap.comments.is_empty();

            if token.kind == TokenKind::RightBrace {
                self.output.decrease_indent();
            }

            let mut line_break = previous
                .and_then(|previous| self.hints.breaks.get(&previous.span.end).copied())
                .unwrap_or(LineBreak::None);
            if previous.is_some_and(|previous| {
                previous.kind == TokenKind::LeftBrace
                    && self.brace_stack.last().is_some_and(|frame| frame.block)
            }) {
                line_break = line_break.max(LineBreak::Line);
            }
            if previous.is_some_and(|previous| {
                previous.kind == TokenKind::RightBrace && self.previous_right_brace_was_block
            }) {
                line_break = line_break.max(LineBreak::Line);
            }
            if current_is_block_close || current_is_multiline_literal_close {
                line_break = line_break.max(LineBreak::Line);
            }
            if token.kind == TokenKind::Semicolon
                || empty_block
                || (token.kind == TokenKind::Keyword(Keyword::Else)
                    && previous.is_some_and(|previous| previous.kind == TokenKind::RightBrace))
            {
                line_break = LineBreak::None;
            }

            self.output.write_gap(&gap, line_break);

            let unary = is_unary(&token.kind, previous.map(|token| &token.kind));
            if previous.is_some_and(|previous| {
                needs_space(
                    &previous.kind,
                    &token.kind,
                    self.previous_unary,
                    current_is_block_open,
                    self.hints.type_starts.contains(&token.span.start),
                    self.hints.receiver_opens.contains(&token.span.start),
                )
            }) {
                self.output.space();
            }
            self.output
                .write(&self.source[token.span.start..token.span.end]);

            match token.kind {
                TokenKind::LeftBrace => {
                    self.brace_stack.push(BraceFrame {
                        block: current_is_block_open,
                        multiline: current_is_block_open,
                    });
                    self.output.increase_indent();
                    if current_is_block_open {
                        self.pending_block = false;
                    }
                }
                TokenKind::RightBrace => {
                    self.brace_stack.pop();
                }
                TokenKind::Keyword(
                    Keyword::Func
                    | Keyword::If
                    | Keyword::Else
                    | Keyword::For
                    | Keyword::Match
                    | Keyword::Case
                    | Keyword::Struct
                    | Keyword::Enum,
                ) => self.pending_block = true,
                _ => {}
            }

            self.previous_unary = unary;
            self.previous_right_brace_was_block = current_is_block_close;
            cursor = token.span.end;
            previous = Some(token);
        }

        let trailing_gap = Gap::parse(&self.source[cursor..], previous.is_some());
        let trailing_break = previous
            .and_then(|previous| self.hints.breaks.get(&previous.span.end).copied())
            .unwrap_or(LineBreak::None);
        self.output.write_gap(&trailing_gap, trailing_break);
        self.output.finish()
    }
}

#[derive(Debug, Clone, Copy)]
struct BraceFrame {
    block: bool,
    multiline: bool,
}

fn is_unary(current: &TokenKind, previous: Option<&TokenKind>) -> bool {
    if *current == TokenKind::Bang {
        return true;
    }
    if *current != TokenKind::Minus {
        return false;
    }

    previous.is_none_or(|previous| {
        matches!(
            previous,
            TokenKind::LeftParen
                | TokenKind::LeftBracket
                | TokenKind::LeftBrace
                | TokenKind::Comma
                | TokenKind::Colon
                | TokenKind::Semicolon
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::Equal
                | TokenKind::EqualEqual
                | TokenKind::Bang
                | TokenKind::BangEqual
                | TokenKind::AmpAmp
                | TokenKind::PipePipe
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
                | TokenKind::ColonEqual
                | TokenKind::Arrow
                | TokenKind::PipeGreater
        )
    })
}

fn needs_space(
    previous: &TokenKind,
    current: &TokenKind,
    previous_unary: bool,
    current_is_block_open: bool,
    current_starts_type: bool,
    current_opens_receiver: bool,
) -> bool {
    if matches!(
        current,
        TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Comma
            | TokenKind::Dot
            | TokenKind::Semicolon
            | TokenKind::Colon
    ) {
        return false;
    }
    if matches!(
        previous,
        TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::Dot
    ) || previous_unary
    {
        return false;
    }
    if *current == TokenKind::LeftParen {
        return *previous == TokenKind::Keyword(Keyword::Func) && current_opens_receiver;
    }
    if *current == TokenKind::LeftBracket {
        if is_operator(previous)
            || matches!(
                previous,
                TokenKind::Comma | TokenKind::Colon | TokenKind::Semicolon
            )
            || matches!(
                previous,
                TokenKind::Keyword(Keyword::Return | Keyword::Range | Keyword::Con | Keyword::Mut)
            )
        {
            return true;
        }
        return current_starts_type
            && matches!(previous, TokenKind::Ident(_) | TokenKind::RightParen);
    }
    if *current == TokenKind::LeftBrace {
        return current_is_block_open;
    }
    if *previous == TokenKind::LeftBrace || *previous == TokenKind::RightBracket {
        return false;
    }
    if matches!(
        previous,
        TokenKind::Comma | TokenKind::Colon | TokenKind::Semicolon
    ) {
        return true;
    }
    if is_operator(previous) || is_operator(current) {
        return true;
    }

    is_word(previous)
        || matches!(previous, TokenKind::RightParen | TokenKind::RightBrace)
        || is_word(current)
}

fn is_operator(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Equal
            | TokenKind::EqualEqual
            | TokenKind::Bang
            | TokenKind::BangEqual
            | TokenKind::AmpAmp
            | TokenKind::PipePipe
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::ColonEqual
            | TokenKind::Arrow
            | TokenKind::PipeGreater
    )
}

fn is_word(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident(_) | TokenKind::Int(_) | TokenKind::String(_) | TokenKind::Keyword(_)
    )
}

#[derive(Debug)]
struct Gap<'a> {
    comments: Vec<Comment<'a>>,
    trailing_newlines: usize,
    total_newlines: usize,
}

impl<'a> Gap<'a> {
    fn parse(source: &'a str, has_previous: bool) -> Self {
        let mut comments = Vec::new();
        let mut cursor = 0;
        let mut newlines = 0;
        let mut total_newlines = 0;

        while cursor < source.len() {
            if source.as_bytes()[cursor..].starts_with(b"//") {
                let start = cursor;
                cursor += 2;
                while cursor < source.len() && !matches!(source.as_bytes()[cursor], b'\n' | b'\r') {
                    cursor += 1;
                }
                comments.push(Comment {
                    text: &source[start..cursor],
                    trailing: has_previous && comments.is_empty() && newlines == 0,
                    blank_before: newlines >= 2,
                });
                newlines = 0;
                continue;
            }

            match source.as_bytes()[cursor] {
                b'\r' if source.as_bytes().get(cursor + 1) == Some(&b'\n') => {
                    cursor += 2;
                    newlines += 1;
                    total_newlines += 1;
                }
                b'\r' | b'\n' => {
                    cursor += 1;
                    newlines += 1;
                    total_newlines += 1;
                }
                _ => cursor += 1,
            }
        }

        Self {
            comments,
            trailing_newlines: newlines,
            total_newlines,
        }
    }
}

#[derive(Debug)]
struct Comment<'a> {
    text: &'a str,
    trailing: bool,
    blank_before: bool,
}

#[derive(Debug, Default)]
struct Output {
    text: String,
    indent: usize,
}

impl Output {
    fn write_gap(&mut self, gap: &Gap<'_>, line_break: LineBreak) {
        let mut requested = line_break.count();

        for comment in &gap.comments {
            if comment.trailing {
                self.space();
            } else {
                let comment_break = if comment.blank_before { 2 } else { 1 };
                self.newlines(requested.max(comment_break));
                requested = 0;
            }
            self.write(comment.text);
            self.newlines(1);
        }

        if gap.comments.is_empty() {
            if requested > 0 && gap.total_newlines >= 2 {
                requested = requested.max(2);
            }
        } else if gap.trailing_newlines >= 2 {
            requested = requested.max(2);
        } else if gap.trailing_newlines > 0 {
            requested = requested.max(1);
        }

        self.newlines(requested);
    }

    fn write(&mut self, text: &str) {
        if self.text.is_empty() || self.text.ends_with('\n') {
            self.text.push_str(&" ".repeat(self.indent * INDENT_WIDTH));
        }
        self.text.push_str(text);
    }

    fn space(&mut self) {
        if !self.text.is_empty() && !self.text.ends_with([' ', '\n']) {
            self.text.push(' ');
        }
    }

    fn newlines(&mut self, count: usize) {
        if count == 0 || self.text.is_empty() {
            return;
        }
        while self.text.ends_with(' ') {
            self.text.pop();
        }
        let existing = self.text.chars().rev().take_while(|ch| *ch == '\n').count();
        for _ in existing..count.min(2) {
            self.text.push('\n');
        }
    }

    fn increase_indent(&mut self) {
        self.indent += 1;
    }

    fn decrease_indent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    fn finish(mut self) -> String {
        while self.text.ends_with(char::is_whitespace) {
            self.text.pop();
        }
        self.text.push('\n');
        self.text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_declarations_statements_and_literals() {
        let source = r#"type User struct{name string
age int}
func main(){mut user:=User{name:"kim",age:30};if user.age>20{print(user.name)}else{print("minor")}}"#;
        let expected = r#"type User struct {
    name string
    age int
}

func main() {
    mut user := User{name: "kim", age: 30};
    if user.age > 20 {
        print(user.name)
    } else {
        print("minor")
    }
}
"#;

        assert_eq!(format_source(source).unwrap(), expected);
    }

    #[test]
    fn formats_contextual_test_declarations_and_assertions() {
        let source = "test AddsValues(){assert(20+22==42);if true{assert(true)}}";
        let expected = r#"test AddsValues() {
    assert(20 + 22 == 42);
    if true {
        assert(true)
    }
}
"#;

        let formatted = format_source(source).unwrap();
        assert_eq!(formatted, expected);
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn preserves_line_comments_and_normalizes_blank_lines() {
        let source = r#"// file


func main(){ // body
// before


name:="kim" // after
print(name)} // end
"#;
        let expected = r#"// file

func main() { // body
    // before

    name := "kim" // after
    print(name)
} // end
"#;

        assert_eq!(format_source(source).unwrap(), expected);
    }

    #[test]
    fn formats_control_expressions_and_function_literals() {
        let source = r#"func choose(flag bool)int{return if flag{match Some(1){case Some(value){value}case None{0}}}else{0}}
func main(){add:=func(value int)int{return value+1};print(add(choose(true)))}"#;
        let formatted = format_source(source).unwrap();

        assert!(formatted.contains("func choose(flag bool) int {"));
        assert!(
            formatted.contains("case Some(value) {\n                value\n            }"),
            "{formatted}"
        );
        assert!(formatted.contains("add := func(value int) int {"));
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn preserves_token_kinds_and_comment_text() {
        let source = r#"// first
func main(){values:=[]int{1,2,3} // values
print(values[0])}"#;
        let formatted = format_source(source).unwrap();
        let original_kinds: Vec<TokenKind> = lex(source)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect();
        let formatted_kinds: Vec<TokenKind> = lex(&formatted)
            .unwrap()
            .into_iter()
            .map(|token| token.kind)
            .collect();

        assert_eq!(formatted_kinds, original_kinds);
        assert_eq!(comments(&formatted), comments(source));
        assert!(formatted.contains("values := []int{1, 2, 3}"));
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn indents_comments_inside_multiline_literals() {
        let source = r#"type User struct{name string
age int}
func main(){user:=User{
// name
name:"kim", // age follows
age:30
};print(user.name)}"#;
        let formatted = format_source(source).unwrap();

        assert!(formatted.contains(
            "user := User{\n        // name\n        name: \"kim\", // age follows\n        age: 30\n    };"
        ));
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn keeps_declaration_spacing_after_trailing_comments() {
        let source = "func first(){} // first\nfunc second(){}\n";
        let expected = "func first() {} // first\n\nfunc second() {}\n";

        assert_eq!(format_source(source).unwrap(), expected);
    }

    #[test]
    fn rejects_invalid_source() {
        let error = format_source("func main( {").unwrap_err();

        assert!(!error.message.is_empty());
        assert!(error.span.end >= error.span.start);
    }

    #[test]
    fn formats_all_examples_idempotently() {
        let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
        let mut paths = Vec::new();
        collect_sources(&examples, &mut paths);
        paths.sort();
        assert!(!paths.is_empty());

        for path in paths {
            let source = std::fs::read_to_string(&path).unwrap();
            let formatted = format_source(&source)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let original_kinds: Vec<TokenKind> = lex(&source)
                .unwrap()
                .into_iter()
                .map(|token| token.kind)
                .collect();
            let formatted_kinds: Vec<TokenKind> = lex(&formatted)
                .unwrap()
                .into_iter()
                .map(|token| token.kind)
                .collect();

            assert_eq!(formatted_kinds, original_kinds, "{}", path.display());
            assert_eq!(
                comments(&formatted),
                comments(&source),
                "{}",
                path.display()
            );
            assert_eq!(
                format_source(&formatted).unwrap(),
                formatted,
                "{}",
                path.display()
            );
        }
    }

    fn comments(source: &str) -> Vec<String> {
        let tokens = lex(source).unwrap();
        let mut comments = Vec::new();
        let mut cursor = 0;
        for token in tokens {
            let gap = Gap::parse(&source[cursor..token.span.start], cursor > 0);
            comments.extend(
                gap.comments
                    .into_iter()
                    .map(|comment| comment.text.to_string()),
            );
            cursor = token.span.end;
        }
        comments
    }

    fn collect_sources(directory: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_sources(&path, paths);
            } else if path.extension().is_some_and(|extension| extension == "mlg") {
                paths.push(path);
            }
        }
    }
}
