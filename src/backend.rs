use std::{collections::HashMap, fmt};

use crate::ast::{
    Arg, BinaryOp, Expr, ExprKind, Function, ParamMode, Program, Stmt, StmtKind, TypeRef, UnaryOp,
};

pub fn generate_c(program: &Program) -> Result<String, CompileError> {
    CGenerator::new(program).generate()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
}

impl CompileError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}

struct CGenerator<'a> {
    program: &'a Program,
    signatures: HashMap<&'a str, FunctionSig>,
}

impl<'a> CGenerator<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            program,
            signatures: HashMap::new(),
        }
    }

    fn generate(mut self) -> Result<String, CompileError> {
        self.collect_signatures()?;

        let mut output = String::new();
        output.push_str("#include <stdbool.h>\n");
        output.push_str("#include <stdint.h>\n");
        output.push_str("#include <stdio.h>\n\n");

        for function in &self.program.functions {
            output.push_str(&self.prototype(function)?);
            output.push_str(";\n");
        }
        output.push('\n');

        for function in &self.program.functions {
            output.push_str(&self.emit_function(function)?);
            output.push('\n');
        }

        Ok(output)
    }

    fn collect_signatures(&mut self) -> Result<(), CompileError> {
        for function in &self.program.functions {
            if self.signatures.contains_key(function.name.as_str()) {
                return Err(CompileError::new(format!(
                    "duplicate function `{}`",
                    function.name
                )));
            }

            let return_type = if function.name == "main" {
                Type::Int
            } else {
                type_from_optional_ref(function.return_type.as_ref())?
            };
            let mut params = Vec::new();
            for param in &function.params {
                if !matches!(param.mode, ParamMode::Owned) {
                    return Err(CompileError::new(format!(
                        "C backend does not support borrowed parameter `{}` yet",
                        param.name
                    )));
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
            return Err(CompileError::new("program must declare `func main()`"));
        }

        Ok(())
    }

    fn prototype(&self, function: &Function) -> Result<String, CompileError> {
        let sig = self.function_sig(&function.name)?;
        let params = if function.name == "main" {
            if !sig.params.is_empty() {
                return Err(CompileError::new("`main` must not take parameters"));
            }
            "void".to_string()
        } else if sig.params.is_empty() {
            "void".to_string()
        } else {
            sig.params
                .iter()
                .map(|(name, ty)| format!("{} {}", ty.c_name(), c_ident(name)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let return_type = if function.name == "main" {
            "int"
        } else {
            sig.return_type.c_name()
        };

        Ok(format!(
            "{} {}({})",
            return_type,
            c_ident(&function.name),
            params
        ))
    }

    fn emit_function(&self, function: &Function) -> Result<String, CompileError> {
        let mut locals = HashMap::new();
        let sig = self.function_sig(&function.name)?;
        for (name, ty) in &sig.params {
            locals.insert(name.clone(), *ty);
        }

        let mut output = String::new();
        output.push_str(&self.prototype(function)?);
        output.push_str(" {\n");

        let mut returned = false;
        for stmt in &function.body.statements {
            if matches!(stmt.kind, StmtKind::Return { .. }) {
                returned = true;
            }
            let line = self.emit_stmt(stmt, &mut locals, sig.return_type)?;
            output.push_str("    ");
            output.push_str(&line);
            output.push('\n');
        }

        if function.name == "main" {
            output.push_str("    return 0;\n");
        } else if !returned && !matches!(sig.return_type, Type::Unit) {
            return Err(CompileError::new(format!(
                "function `{}` must return `{}`",
                function.name,
                sig.return_type.source_name()
            )));
        }

        output.push_str("}\n");
        Ok(output)
    }

    fn emit_stmt(
        &self,
        stmt: &Stmt,
        locals: &mut HashMap<String, Type>,
        return_type: Type,
    ) -> Result<String, CompileError> {
        match &stmt.kind {
            StmtKind::Let { name, expr, .. } => {
                let typed = self.emit_expr(expr, locals)?;
                locals.insert(name.clone(), typed.ty);
                Ok(format!(
                    "{} {} = {};",
                    typed.ty.c_name(),
                    c_ident(name),
                    typed.code
                ))
            }
            StmtKind::Return { expr } => {
                let typed = self.emit_expr(expr, locals)?;
                if typed.ty != return_type {
                    return Err(CompileError::new(format!(
                        "return type mismatch: expected `{}`, got `{}`",
                        return_type.source_name(),
                        typed.ty.source_name()
                    )));
                }
                Ok(format!("return {};", typed.code))
            }
            StmtKind::Expr { expr } => self.emit_expr_stmt(expr, locals),
        }
    }

    fn emit_expr_stmt(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<String, CompileError> {
        if let ExprKind::Call { callee, args } = &expr.kind {
            if let ExprKind::Var(name) = &callee.kind {
                if name == "print" {
                    return self.emit_print(args, locals);
                }
            }
        }

        let typed = self.emit_expr(expr, locals)?;
        Ok(format!("{};", typed.code))
    }

    fn emit_print(
        &self,
        args: &[Arg],
        locals: &HashMap<String, Type>,
    ) -> Result<String, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::new("`print` expects exactly one argument"));
        }
        let typed = self.emit_expr(&args[0].expr, locals)?;
        match typed.ty {
            Type::Int => Ok(format!("printf(\"%lld\\n\", (long long)({}));", typed.code)),
            Type::Bool => Ok(format!(
                "printf(\"%s\\n\", ({}) ? \"true\" : \"false\");",
                typed.code
            )),
            Type::String => Ok(format!("printf(\"%s\\n\", {});", typed.code)),
            Type::Unit => Err(CompileError::new("cannot print unit value")),
        }
    }

    fn emit_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<TypedCode, CompileError> {
        match &expr.kind {
            ExprKind::Int(value) => Ok(TypedCode {
                ty: Type::Int,
                code: value.to_string(),
            }),
            ExprKind::String(value) => Ok(TypedCode {
                ty: Type::String,
                code: c_string(value),
            }),
            ExprKind::Bool(value) => Ok(TypedCode {
                ty: Type::Bool,
                code: if *value { "true" } else { "false" }.to_string(),
            }),
            ExprKind::Var(name) => {
                let Some(ty) = locals.get(name).copied() else {
                    return Err(CompileError::new(format!("unknown variable `{name}`")));
                };
                Ok(TypedCode {
                    ty,
                    code: c_ident(name),
                })
            }
            ExprKind::Call { callee, args } => self.emit_call(callee, args, locals),
            ExprKind::Unary { op, expr } => {
                let typed = self.emit_expr(expr, locals)?;
                match (op, typed.ty) {
                    (UnaryOp::Negate, Type::Int) => Ok(TypedCode {
                        ty: Type::Int,
                        code: format!("(-{})", typed.code),
                    }),
                    (UnaryOp::Not, Type::Bool) => Ok(TypedCode {
                        ty: Type::Bool,
                        code: format!("(!{})", typed.code),
                    }),
                    _ => Err(CompileError::new("unsupported unary operand type")),
                }
            }
            ExprKind::Binary { op, left, right } => self.emit_binary(*op, left, right, locals),
        }
    }

    fn emit_call(
        &self,
        callee: &Expr,
        args: &[Arg],
        locals: &HashMap<String, Type>,
    ) -> Result<TypedCode, CompileError> {
        let ExprKind::Var(name) = &callee.kind else {
            return Err(CompileError::new(
                "C backend only supports direct function calls",
            ));
        };
        if name == "print" {
            return Err(CompileError::new(
                "`print` is only supported as a statement",
            ));
        }

        let sig = self.function_sig(name)?;
        if args.len() != sig.params.len() {
            return Err(CompileError::new(format!(
                "function `{name}` expects {} arguments, got {}",
                sig.params.len(),
                args.len()
            )));
        }

        let mut emitted_args = Vec::new();
        for (arg, (_, expected_ty)) in args.iter().zip(sig.params.iter()) {
            let typed = self.emit_expr(&arg.expr, locals)?;
            if typed.ty != *expected_ty {
                return Err(CompileError::new(format!(
                    "argument type mismatch for `{name}`: expected `{}`, got `{}`",
                    expected_ty.source_name(),
                    typed.ty.source_name()
                )));
            }
            emitted_args.push(typed.code);
        }

        Ok(TypedCode {
            ty: sig.return_type,
            code: format!("{}({})", c_ident(name), emitted_args.join(", ")),
        })
    }

    fn emit_binary(
        &self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<TypedCode, CompileError> {
        let left = self.emit_expr(left, locals)?;
        let right = self.emit_expr(right, locals)?;
        if left.ty != right.ty {
            return Err(CompileError::new("binary operands must have the same type"));
        }

        match op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder => {
                if left.ty != Type::Int {
                    return Err(CompileError::new(
                        "arithmetic operators currently require `int` operands",
                    ));
                }
                Ok(TypedCode {
                    ty: Type::Int,
                    code: format!("({} {} {})", left.code, op.c_operator(), right.code),
                })
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                if left.ty != Type::Int && left.ty != Type::Bool {
                    return Err(CompileError::new(
                        "comparison operators currently support `int` and `bool` operands",
                    ));
                }
                Ok(TypedCode {
                    ty: Type::Bool,
                    code: format!("({} {} {})", left.code, op.c_operator(), right.code),
                })
            }
        }
    }

    fn function_sig(&self, name: &str) -> Result<&FunctionSig, CompileError> {
        self.signatures
            .get(name)
            .ok_or_else(|| CompileError::new(format!("unknown function `{name}`")))
    }
}

#[derive(Debug, Clone)]
struct FunctionSig {
    return_type: Type,
    params: Vec<(String, Type)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
    Int,
    Bool,
    String,
    Unit,
}

impl Type {
    fn c_name(self) -> &'static str {
        match self {
            Self::Int => "int64_t",
            Self::Bool => "bool",
            Self::String => "const char *",
            Self::Unit => "void",
        }
    }

    fn source_name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Bool => "bool",
            Self::String => "string",
            Self::Unit => "unit",
        }
    }
}

