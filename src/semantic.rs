use std::{collections::HashMap, fmt};

use crate::{
    ast::{
        Arg, ArgMode, BinaryOp, Expr, ExprKind, Function, ParamMode, Program, Stmt, StmtKind,
        TypeRef, UnaryOp,
    },
    token::Span,
};

pub fn check(program: &Program) -> Result<CheckedProgram<'_>, SemanticError> {
    Checker::new(program).check()
}

#[derive(Debug, Clone)]
pub struct CheckedProgram<'a> {
    pub program: &'a Program,
    pub signatures: HashMap<&'a str, FunctionSig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSig {
    pub return_type: Type,
    pub params: Vec<ParamSig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSig {
    pub name: String,
    pub mode: ParamMode,
    pub ty: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    String,
    Unit,
}

impl Type {
    pub fn source_name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Bool => "bool",
            Self::String => "string",
            Self::Unit => "unit",
        }
    }

    pub fn is_copy(self) -> bool {
        matches!(self, Self::Int | Self::Bool | Self::Unit)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub message: String,
    pub span: Span,
}

impl SemanticError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SemanticError {}

struct Checker<'a> {
    program: &'a Program,
    signatures: HashMap<&'a str, FunctionSig>,
}

impl<'a> Checker<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            program,
            signatures: HashMap::new(),
        }
    }

    fn check(mut self) -> Result<CheckedProgram<'a>, SemanticError> {
        self.collect_signatures()?;
        for function in &self.program.functions {
            self.check_function(function)?;
        }

        Ok(CheckedProgram {
            program: self.program,
            signatures: self.signatures,
        })
    }

    fn collect_signatures(&mut self) -> Result<(), SemanticError> {
        for function in &self.program.functions {
            if self.signatures.contains_key(function.name.as_str()) {
                return Err(SemanticError::new(
                    format!("duplicate function `{}`", function.name),
                    function.span,
                ));
            }

            if function.name == "main" {
                if !function.params.is_empty() {
                    return Err(SemanticError::new(
                        "`main` must not take parameters",
                        function.params[0].span,
                    ));
                }
                if let Some(return_type) = &function.return_type {
                    return Err(SemanticError::new(
                        "`main` must not declare a return type in v0",
                        return_type.span,
                    ));
                }
            }

            let return_type = type_from_optional_ref(function.return_type.as_ref())?;
            let mut params = Vec::new();
            for param in &function.params {
                params.push(ParamSig {
                    name: param.name.clone(),
                    mode: param.mode,
                    ty: type_from_ref(&param.ty)?,
                });
            }

            self.signatures.insert(
                function.name.as_str(),
                FunctionSig {
                    return_type,
                    params,
                },
            );
        }

        if !self.signatures.contains_key("main") {
            return Err(SemanticError::new(
                "program must declare `func main()`",
                self.program.span,
            ));
        }

        Ok(())
    }

    fn check_function(&self, function: &Function) -> Result<(), SemanticError> {
        let sig = self.function_sig(&function.name, function.span)?;
        let mut locals = HashMap::new();
        for param in &sig.params {
            if locals
                .insert(
                    param.name.clone(),
                    Local {
                        ty: param.ty,
                        mutable: matches!(param.mode, ParamMode::Mut),
                        moved: false,
                    },
                )
                .is_some()
            {
                return Err(SemanticError::new(
                    format!("duplicate parameter `{}`", param.name),
                    function.span,
                ));
            }
        }

        let mut returned = false;
        for stmt in &function.body.statements {
            if matches!(stmt.kind, StmtKind::Return { .. }) {
                returned = true;
            }
            self.check_stmt(stmt, &mut locals, sig.return_type)?;
        }

        if sig.return_type != Type::Unit && !returned {
            return Err(SemanticError::new(
                format!(
                    "function `{}` must return `{}`",
                    function.name,
                    sig.return_type.source_name()
                ),
                function.span,
            ));
        }

        Ok(())
    }

    fn check_stmt(
        &self,
        stmt: &Stmt,
        locals: &mut HashMap<String, Local>,
        return_type: Type,
    ) -> Result<Type, SemanticError> {
        match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                expr,
            } => {
                let ty = self.check_expr(expr, locals, ValueUse::Owned)?;
                if locals
                    .insert(
                        name.clone(),
                        Local {
                            ty,
                            mutable: *mutable,
                            moved: false,
                        },
                    )
                    .is_some()
                {
                    return Err(SemanticError::new(
                        format!("binding `{name}` already exists in this block"),
                        stmt.span,
                    ));
                }
                Ok(Type::Unit)
            }
            StmtKind::Assign { name, expr } => {
                let value_ty = self.check_expr(expr, locals, ValueUse::Owned)?;
                let Some(local) = locals.get(name) else {
                    return Err(SemanticError::new(
                        format!("unknown variable `{name}`"),
                        stmt.span,
                    ));
                };
                if !local.mutable {
                    return Err(SemanticError::new(
                        format!("cannot assign to immutable binding `{name}`"),
                        stmt.span,
                    ));
                }
                if value_ty != local.ty {
                    return Err(SemanticError::new(
                        format!(
                            "assignment type mismatch for `{name}`: expected `{}`, got `{}`",
                            local.ty.source_name(),
                            value_ty.source_name()
                        ),
                        stmt.span,
                    ));
                }
                Ok(Type::Unit)
            }
            StmtKind::Return { expr } => {
                let value_ty = self.check_expr(expr, locals, ValueUse::Owned)?;
                if value_ty != return_type {
                    return Err(SemanticError::new(
                        format!(
                            "return type mismatch: expected `{}`, got `{}`",
                            return_type.source_name(),
                            value_ty.source_name()
                        ),
                        stmt.span,
                    ));
                }
                Ok(Type::Unit)
            }
            StmtKind::Expr { expr } => self.check_expr(expr, locals, ValueUse::Owned),
        }
    }

    fn check_expr(
        &self,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
    ) -> Result<Type, SemanticError> {
        match &expr.kind {
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::String(_) => Ok(Type::String),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::Nil => Err(SemanticError::new(
                "`nil` is reserved; use Option[T] when optional values are implemented",
                expr.span,
            )),
            ExprKind::Var(name) => self.check_var(name, locals, value_use, expr.span),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.check_if_expr(
                condition,
                then_branch,
                else_branch,
                locals,
                value_use,
                expr.span,
            ),
            ExprKind::Call { callee, args } => self.check_call(callee, args, locals, expr.span),
            ExprKind::Unary { op, expr } => {
                let ty = self.check_expr(expr, locals, ValueUse::Owned)?;
                match (op, ty) {
                    (UnaryOp::Negate, Type::Int) => Ok(Type::Int),
                    (UnaryOp::Not, Type::Bool) => Ok(Type::Bool),
                    (UnaryOp::Negate, _) => Err(SemanticError::new(
                        "`-` expects an `int` operand",
                        expr.span,
                    )),
                    (UnaryOp::Not, _) => Err(SemanticError::new(
                        "`!` expects a `bool` operand",
                        expr.span,
                    )),
                }
            }
            ExprKind::Binary { op, left, right } => self.check_binary(*op, left, right, locals),
        }
    }

    fn check_if_expr(
        &self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let condition_ty = self.check_expr(condition, locals, ValueUse::Owned)?;
        if condition_ty != Type::Bool {
            return Err(SemanticError::new(
                "if condition must have type `bool`",
                condition.span,
            ));
        }

        let mut then_locals = locals.clone();
        let then_ty = self.check_expr(then_branch, &mut then_locals, value_use)?;
        let mut else_locals = locals.clone();
        let else_ty = self.check_expr(else_branch, &mut else_locals, value_use)?;

        if then_ty != else_ty {
            return Err(SemanticError::new(
                format!(
                    "if branches must have the same type: got `{}` and `{}`",
                    then_ty.source_name(),
                    else_ty.source_name()
                ),
                span,
            ));
        }
        if then_ty == Type::Unit {
            return Err(SemanticError::new(
                "if expression branches must produce a value in v0",
                span,
            ));
        }

        merge_branch_moves(locals, &then_locals, &else_locals);
        Ok(then_ty)
    }

    fn check_var(
        &self,
        name: &str,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let Some(local) = locals.get_mut(name) else {
            return Err(SemanticError::new(
                format!("unknown variable `{name}`"),
                span,
            ));
        };
        if local.moved {
            return Err(SemanticError::new(
                format!("use of moved value `{name}`"),
                span,
            ));
        }

        let ty = local.ty;
        if matches!(value_use, ValueUse::Owned) && !ty.is_copy() {
            local.moved = true;
        }

        Ok(ty)
    }

    fn check_call(
        &self,
        callee: &Expr,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let ExprKind::Var(name) = &callee.kind else {
            return Err(SemanticError::new(
                "only direct function calls are supported in v0",
                callee.span,
            ));
        };

        if name == "print" {
            if args.len() != 1 {
                return Err(SemanticError::new(
                    "`print` expects exactly one argument",
                    span,
                ));
            }
            let arg_ty = self.check_expr(&args[0].expr, locals, ValueUse::Borrow)?;
            if matches!(arg_ty, Type::Unit) {
                return Err(SemanticError::new("cannot print unit value", args[0].span));
            }
            return Ok(Type::Unit);
        }

        let sig = self.function_sig(name, callee.span)?;
        if args.len() != sig.params.len() {
            return Err(SemanticError::new(
                format!(
                    "function `{name}` expects {} arguments, got {}",
                    sig.params.len(),
                    args.len()
                ),
                span,
            ));
        }

        let mut call_borrows = HashMap::new();
        for (arg, param) in args.iter().zip(sig.params.iter()) {
            let arg_ty = match (param.mode, arg.mode) {
                (ParamMode::Owned, ArgMode::Owned) => {
                    self.check_expr(&arg.expr, locals, ValueUse::Owned)?
                }
                (ParamMode::In, ArgMode::In) => {
                    self.register_call_borrow(arg, &mut call_borrows, BorrowKind::Shared)?;
                    self.check_borrow_arg(arg, locals, false)?
                }
                (ParamMode::Mut, ArgMode::Mut) => {
                    self.register_call_borrow(arg, &mut call_borrows, BorrowKind::Exclusive)?;
                    self.check_borrow_arg(arg, locals, true)?
                }
                (ParamMode::Owned, _) => {
                    return Err(SemanticError::new(
                        format!("parameter `{}` expects an owned argument", param.name),
                        arg.span,
                    ));
                }
                (ParamMode::In, _) => {
                    return Err(SemanticError::new(
                        format!("parameter `{}` expects `in` argument", param.name),
                        arg.span,
                    ));
                }
                (ParamMode::Mut, _) => {
                    return Err(SemanticError::new(
                        format!("parameter `{}` expects `mut` argument", param.name),
                        arg.span,
                    ));
                }
            };
            if arg_ty != param.ty {
                return Err(SemanticError::new(
                    format!(
                        "argument type mismatch for `{name}`: expected `{}`, got `{}`",
                        param.ty.source_name(),
                        arg_ty.source_name()
                    ),
                    arg.span,
                ));
            }
        }

        Ok(sig.return_type)
    }

    fn register_call_borrow(
        &self,
        arg: &Arg,
        call_borrows: &mut HashMap<String, BorrowKind>,
        kind: BorrowKind,
    ) -> Result<(), SemanticError> {
        let name = borrow_arg_name(arg)?.to_string();
        match (call_borrows.get(&name).copied(), kind) {
            (None, kind) => {
                call_borrows.insert(name, kind);
                Ok(())
            }
            (Some(BorrowKind::Shared), BorrowKind::Shared) => Ok(()),
            (Some(BorrowKind::Shared), BorrowKind::Exclusive)
            | (Some(BorrowKind::Exclusive), BorrowKind::Shared)
            | (Some(BorrowKind::Exclusive), BorrowKind::Exclusive) => Err(SemanticError::new(
                format!("borrow of `{name}` overlaps with an active borrow in this call"),
                arg.span,
            )),
        }
    }

    fn check_borrow_arg(
        &self,
        arg: &Arg,
        locals: &mut HashMap<String, Local>,
        mutable: bool,
    ) -> Result<Type, SemanticError> {
        let name = borrow_arg_name(arg)?;
        let Some(local) = locals.get(name) else {
            return Err(SemanticError::new(
                format!("unknown variable `{name}`"),
                arg.expr.span,
            ));
        };
        if local.moved {
            return Err(SemanticError::new(
                format!("borrow of moved value `{name}`"),
                arg.expr.span,
            ));
        }
        if mutable && !local.mutable {
            return Err(SemanticError::new(
                format!("cannot mutably borrow immutable binding `{name}`"),
                arg.span,
            ));
        }

        Ok(local.ty)
    }

    fn check_binary(
        &self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        let left_ty = self.check_expr(left, locals, ValueUse::Owned)?;
        let right_ty = self.check_expr(right, locals, ValueUse::Owned)?;
        if left_ty != right_ty {
            return Err(SemanticError::new(
                "binary operands must have the same type",
                left.span.join(right.span),
            ));
        }

        match op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder => {
                if left_ty == Type::Int {
                    Ok(Type::Int)
                } else {
                    Err(SemanticError::new(
                        "arithmetic operators currently require `int` operands",
                        left.span.join(right.span),
                    ))
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                if matches!(left_ty, Type::Int | Type::Bool) {
                    Ok(Type::Bool)
                } else {
                    Err(SemanticError::new(
                        "equality currently supports `int` and `bool` operands",
                        left.span.join(right.span),
                    ))
                }
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                if left_ty == Type::Int {
                    Ok(Type::Bool)
                } else {
                    Err(SemanticError::new(
                        "ordering comparisons currently require `int` operands",
                        left.span.join(right.span),
                    ))
                }
            }
        }
    }

    fn function_sig(&self, name: &str, span: Span) -> Result<&FunctionSig, SemanticError> {
        self.signatures
            .get(name)
            .ok_or_else(|| SemanticError::new(format!("unknown function `{name}`"), span))
    }
}

