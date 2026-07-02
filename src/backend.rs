use std::{collections::HashMap, fmt};

use crate::{
    ast::Program,
    ir::{
        lower, IrAdtConstructor, IrArg, IrExpr, IrExprKind, IrFunction, IrMatchArm,
        IrMatchBlockArm, IrMatchPattern, IrProgram, IrStmt, IrStmtKind,
    },
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

struct CExpr {
    prelude: Vec<String>,
    code: String,
}

impl CExpr {
    fn simple(code: String) -> Self {
        Self {
            prelude: Vec::new(),
            code,
        }
    }
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

        let defined_types = self.collect_defined_types();
        let mut emitted_types = Vec::new();
        for ty in &defined_types {
            self.emit_type_def(ty, &mut emitted_types, &mut Vec::new(), &mut output)?;
        }
        if !emitted_types.is_empty() {
            output.push('\n');
        }

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
                .map(|param| format!("{} {}", param.ty.c_name(), c_ident(&param.name)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let return_type = if function.name == "main" {
            "int".to_string()
        } else {
            function.return_type.c_name()
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
            push_indented_lines(&mut output, &line, 1);
        }

        if function.name == "main" {
            output.push_str("    return 0;\n");
        }

        output.push_str("}\n");
        Ok(output)
    }

    fn emit_stmt(&self, stmt: &IrStmt) -> Result<String, CompileError> {
        self.emit_stmt_with_env(stmt, &HashMap::new())
    }

    fn emit_stmt_with_env(
        &self,
        stmt: &IrStmt,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        match &stmt.kind {
            IrStmtKind::Let { name, ty, expr, .. } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_prelude(
                    prelude,
                    format!("{} {} = {};", ty.c_name(), c_ident(name), code),
                ))
            }
            IrStmtKind::Assign { name, expr } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_prelude(
                    prelude,
                    format!("{} = {};", c_ident(name), code),
                ))
            }
            IrStmtKind::FieldAssign { base, field, expr } => {
                let base = self.emit_stmt_expr_with_env(base, env)?;
                let expr = self.emit_stmt_expr_with_env(expr, env)?;
                let mut prelude = base.prelude;
                prelude.extend(expr.prelude);
                Ok(finish_with_prelude(
                    prelude,
                    format!("({}).{} = {};", base.code, c_field(field), expr.code),
                ))
            }
            IrStmtKind::Return { expr } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_prelude(prelude, format!("return {};", code)))
            }
            IrStmtKind::If {
                condition,
                then_body,
                else_body,
            } => self.emit_if_stmt(condition, then_body, else_body, env),
            IrStmtKind::Match { scrutinee, arms } => self.emit_match_stmt(scrutinee, arms, env),
            IrStmtKind::Expr { expr } => {
                if let IrExprKind::Call { callee, args } = &expr.kind {
                    if callee == "print" {
                        return self.emit_print(args, env);
                    }
                }

                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_prelude(prelude, format!("{code};")))
            }
        }
    }

    fn emit_if_stmt(
        &self,
        condition: &IrExpr,
        then_body: &[IrStmt],
        else_body: &[IrStmt],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let mut output = String::new();
        let CExpr { prelude, code } = self.emit_stmt_expr_with_env(condition, env)?;
        for line in prelude {
            output.push_str(&line);
            output.push('\n');
        }
        output.push_str(&format!("if ({code}) {{\n"));
        for stmt in then_body {
            let code = self.emit_stmt_with_env(stmt, env)?;
            push_indented_lines(&mut output, &code, 1);
        }
        if else_body.is_empty() {
            output.push('}');
            return Ok(output);
        }

        output.push_str("} else {\n");
        for stmt in else_body {
            let code = self.emit_stmt_with_env(stmt, env)?;
            push_indented_lines(&mut output, &code, 1);
        }
        output.push('}');
        Ok(output)
    }

    fn emit_match_stmt(
        &self,
        scrutinee: &IrExpr,
        arms: &[IrMatchBlockArm],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let mut output = String::new();
        let scrutinee_code = if let IrExprKind::Var(name) = &scrutinee.kind {
            env.get(name).cloned().unwrap_or_else(|| c_ident(name))
        } else {
            let CExpr { prelude, code } = self.emit_stmt_expr_with_env(scrutinee, env)?;
            for line in prelude {
                output.push_str(&line);
                output.push('\n');
            }
            let temp = match_scrutinee_temp_name(scrutinee);
            output.push_str(&format!("{} {temp} = {code};\n", scrutinee.ty.c_name()));
            temp
        };

        match &scrutinee.ty {
            Type::Option(_) => self.emit_option_match_stmt(&scrutinee_code, arms, env, output),
            Type::Result(_, _) => self.emit_result_match_stmt(&scrutinee_code, arms, env, output),
            _ => Err(CompileError::new(
                "IR invariant violation: match on non-ADT value",
            )),
        }
    }

    fn emit_option_match_stmt(
        &self,
        scrutinee: &str,
        arms: &[IrMatchBlockArm],
        env: &HashMap<String, String>,
        mut output: String,
    ) -> Result<String, CompileError> {
        output.push_str(&format!("switch (({scrutinee}).tag) {{\n"));
        for arm in arms {
            match &arm.pattern {
                IrMatchPattern::Some(binding) => {
                    output.push_str("case 1: {\n");
                    let mut arm_env = env.clone();
                    arm_env.insert(binding.clone(), format!("({scrutinee}).some"));
                    self.emit_match_stmt_body(&arm.body, &arm_env, &mut output)?;
                    output.push_str("    break;\n");
                    output.push_str("}\n");
                }
                IrMatchPattern::None => {
                    output.push_str("case 0: {\n");
                    self.emit_match_stmt_body(&arm.body, env, &mut output)?;
                    output.push_str("    break;\n");
                    output.push_str("}\n");
                }
                IrMatchPattern::Ok(_) | IrMatchPattern::Err(_) => {
                    return Err(CompileError::new(
                        "IR invariant violation: invalid Option match arm",
                    ));
                }
            }
        }
        output.push('}');
        Ok(output)
    }

    fn emit_result_match_stmt(
        &self,
        scrutinee: &str,
        arms: &[IrMatchBlockArm],
        env: &HashMap<String, String>,
        mut output: String,
    ) -> Result<String, CompileError> {
        output.push_str(&format!("switch (({scrutinee}).tag) {{\n"));
        for arm in arms {
            match &arm.pattern {
                IrMatchPattern::Ok(binding) => {
                    output.push_str("case 0: {\n");
                    let mut arm_env = env.clone();
                    arm_env.insert(binding.clone(), format!("({scrutinee}).ok"));
                    self.emit_match_stmt_body(&arm.body, &arm_env, &mut output)?;
                    output.push_str("    break;\n");
                    output.push_str("}\n");
                }
                IrMatchPattern::Err(binding) => {
                    output.push_str("case 1: {\n");
                    let mut arm_env = env.clone();
                    arm_env.insert(binding.clone(), format!("({scrutinee}).err"));
                    self.emit_match_stmt_body(&arm.body, &arm_env, &mut output)?;
                    output.push_str("    break;\n");
                    output.push_str("}\n");
                }
                IrMatchPattern::Some(_) | IrMatchPattern::None => {
                    return Err(CompileError::new(
                        "IR invariant violation: invalid Result match arm",
                    ));
                }
            }
        }
        output.push('}');
        Ok(output)
    }

    fn emit_match_stmt_body(
        &self,
        body: &[IrStmt],
        env: &HashMap<String, String>,
        output: &mut String,
    ) -> Result<(), CompileError> {
        for stmt in body {
            let code = self.emit_stmt_with_env(stmt, env)?;
            push_indented_lines(output, &code, 1);
        }
        Ok(())
    }

    fn emit_print(
        &self,
        args: &[IrArg],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        if args.len() != 1 {
            return Err(CompileError::new("IR invariant violation: print arity"));
        }

        let arg = &args[0].expr;
        let CExpr { prelude, code } = self.emit_stmt_expr_with_env(arg, env)?;
        match &arg.ty {
            Type::Int => Ok(finish_with_prelude(
                prelude,
                format!("printf(\"%lld\\n\", (long long)({code}));"),
            )),
            Type::Bool => Ok(finish_with_prelude(
                prelude,
                format!("printf(\"%s\\n\", ({code}) ? \"true\" : \"false\");"),
            )),
            Type::String => Ok(finish_with_prelude(
                prelude,
                format!("printf(\"%s\\n\", {code});"),
            )),
            Type::Unit => Err(CompileError::new(
                "IR invariant violation: cannot print unit",
            )),
            Type::Option(_) | Type::Result(_, _) | Type::Struct(_) => {
                Err(CompileError::new(format!(
                    "printing `{}` values is not implemented yet",
                    arg.ty.source_name()
                )))
            }
        }
    }

    fn emit_stmt_expr_with_env(
        &self,
        expr: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        match &expr.kind {
            IrExprKind::Int(value) => Ok(CExpr::simple(value.to_string())),
            IrExprKind::String(value) => Ok(CExpr::simple(c_string(value))),
            IrExprKind::Bool(value) => Ok(CExpr::simple(
                if *value { "true" } else { "false" }.to_string(),
            )),
            IrExprKind::Var(name) => Ok(CExpr::simple(
                env.get(name).cloned().unwrap_or_else(|| c_ident(name)),
            )),
            IrExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(condition, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!(
                        "(({}) ? ({}) : ({}))",
                        code,
                        self.emit_expr_with_env(then_branch, env)?,
                        self.emit_expr_with_env(else_branch, env)?
                    ),
                })
            }
            IrExprKind::AdtConstructor {
                constructor,
                payload,
            } => {
                self.emit_adt_constructor_stmt_expr(&expr.ty, *constructor, payload.as_deref(), env)
            }
            IrExprKind::Match { scrutinee, arms } => {
                self.emit_match_stmt_expr(scrutinee, arms, env)
            }
            IrExprKind::StructLiteral { type_name, fields } => {
                self.emit_struct_literal_stmt_expr(type_name, fields, env)
            }
            IrExprKind::FieldAccess { base, field } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(base, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({}).{}", code, c_field(field)),
                })
            }
            IrExprKind::Call { callee, args } => {
                if callee == "print" {
                    return Err(CompileError::new(
                        "`print` is only supported as a statement",
                    ));
                }

                let mut prelude = Vec::new();
                let mut arg_codes = Vec::new();
                for arg in args {
                    let emitted = self.emit_stmt_expr_with_env(&arg.expr, env)?;
                    prelude.extend(emitted.prelude);
                    arg_codes.push(emitted.code);
                }
                Ok(CExpr {
                    prelude,
                    code: format!("{}({})", c_ident(callee), arg_codes.join(", ")),
                })
            }
            IrExprKind::Unary { op, expr } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({}{})", op.c_operator(), code),
                })
            }
            IrExprKind::Binary { op, left, right } => {
                let left = self.emit_stmt_expr_with_env(left, env)?;
                let right = self.emit_stmt_expr_with_env(right, env)?;
                let mut prelude = left.prelude;
                prelude.extend(right.prelude);
                Ok(CExpr {
                    prelude,
                    code: format!("({} {} {})", left.code, op.c_operator(), right.code),
                })
            }
        }
    }

    fn emit_expr_with_env(
        &self,
        expr: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        match &expr.kind {
            IrExprKind::Int(value) => Ok(value.to_string()),
            IrExprKind::String(value) => Ok(c_string(value)),
            IrExprKind::Bool(value) => Ok(if *value { "true" } else { "false" }.to_string()),
            IrExprKind::Var(name) => Ok(env.get(name).cloned().unwrap_or_else(|| c_ident(name))),
            IrExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => Ok(format!(
                "(({}) ? ({}) : ({}))",
                self.emit_expr_with_env(condition, env)?,
                self.emit_expr_with_env(then_branch, env)?,
                self.emit_expr_with_env(else_branch, env)?
            )),
            IrExprKind::AdtConstructor {
                constructor,
                payload,
            } => self.emit_adt_constructor(&expr.ty, *constructor, payload.as_deref(), env),
            IrExprKind::Match { scrutinee, arms } => self.emit_match(scrutinee, arms, env),
            IrExprKind::StructLiteral { type_name, fields } => {
                self.emit_struct_literal(type_name, fields, env)
            }
            IrExprKind::FieldAccess { base, field } => Ok(format!(
                "({}).{}",
                self.emit_expr_with_env(base, env)?,
                c_field(field)
            )),
            IrExprKind::Call { callee, args } => {
                if callee == "print" {
                    return Err(CompileError::new(
                        "`print` is only supported as a statement",
                    ));
                }
                let args = args
                    .iter()
                    .map(|arg| self.emit_expr_with_env(&arg.expr, env))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(format!("{}({})", c_ident(callee), args.join(", ")))
            }
            IrExprKind::Unary { op, expr } => Ok(format!(
                "({}{})",
                op.c_operator(),
                self.emit_expr_with_env(expr, env)?
            )),
            IrExprKind::Binary { op, left, right } => Ok(format!(
                "({} {} {})",
                self.emit_expr_with_env(left, env)?,
                op.c_operator(),
                self.emit_expr_with_env(right, env)?
            )),
        }
    }

    fn emit_adt_constructor_stmt_expr(
        &self,
        ty: &Type,
        constructor: IrAdtConstructor,
        payload: Option<&IrExpr>,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let c_type = ty.c_name();
        match (ty, constructor) {
            (Type::Option(_), IrAdtConstructor::Some) => {
                let payload = payload.ok_or_else(|| {
                    CompileError::new("IR invariant violation: Some payload missing")
                })?;
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(payload, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({c_type}){{ .tag = 1, .some = {code} }}"),
                })
            }
            (Type::Option(_), IrAdtConstructor::None) => {
                Ok(CExpr::simple(format!("({c_type}){{ .tag = 0 }}")))
            }
            (Type::Result(_, _), IrAdtConstructor::Ok) => {
                let payload = payload.ok_or_else(|| {
                    CompileError::new("IR invariant violation: Ok payload missing")
                })?;
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(payload, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({c_type}){{ .tag = 0, .ok = {code} }}"),
                })
            }
            (Type::Result(_, _), IrAdtConstructor::Err) => {
                let payload = payload.ok_or_else(|| {
                    CompileError::new("IR invariant violation: Err payload missing")
                })?;
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(payload, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({c_type}){{ .tag = 1, .err = {code} }}"),
                })
            }
            _ => Err(CompileError::new(format!(
                "IR invariant violation: `{}` constructor does not match `{}`",
                constructor.c_name(),
                ty.source_name()
            ))),
        }
    }

    fn emit_adt_constructor(
        &self,
        ty: &Type,
        constructor: IrAdtConstructor,
        payload: Option<&IrExpr>,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let c_type = ty.c_name();
        match (ty, constructor) {
            (Type::Option(_), IrAdtConstructor::Some) => {
                let payload = payload.ok_or_else(|| {
                    CompileError::new("IR invariant violation: Some payload missing")
                })?;
                Ok(format!(
                    "({c_type}){{ .tag = 1, .some = {} }}",
                    self.emit_expr_with_env(payload, env)?
                ))
            }
            (Type::Option(_), IrAdtConstructor::None) => Ok(format!("({c_type}){{ .tag = 0 }}")),
            (Type::Result(_, _), IrAdtConstructor::Ok) => {
                let payload = payload.ok_or_else(|| {
                    CompileError::new("IR invariant violation: Ok payload missing")
                })?;
                Ok(format!(
                    "({c_type}){{ .tag = 0, .ok = {} }}",
                    self.emit_expr_with_env(payload, env)?
                ))
            }
            (Type::Result(_, _), IrAdtConstructor::Err) => {
                let payload = payload.ok_or_else(|| {
                    CompileError::new("IR invariant violation: Err payload missing")
                })?;
                Ok(format!(
                    "({c_type}){{ .tag = 1, .err = {} }}",
                    self.emit_expr_with_env(payload, env)?
                ))
            }
            _ => Err(CompileError::new(format!(
                "IR invariant violation: `{}` constructor does not match `{}`",
                constructor.c_name(),
                ty.source_name()
            ))),
        }
    }

    fn emit_struct_literal_stmt_expr(
        &self,
        type_name: &str,
        fields: &[crate::ir::IrFieldValue],
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let mut prelude = Vec::new();
        let mut field_codes = Vec::new();
        for field in fields {
            let emitted = self.emit_stmt_expr_with_env(&field.expr, env)?;
            prelude.extend(emitted.prelude);
            field_codes.push(format!(".{} = {}", c_field(&field.name), emitted.code));
        }

        Ok(CExpr {
            prelude,
            code: format!(
                "({}){{ {} }}",
                Type::Struct(type_name.to_string()).c_name(),
                field_codes.join(", ")
            ),
        })
    }

    fn emit_struct_literal(
        &self,
        type_name: &str,
        fields: &[crate::ir::IrFieldValue],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let field_codes = fields
            .iter()
            .map(|field| {
                Ok(format!(
                    ".{} = {}",
                    c_field(&field.name),
                    self.emit_expr_with_env(&field.expr, env)?
                ))
            })
            .collect::<Result<Vec<_>, CompileError>>()?;

        Ok(format!(
            "({}){{ {} }}",
            Type::Struct(type_name.to_string()).c_name(),
            field_codes.join(", ")
        ))
    }

    fn emit_match(
        &self,
        scrutinee: &IrExpr,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let IrExprKind::Var(name) = &scrutinee.kind else {
            return Err(CompileError::new(
                "C backend currently requires match scrutinee to be a local variable",
            ));
        };
        let scrutinee_code = env.get(name).cloned().unwrap_or_else(|| c_ident(name));

        self.emit_match_with_scrutinee(&scrutinee_code, &scrutinee.ty, arms, env)
    }

    fn emit_match_stmt_expr(
        &self,
        scrutinee: &IrExpr,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        if matches!(scrutinee.kind, IrExprKind::Var(_)) {
            return Ok(CExpr::simple(self.emit_match(scrutinee, arms, env)?));
        }

        let CExpr { mut prelude, code } = self.emit_stmt_expr_with_env(scrutinee, env)?;
        let temp = match_scrutinee_temp_name(scrutinee);
        prelude.push(format!("{} {temp} = {code};", scrutinee.ty.c_name()));
        let code = self.emit_match_with_scrutinee(&temp, &scrutinee.ty, arms, env)?;
        Ok(CExpr { prelude, code })
    }

    fn emit_match_with_scrutinee(
        &self,
        scrutinee: &str,
        scrutinee_ty: &Type,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        match scrutinee_ty {
            Type::Option(_) => self.emit_option_match(scrutinee, arms, env),
            Type::Result(_, _) => self.emit_result_match(scrutinee, arms, env),
            _ => Err(CompileError::new(
                "IR invariant violation: match on non-ADT value",
            )),
        }
    }

    fn emit_option_match(
        &self,
        scrutinee: &str,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let some_arm = arms
            .iter()
            .find_map(|arm| match &arm.pattern {
                IrMatchPattern::Some(binding) => Some((binding, &arm.expr)),
                _ => None,
            })
            .ok_or_else(|| CompileError::new("IR invariant violation: missing Some arm"))?;
        let none_expr = arms
            .iter()
            .find_map(|arm| match arm.pattern {
                IrMatchPattern::None => Some(&arm.expr),
                _ => None,
            })
            .ok_or_else(|| CompileError::new("IR invariant violation: missing None arm"))?;

        let mut some_env = env.clone();
        some_env.insert(some_arm.0.clone(), format!("({scrutinee}).some"));
        Ok(format!(
            "((({scrutinee}).tag == 1) ? ({}) : ({}))",
            self.emit_expr_with_env(some_arm.1, &some_env)?,
            self.emit_expr_with_env(none_expr, env)?
        ))
    }

    fn emit_result_match(
        &self,
        scrutinee: &str,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let ok_arm = arms
            .iter()
            .find_map(|arm| match &arm.pattern {
                IrMatchPattern::Ok(binding) => Some((binding, &arm.expr)),
                _ => None,
            })
            .ok_or_else(|| CompileError::new("IR invariant violation: missing Ok arm"))?;
        let err_arm = arms
            .iter()
            .find_map(|arm| match &arm.pattern {
                IrMatchPattern::Err(binding) => Some((binding, &arm.expr)),
                _ => None,
            })
            .ok_or_else(|| CompileError::new("IR invariant violation: missing Err arm"))?;

        let mut ok_env = env.clone();
        ok_env.insert(ok_arm.0.clone(), format!("({scrutinee}).ok"));
        let mut err_env = env.clone();
        err_env.insert(err_arm.0.clone(), format!("({scrutinee}).err"));
        Ok(format!(
            "((({scrutinee}).tag == 0) ? ({}) : ({}))",
            self.emit_expr_with_env(ok_arm.1, &ok_env)?,
            self.emit_expr_with_env(err_arm.1, &err_env)?
        ))
    }

    fn collect_defined_types(&self) -> Vec<Type> {
        let mut types = Vec::new();
        for struct_def in &self.program.structs {
            collect_type(&Type::Struct(struct_def.name.clone()), &mut types);
            for field in &struct_def.fields {
                collect_type(&field.ty, &mut types);
            }
        }
        for function in &self.program.functions {
            collect_type(&function.return_type, &mut types);
            for param in &function.params {
                collect_type(&param.ty, &mut types);
            }
            for stmt in &function.body {
                self.collect_stmt_types(stmt, &mut types);
            }
        }
        types
    }

    fn emit_type_def(
        &self,
        ty: &Type,
        emitted: &mut Vec<Type>,
        visiting: &mut Vec<Type>,
        output: &mut String,
    ) -> Result<(), CompileError> {
        if emitted.contains(ty) || matches!(ty, Type::Int | Type::Bool | Type::String | Type::Unit)
        {
            return Ok(());
        }
        if visiting.contains(ty) {
            return Err(CompileError::new(format!(
                "recursive type definition involving `{}` is not supported in v0",
                ty.source_name()
            )));
        }

        visiting.push(ty.clone());
        match ty {
            Type::Option(inner) => {
                self.emit_type_def(inner, emitted, visiting, output)?;
                output.push_str(&self.typedef_for_adt(ty)?);
                output.push('\n');
            }
            Type::Result(ok, err) => {
                self.emit_type_def(ok, emitted, visiting, output)?;
                self.emit_type_def(err, emitted, visiting, output)?;
                output.push_str(&self.typedef_for_adt(ty)?);
                output.push('\n');
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                for field in &struct_def.fields {
                    self.emit_type_def(&field.ty, emitted, visiting, output)?;
                }
                output.push_str(&self.typedef_for_struct(struct_def));
                output.push('\n');
            }
            Type::Int | Type::Bool | Type::String | Type::Unit => {}
        }
        visiting.pop();
        emitted.push(ty.clone());
        Ok(())
    }

    fn struct_def(&self, name: &str) -> Result<&crate::ir::IrStruct, CompileError> {
        self.program
            .structs
            .iter()
            .find(|struct_def| struct_def.name == name)
            .ok_or_else(|| {
                CompileError::new(format!("IR invariant violation: unknown struct `{name}`"))
            })
    }

    fn collect_stmt_types(&self, stmt: &IrStmt, types: &mut Vec<Type>) {
        match &stmt.kind {
            IrStmtKind::Let { ty, expr, .. } => {
                collect_type(ty, types);
                self.collect_expr_types(expr, types);
            }
            IrStmtKind::Assign { expr, .. }
            | IrStmtKind::Return { expr }
            | IrStmtKind::Expr { expr } => self.collect_expr_types(expr, types),
            IrStmtKind::FieldAssign { base, expr, .. } => {
                self.collect_expr_types(base, types);
                self.collect_expr_types(expr, types);
            }
            IrStmtKind::If {
                condition,
                then_body,
                else_body,
            } => {
                self.collect_expr_types(condition, types);
                for stmt in then_body {
                    self.collect_stmt_types(stmt, types);
                }
                for stmt in else_body {
                    self.collect_stmt_types(stmt, types);
                }
            }
            IrStmtKind::Match { scrutinee, arms } => {
                self.collect_expr_types(scrutinee, types);
                for arm in arms {
                    for stmt in &arm.body {
                        self.collect_stmt_types(stmt, types);
                    }
                }
            }
        }
    }

    fn collect_expr_types(&self, expr: &IrExpr, types: &mut Vec<Type>) {
        collect_type(&expr.ty, types);
        match &expr.kind {
            IrExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_expr_types(condition, types);
                self.collect_expr_types(then_branch, types);
                self.collect_expr_types(else_branch, types);
            }
            IrExprKind::AdtConstructor { payload, .. } => {
                if let Some(payload) = payload {
                    self.collect_expr_types(payload, types);
                }
            }
            IrExprKind::Match { scrutinee, arms } => {
                self.collect_expr_types(scrutinee, types);
                for arm in arms {
                    self.collect_expr_types(&arm.expr, types);
                }
            }
            IrExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_expr_types(&field.expr, types);
                }
            }
            IrExprKind::FieldAccess { base, .. } => self.collect_expr_types(base, types),
            IrExprKind::Call { args, .. } => {
                for arg in args {
                    self.collect_expr_types(&arg.expr, types);
                }
            }
            IrExprKind::Unary { expr, .. } => self.collect_expr_types(expr, types),
            IrExprKind::Binary { left, right, .. } => {
                self.collect_expr_types(left, types);
                self.collect_expr_types(right, types);
            }
            IrExprKind::Int(_)
            | IrExprKind::String(_)
            | IrExprKind::Bool(_)
            | IrExprKind::Var(_) => {}
        }
    }

    fn typedef_for_adt(&self, ty: &Type) -> Result<String, CompileError> {
        match ty {
            Type::Option(inner) => Ok(format!(
                "typedef struct {{\n    int32_t tag;\n    {} some;\n}} {};\n",
                inner.c_name(),
                ty.c_name()
            )),
            Type::Result(ok, err) => Ok(format!(
                "typedef struct {{\n    int32_t tag;\n    {} ok;\n    {} err;\n}} {};\n",
                ok.c_name(),
                err.c_name(),
                ty.c_name()
            )),
            _ => Err(CompileError::new("internal error: expected ADT type")),
        }
    }

    fn typedef_for_struct(&self, struct_def: &crate::ir::IrStruct) -> String {
        let mut output = String::new();
        output.push_str("typedef struct {\n");
        for field in &struct_def.fields {
            output.push_str("    ");
            output.push_str(&field.ty.c_name());
            output.push(' ');
            output.push_str(&c_field(&field.name));
            output.push_str(";\n");
        }
        output.push_str("} ");
        output.push_str(&Type::Struct(struct_def.name.clone()).c_name());
        output.push_str(";\n");
        output
    }
}