#[derive(Debug, Clone)]
struct TypedCode {
    ty: Type,
    code: String,
}

impl BinaryOp {
    fn c_operator(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
        }
    }
}

fn type_from_optional_ref(ty: Option<&TypeRef>) -> Result<Type, CompileError> {
    ty.map_or(Ok(Type::Unit), type_from_ref)
}

fn type_from_ref(ty: &TypeRef) -> Result<Type, CompileError> {
    match ty.name.as_str() {
        "int" => Ok(Type::Int),
        "bool" => Ok(Type::Bool),
        "string" => Ok(Type::String),
        "unit" => Ok(Type::Unit),
        _ => Err(CompileError::new(format!("unknown type `{}`", ty.name))),
    }
}

fn c_ident(name: &str) -> String {
    if name == "main" {
        return name.to_string();
    }
    format!("mlg_{name}")
}

fn c_string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\n' => output.push_str("\\n"),
            '\t' => output.push_str("\\t"),
            _ => output.push(ch),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn generates_c_for_first_target_program() {
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
        let c = generate_c(&program).unwrap();

        assert!(c.contains("int main(void)"));
        assert!(c.contains("int64_t mlg_add(int64_t mlg_a, int64_t mlg_b);"));
        assert!(c.contains("printf(\"%lld\\n\", (long long)(mlg_y));"));
    }
}