#[derive(Debug, Clone, Copy)]
struct Local {
    ty: Type,
    mutable: bool,
    moved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueUse {
    Owned,
    Borrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorrowKind {
    Shared,
    Exclusive,
}

fn borrow_arg_name(arg: &Arg) -> Result<&str, SemanticError> {
    let ExprKind::Var(name) = &arg.expr.kind else {
        return Err(SemanticError::new(
            "borrow arguments must be direct local variables in v0",
            arg.span,
        ));
    };
    Ok(name)
}

fn merge_branch_moves(
    locals: &mut HashMap<String, Local>,
    then_locals: &HashMap<String, Local>,
    else_locals: &HashMap<String, Local>,
) {
    for (name, local) in locals {
        let moved_in_then = then_locals.get(name).is_some_and(|branch| branch.moved);
        let moved_in_else = else_locals.get(name).is_some_and(|branch| branch.moved);
        local.moved |= moved_in_then || moved_in_else;
    }
}

fn type_from_optional_ref(ty: Option<&TypeRef>) -> Result<Type, SemanticError> {
    ty.map_or(Ok(Type::Unit), type_from_ref)
}

fn type_from_ref(ty: &TypeRef) -> Result<Type, SemanticError> {
    match ty.name.as_str() {
        "int" => Ok(Type::Int),
        "bool" => Ok(Type::Bool),
        "string" => Ok(Type::String),
        "unit" => Ok(Type::Unit),
        _ => Err(SemanticError::new(
            format!("unknown type `{}`", ty.name),
            ty.span,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn check_ok(source: &str) {
        let program = parse(source).unwrap();
        check(&program).unwrap();
    }

    fn check_error(source: &str) -> SemanticError {
        let program = parse(source).unwrap();
        check(&program).unwrap_err()
    }

    #[test]
    fn checks_first_target_program() {
        check_ok(
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
        );
    }

    #[test]
    fn rejects_unknown_variable() {
        let error = check_error("func main() { print(missing) }");
        assert!(error.message.contains("unknown variable `missing`"));
    }

    #[test]
    fn rejects_nil() {
        let error = check_error("func main() { print(nil) }");
        assert!(error.message.contains("`nil` is reserved"));
    }

    #[test]
    fn rejects_argument_type_mismatch() {
        let error = check_error(
            r#"
func main() {
    print(add("x", 1))
}

func add(a int, b int) int {
    return a + b
}
"#,
        );
        assert!(error.message.contains("argument type mismatch"));
    }

    #[test]
    fn rejects_immutable_assignment() {
        let error = check_error("func main() { x := 1 x = 2 }");
        assert!(error
            .message
            .contains("cannot assign to immutable binding `x`"));
    }

    #[test]
    fn allows_mutable_assignment() {
        check_ok("func main() { mut x := 1 x = 2 print(x) }");
    }

    #[test]
    fn allows_if_expression_value() {
        check_ok(
            r#"
func main() {
    score := 70
    label := if score >= 60 { "pass" } else { "fail" }
    print(label)
}
"#,
        );
    }

    #[test]
    fn rejects_if_condition_that_is_not_bool() {
        let error = check_error("func main() { x := if 1 { 1 } else { 2 } }");
        assert!(error.message.contains("if condition must have type `bool`"));
    }

    #[test]
    fn rejects_if_branch_type_mismatch() {
        let error = check_error("func main() { x := if true { 1 } else { false } }");
        assert!(error
            .message
            .contains("if branches must have the same type"));
    }

    #[test]
    fn ownership_merges_moves_from_if_branches() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    flag := true
    picked := if flag { s } else { "fallback" }
    print(s)
    print(picked)
}
"#,
        );
        assert!(error.message.contains("use of moved value `s`"));
    }

    #[test]
    fn ownership_rejects_use_after_move_for_string() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    consume(s)
    print(s)
}

func consume(s string) {
    print(s)
}
"#,
        );
        assert!(error.message.contains("use of moved value `s`"));
    }

    #[test]
    fn ownership_allows_copy_reuse_for_int() {
        check_ok(
            r#"
func main() {
    x := 1
    printInt(x)
    print(x)
}

func printInt(x int) {
    print(x)
}
"#,
        );
    }

    #[test]
    fn ownership_allows_in_borrow_without_move() {
        check_ok(
            r#"
func main() {
    s := "hello"
    show(in s)
    show(in s)
}

func show(s in string) {
    print(s)
}
"#,
        );
    }

    #[test]
    fn ownership_rejects_missing_in_call_mode() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    show(s)
}

