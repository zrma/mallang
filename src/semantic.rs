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
    pub params: Vec<(String, Type)>,
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
                if !matches!(param.mode, ParamMode::Owned) {
                    return Err(SemanticError::new(
                        format!(
                            "borrowed parameter `{}` is planned but not supported yet",
                            param.name
                        ),
                        param.span,
                    ));
                }
                params.push((param.name.clone(), type_from_ref(&param.ty)?));
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
        for (name, ty) in &sig.params {
            if locals
                .insert(
                    name.clone(),
                    Local {
                        ty: *ty,
                        mutable: false,
                    },
                )
                .is_some()
            {
                return Err(SemanticError::new(
                    format!("duplicate parameter `{name}`"),
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
                let ty = self.check_expr(expr, locals)?;
                if locals
                    .insert(
                        name.clone(),
                        Local {
                            ty,
                            mutable: *mutable,
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
                let value_ty = self.check_expr(expr, locals)?;
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
                let value_ty = self.check_expr(expr, locals)?;
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
            StmtKind::Expr { expr } => self.check_expr(expr, locals),
        }
    }

    fn check_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        match &expr.kind {
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::String(_) => Ok(Type::String),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::Nil => Err(SemanticError::new(
                "`nil` is reserved; use Option[T] when optional values are implemented",
                expr.span,
            )),
            ExprKind::Var(name) => locals
                .get(name)
                .map(|local| local.ty)
                .ok_or_else(|| SemanticError::new(format!("unknown variable `{name}`"), expr.span)),
            ExprKind::Call { callee, args } => self.check_call(callee, args, locals, expr.span),
            ExprKind::Unary { op, expr } => {
                let ty = self.check_expr(expr, locals)?;
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

    fn check_call(
        &self,
        callee: &Expr,
        args: &[Arg],
        locals: &HashMap<String, Local>,
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
            let arg_ty = self.check_expr(&args[0].expr, locals)?;
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

        for (arg, (_, expected_ty)) in args.iter().zip(sig.params.iter()) {
            if !matches!(arg.mode, ArgMode::Owned) {
                return Err(SemanticError::new(
                    "borrowed call arguments are planned but not supported yet",
                    arg.span,
                ));
            }
            let arg_ty = self.check_expr(&arg.expr, locals)?;
            if arg_ty != *expected_ty {
                return Err(SemanticError::new(
                    format!(
                        "argument type mismatch for `{name}`: expected `{}`, got `{}`",
                        expected_ty.source_name(),
                        arg_ty.source_name()
                    ),
                    arg.span,
                ));
            }
        }

        Ok(sig.return_type)
    }

    fn check_binary(
        &self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        locals: &HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        let left_ty = self.check_expr(left, locals)?;
        let right_ty = self.check_expr(right, locals)?;
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
}