impl Type {
    fn c_name(&self) -> String {
        match self {
            Self::Int => "int64_t".to_string(),
            Self::Bool => "bool".to_string(),
            Self::String => "const char *".to_string(),
            Self::Unit => "void".to_string(),
            Self::Option(_) | Self::Result(_, _) => format!("mlg_{}", mangle_type(self)),
            Self::Struct(name) => format!("mlg_struct_{}", c_type_ident(name)),
        }
    }
}

impl IrAdtConstructor {
    fn c_name(self) -> &'static str {
        match self {
            Self::Some => "Some",
            Self::None => "None",
            Self::Ok => "Ok",
            Self::Err => "Err",
        }
    }
}

fn collect_type(ty: &Type, types: &mut Vec<Type>) {
    match ty {
        Type::Option(inner) => {
            collect_type(inner, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Result(ok, err) => {
            collect_type(ok, types);
            collect_type(err, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Struct(_) => {
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Int | Type::Bool | Type::String | Type::Unit => {}
    }
}

fn finish_with_prelude(prelude: Vec<String>, body: String) -> String {
    let mut output = String::new();
    for line in prelude {
        output.push_str(&line);
        output.push('\n');
    }
    output.push_str(&body);
    output
}

fn push_indented_lines(output: &mut String, code: &str, level: usize) {
    let indent = "    ".repeat(level);
    for line in code.lines() {
        if line.is_empty() {
            output.push('\n');
        } else {
            output.push_str(&indent);
            output.push_str(line);
            output.push('\n');
        }
    }
}

fn match_scrutinee_temp_name(expr: &IrExpr) -> String {
    format!("mallang_match_tmp_{}", expr.span.start)
}

fn mangle_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Unit => "unit".to_string(),
        Type::Option(inner) => format!("Option_{}", mangle_type(inner)),
        Type::Result(ok, err) => format!("Result_{}_{}", mangle_type(ok), mangle_type(err)),
        Type::Struct(name) => format!("Struct_{}", c_type_ident(name)),
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
    format!("mlg_{}", c_type_ident(name))
}

fn c_field(name: &str) -> String {
    format!("mlg_{name}")
}

fn c_type_ident(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
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
    fn generates_c_for_if_statement_from_ir() {
        let program = parse(
            r#"
func main() {
    if true {
        print("yes")
    } else {
        print("no")
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("if (true) {"));
        assert!(c.contains("} else {"));
        assert!(c.contains("printf(\"%s\\n\", \"yes\");"));
    }

    #[test]
    fn generates_c_for_adt_constructors_and_match() {
        let program = parse(
            r#"
func main() {
    print(unwrap(maybe(false)))
}

func maybe(flag bool) Option[int] {
    return if flag { Some(1) } else { None }
}

func unwrap(value Option[int]) int {
    return match value {
        case Some(inner) { inner }
        case None { 0 }
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("typedef struct"));
        assert!(c.contains("mlg_Option_int"));
        assert!(c.contains(".tag = 1"));
        assert!(c.contains(".tag = 0"));
        assert!(c.contains(".some"));
    }

    #[test]
    fn generates_temp_for_non_local_match_scrutinee() {
        let program = parse(
            r#"
func main() {
    print(match maybe(false) {
        case Some(inner) { inner }
        case None { 0 }
    })
}

func maybe(flag bool) Option[int] {
    return if flag { Some(7) } else { None }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mlg_Option_int mallang_match_tmp_"));
        assert!(c.contains("= mlg_maybe(false);"));
        assert!(c.contains("printf(\"%lld\\n\", (long long)((("));
    }

    #[test]
    fn generates_c_for_match_statement() {
        let program = parse(
            r#"
func main() {
    match maybe(true) {
        case Some(inner) {
            print(inner)
        }
        case None {
            print(0)
        }
    }
}

func maybe(flag bool) Option[int] {
    return if flag { Some(7) } else { None }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mlg_Option_int mallang_match_tmp_"));
        assert!(c.contains("switch ((mallang_match_tmp_"));
        assert!(c.contains("case 1: {"));
        assert!(c.contains(".some"));
        assert!(c.contains("case 0: {"));
    }

    #[test]
    fn generates_c_for_struct_literals_and_field_access() {
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
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("typedef struct"));
        assert!(c.contains("const char * mlg_name;"));
        assert!(c.contains("int64_t mlg_age;"));
        assert!(c.contains(
            "mlg_struct_User mlg_user = (mlg_struct_User){ .mlg_name = \"kim\", .mlg_age = 30 };"
        ));
        assert!(c.contains("printf(\"%lld\\n\", (long long)((mlg_user).mlg_age));"));
    }

    #[test]
    fn generates_c_for_struct_methods() {
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
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("int64_t mlg_User_age(mlg_struct_User mlg_self);"));
        assert!(c.contains("return (mlg_self).mlg_age;"));
        assert!(c.contains("printf(\"%lld\\n\", (long long)(mlg_User_age(mlg_user)));"));
    }

    #[test]
    fn generates_c_for_field_assignment() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    mut user := User{age: 30}
    user.age = 31
    print(user.age)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("(mlg_user).mlg_age = 31;"));
        assert!(c.contains("printf(\"%lld\\n\", (long long)((mlg_user).mlg_age));"));
    }
}