func show(s in string) {
    print(s)
}
"#,
        );
        assert!(error.message.contains("expects `in` argument"));
    }

    #[test]
    fn ownership_rejects_mut_borrow_of_immutable_binding() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    touch(mut s)
}

func touch(s mut string) {
    print(s)
}
"#,
        );
        assert!(error
            .message
            .contains("cannot mutably borrow immutable binding `s`"));
    }

    #[test]
    fn ownership_allows_mut_borrow_of_mutable_binding() {
        check_ok(
            r#"
func main() {
    mut s := "hello"
    touch(mut s)
    print(s)
}

func touch(s mut string) {
    print(s)
}
"#,
        );
    }

    #[test]
    fn borrow_conflict_allows_multiple_shared_borrows_in_one_call() {
        check_ok(
            r#"
func main() {
    s := "hello"
    compare(in s, in s)
}

func compare(left in string, right in string) {
    print(left)
    print(right)
}
"#,
        );
    }

    #[test]
    fn borrow_conflict_rejects_shared_then_mut_borrow_in_one_call() {
        let error = check_error(
            r#"
func main() {
    mut s := "hello"
    compare(in s, mut s)
}

func compare(left in string, right mut string) {
    print(left)
    print(right)
}
"#,
        );
        assert!(error.message.contains("overlaps with an active borrow"));
    }

    #[test]
    fn borrow_conflict_rejects_two_mut_borrows_in_one_call() {
        let error = check_error(
            r#"
func main() {
    mut s := "hello"
    compare(mut s, mut s)
}

func compare(left mut string, right mut string) {
    print(left)
    print(right)
}
"#,
        );
        assert!(error.message.contains("overlaps with an active borrow"));
    }
}
