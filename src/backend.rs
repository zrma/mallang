use std::fmt;

use crate::{
    ast::Program,
    ir::{lower, IrArg, IrExpr, IrExprKind, IrFunction, IrProgram, IrStmt, IrStmtKind},
    semantic::{check, Type},
};

pub fn generate_c(program: &Program) -> Result<String, CompileError> {
    let checked = check(program).map_err(|error| CompileError::new(error.to_string()))?;
    let ir = lower(&checked).map_err(|error| CompileError::new(error.to_string()))?;
    generate_c_from_ir(&ir)
}

pub fn generate_c_from_ir(program: &IrProgram) -> Result<String, CompileError> {
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
    program: &'a IrProgram,
}

impl<'a> CGenerator<'a> {
    fn new(program: &'a IrProgram) -> Self {
        Self { program }
    }

    fn generate(self) -> Result<String, CompileError> {
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

    fn prototype(&self, function: &IrFunction) -> Result<String, CompileError> {
        let params = if function.name == "main" || function.params.is_empty() {
            "void".to_string()
        } else {
            function
                .params
                .iter()
                .map(|param| Ok(format!("{} {}", param.ty.c_name()?, c_ident(&param.name))))
                .collect::<Result<Vec<_>, CompileError>>()?
                .join(", ")
        };

        let return_type = if function.name == "main" {
            "int".to_string()
        } else {
            function.return_type.c_name()?.to_string()
        };

        Ok(format!(
            "{} {}({})",
            return_type,
            c_ident(&function.name),
            params
        ))
    }

    fn emit_function(&self, function: &IrFunction) -> Result<String, CompileError> {
        let mut output = String::new();
        output.push_str(&self.prototype(function)?);
        output.push_str(" {\n");

        for stmt in &function.body {
            let line = self.emit_stmt(stmt)?;
            output.push_str("    ");
            output.push_str(&line);
            output.push('\n');
        }

        if function.name == "main" {
            output.push_str("    return 0;\n");
        }

        output.push_str("}\n");
        Ok(output)
    }

    fn emit_stmt(&self, stmt: &IrStmt) -> Result<String, CompileError> {
        match &stmt.kind {
            IrStmtKind::Let { name, ty, expr, .. } => Ok(format!(
                "{} {} = {};",
                ty.c_name()?,
                c_ident(name),
                self.emit_expr(expr)?
            )),
            IrStmtKind::Assign { name, expr } => {
                Ok(format!("{} = {};", c_ident(name), self.emit_expr(expr)?))
            }
            IrStmtKind::Return { expr } => Ok(format!("return {};", self.emit_expr(expr)?)),
            IrStmtKind::Expr { expr } => {
                if let IrExprKind::Call { callee, args } = &expr.kind {
                    if callee == "print" {
                        return self.emit_print(args);
                    }
                }

                Ok(format!("{};", self.emit_expr(expr)?))
            }
        }
    }

    fn emit_print(&self, args: &[IrArg]) -> Result<String, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::new("IR invariant violation: print arity"));
        }

        let arg = &args[0].expr;
        let code = self.emit_expr(arg)?;
        match &arg.ty {
            Type::Int => Ok(format!("printf(\"%lld\\n\", (long long)({code}));")),
            Type::Bool => Ok(format!(
                "printf(\"%s\\n\", ({code}) ? \"true\" : \"false\");"
            )),
            Type::String => Ok(format!("printf(\"%s\\n\", {code});")),
            Type::Unit => Err(CompileError::new(
                "IR invariant violation: cannot print unit",
            )),
            Type::Option(_) | Type::Result(_, _) => Err(CompileError::new(format!(
                "printing `{}` values is not implemented yet",
                arg.ty.source_name()
            ))),
        }
    }

    fn emit_expr(&self, expr: &IrExpr) -> Result<String, CompileError> {
        match &expr.kind {
            IrExprKind::Int(value) => Ok(value.to_string()),
            IrExprKind::String(value) => Ok(c_string(value)),
            IrExprKind::Bool(value) => Ok(if *value { "true" } else { "false" }.to_string()),
            IrExprKind::Var(name) => Ok(c_ident(name)),
            IrExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(format!(
                "(({}) ? ({}) : ({}))",
                self.emit_expr(condition)?,
                self.emit_expr(then_branch)?,
                self.emit_expr(else_branch)?
            )),
            IrExprKind::Call { callee, args } => {
                if callee == "print" {
                    return Err(CompileError::new(
                        "`print` is only supported as a statement",
                    ));
                }
                let args = args
                    .iter()
                    .map(|arg| self.emit_expr(&arg.expr))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{}({})", c_ident(callee), args.join(", ")))
            }
            IrExprKind::Unary { op, expr } => {
                Ok(format!("({}{})", op.c_operator(), self.emit_expr(expr)?))
            }
            IrExprKind::Binary { op, left, right } => Ok(format!(
                "({} {} {})",
                self.emit_expr(left)?,
                op.c_operator(),
                self.emit_expr(right)?
            )),
        }
    }
}

impl Type {
    fn c_name(&self) -> Result<&'static str, CompileError> {
        match self {
            Self::Int => Ok("int64_t"),
            Self::Bool => Ok("bool"),
            Self::String => Ok("const char *"),
            Self::Unit => Ok("void"),
            Self::Option(_) | Self::Result(_, _) => Err(CompileError::new(format!(
                "C backend layout for `{}` is not implemented yet",
                self.source_name()
            ))),
        }
    }
}

trait COperator {
    fn c_operator(self) -> &'static str;
}

impl COperator for crate::ast::UnaryOp {
    fn c_operator(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

impl COperator for crate::ast::BinaryOp {
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
    use crate::{check, ir::lower, parse};

    #[test]
    fn generates_c_for_first_target_program_from_ir() {
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
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("int main(void)"));
        assert!(c.contains("int64_t mlg_add(int64_t mlg_a, int64_t mlg_b);"));
        assert!(c.contains("printf(\"%lld\\n\", (long long)(mlg_y));"));
    }

    #[test]
    fn generates_c_for_if_expression_from_ir() {
        let program = parse(
            r#"
func main() {
    label := if true { "pass" } else { "fail" }
    print(label)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("const char * mlg_label = ((true) ? (\"pass\") : (\"fail\"));"));
    }

    #[test]
    fn rejects_adt_types_until_backend_layout_exists() {
        let program = parse(
            r#"
func main() {
}

func accept(value Option[int]) {
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let error = generate_c_from_ir(&ir).unwrap_err();

        assert!(error
            .message
            .contains("C backend layout for `Option[int]` is not implemented yet"));
    }
}
