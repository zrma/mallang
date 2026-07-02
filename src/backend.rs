use std::{collections::HashMap, fmt};

use crate::{
    ast::{ArgMode, BinaryOp, ParamMode, Program, UnaryOp},
    ir::{
        lower, IrAdtConstructor, IrArg, IrExpr, IrExprKind, IrForInit, IrForPost, IrFunction,
        IrMatchArm, IrMatchBlockArm, IrMatchPattern, IrProgram, IrStmt, IrStmtKind,
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
        output.push_str("#include <stdio.h>\n");
        output.push_str("#include <stdlib.h>\n");
        output.push_str("#include <string.h>\n\n");
        output.push_str("static int64_t mallang_check_index(int64_t index, int64_t len) {\n");
        output.push_str("    if (index < 0 || index >= len) {\n");
        output.push_str(
            "        fprintf(stderr, \"mallang runtime error: array index out of bounds\\n\");\n",
        );
        output.push_str("        exit(1);\n");
        output.push_str("    }\n");
        output.push_str("    return index;\n");
        output.push_str("}\n\n");

        let defined_types = self.collect_defined_types();
        let mut emitted_types = Vec::new();
        for ty in &defined_types {
            self.emit_type_def(ty, &mut emitted_types, &mut Vec::new(), &mut output)?;
        }
        if !emitted_types.is_empty() {
            output.push('\n');
        }

        let mut emitted_drop_helpers = Vec::new();
        for ty in &defined_types {
            self.emit_drop_helper(ty, &mut emitted_drop_helpers, &mut Vec::new(), &mut output)?;
        }
        if !emitted_drop_helpers.is_empty() {
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
                .map(c_param_decl)
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
        let env = param_env(function);

        for stmt in &function.body {
            let line = self.emit_stmt_with_env(stmt, &env)?;
            push_indented_lines(&mut output, &line, 1);
        }

        if function.name == "main" {
            output.push_str("    return 0;\n");
        }

        output.push_str("}\n");
        Ok(output)
    }

    fn emit_stmt_with_env(
        &self,
        stmt: &IrStmt,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        self.emit_stmt_with_env_and_continue(stmt, env, None)
    }

    fn emit_stmt_with_env_and_continue(
        &self,
        stmt: &IrStmt,
        env: &HashMap<String, String>,
        continue_label: Option<&str>,
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
                    format!("{} = {};", c_assignment_target(name, env), code),
                ))
            }
            IrStmtKind::FieldAssign { base, field, expr } => {
                let base = self.emit_assignment_target_expr(base, env)?;
                let expr = self.emit_stmt_expr_with_env(expr, env)?;
                let mut prelude = base.prelude;
                prelude.extend(expr.prelude);
                Ok(finish_with_prelude(
                    prelude,
                    format!("({}).{} = {};", base.code, c_field(field), expr.code),
                ))
            }
            IrStmtKind::IndexAssign { base, index, expr } => {
                self.emit_index_assign_stmt(base, index, expr, env)
            }
            IrStmtKind::Return { expr } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_prelude(prelude, format!("return {};", code)))
            }
            IrStmtKind::If {
                condition,
                then_body,
                else_body,
            } => self.emit_if_stmt(condition, then_body, else_body, env, continue_label),
            IrStmtKind::For {
                init,
                condition,
                post,
                body,
                cleanup,
            } => self.emit_for_stmt(
                init.as_deref(),
                condition.as_deref(),
                post.as_deref(),
                body,
                cleanup,
                env,
            ),
            IrStmtKind::RangeFor {
                index_name,
                value_name,
                source,
                element_ty,
                body,
            } => self.emit_range_for_stmt(index_name, value_name, source, element_ty, body, env),
            IrStmtKind::Drop { expr } => self.emit_drop_stmt(expr, env),
            IrStmtKind::Break => Ok("break;".to_string()),
            IrStmtKind::Continue => Ok(continue_label
                .map(|label| format!("goto {label};"))
                .unwrap_or_else(|| "continue;".to_string())),
            IrStmtKind::Match { scrutinee, arms } => {
                self.emit_match_stmt(scrutinee, arms, env, continue_label)
            }
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
        continue_label: Option<&str>,
    ) -> Result<String, CompileError> {
        let mut output = String::new();
        let CExpr { prelude, code } = self.emit_stmt_expr_with_env(condition, env)?;
        for line in prelude {
            output.push_str(&line);
            output.push('\n');
        }
        output.push_str(&format!("if ({}) {{\n", c_condition(&code)));
        for stmt in then_body {
            let code = self.emit_stmt_with_env_and_continue(stmt, env, continue_label)?;
            push_indented_lines(&mut output, &code, 1);
        }
        if else_body.is_empty() {
            output.push('}');
            return Ok(output);
        }

        output.push_str("} else {\n");
        for stmt in else_body {
            let code = self.emit_stmt_with_env_and_continue(stmt, env, continue_label)?;
            push_indented_lines(&mut output, &code, 1);
        }
        output.push('}');
        Ok(output)
    }

    fn emit_for_stmt(
        &self,
        init: Option<&IrForInit>,
        condition: Option<&IrExpr>,
        post: Option<&IrForPost>,
        body: &[IrStmt],
        cleanup: &[IrStmt],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        if init.is_some() || post.is_some() || !cleanup.is_empty() {
            return self.emit_for_clause_stmt(init, condition, post, body, cleanup, env);
        }

        let (prelude, code) = if let Some(condition) = condition {
            let CExpr { prelude, code } = self.emit_stmt_expr_with_env(condition, env)?;
            (prelude, c_condition(&code))
        } else {
            (Vec::new(), "true".to_string())
        };
        let mut output = String::new();

        if prelude.is_empty() {
            output.push_str(&format!("while ({code}) {{\n"));
            for stmt in body {
                let code = self.emit_stmt_with_env(stmt, env)?;
                push_indented_lines(&mut output, &code, 1);
            }
            output.push('}');
            return Ok(output);
        }

        output.push_str("while (true) {\n");
        for line in prelude {
            push_indented_lines(&mut output, &line, 1);
        }
        push_indented_lines(&mut output, &format!("if (!({code})) {{"), 1);
        push_indented_lines(&mut output, "break;", 2);
        push_indented_lines(&mut output, "}", 1);
        for stmt in body {
            let code = self.emit_stmt_with_env(stmt, env)?;
            push_indented_lines(&mut output, &code, 1);
        }
        output.push('}');
        Ok(output)
    }

    fn emit_range_for_stmt(
        &self,
        index_name: &str,
        value_name: &str,
        source: &IrExpr,
        element_ty: &Type,
        body: &[IrStmt],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let (range_len, array_len) = match &source.ty {
            Type::Array { len, .. } => (len.to_string(), Some(*len)),
            Type::Slice(_) => (
                format!("({}).{}", range_source_temp_name(source), c_field("len")),
                None,
            ),
            _ => {
                return Err(CompileError::new(
                    "IR invariant violation: range source must be an array or slice",
                ));
            }
        };

        let CExpr { prelude, code } = self.emit_stmt_expr_with_env(source, env)?;
        let source_temp = range_source_temp_name(source);
        let index_ident = if is_blank_identifier(index_name) {
            range_index_temp_name(source)
        } else {
            c_ident(index_name)
        };
        let value_ident = if is_blank_identifier(value_name) {
            None
        } else {
            Some(c_ident(value_name))
        };
        let mut output = String::new();
        for line in prelude {
            output.push_str(&line);
            output.push('\n');
        }
        output.push_str(&format!("{} {source_temp} = {code};\n", source.ty.c_name()));
        output.push_str(&format!(
            "for (int64_t {index_ident} = 0; {index_ident} < {range_len}; {index_ident} = ({index_ident} + 1)) {{\n"
        ));
        if let Some(value_ident) = &value_ident {
            if array_len != Some(0) {
                push_indented_lines(
                    &mut output,
                    &format!(
                        "{} {value_ident} = ({source_temp}).{}[{index_ident}];",
                        element_ty.c_name(),
                        c_field("data")
                    ),
                    1,
                );
            } else {
                push_indented_lines(
                    &mut output,
                    &format!("{} {value_ident};", element_ty.c_name()),
                    1,
                );
            }
        }

        let mut body_env = env.clone();
        if !is_blank_identifier(index_name) {
            body_env.insert(index_name.to_string(), index_ident);
        }
        if let Some(value_ident) = value_ident {
            body_env.insert(value_name.to_string(), value_ident);
        }
        push_indented_lines(&mut output, "{", 1);
        for stmt in body {
            let code = self.emit_stmt_with_env(stmt, &body_env)?;
            push_indented_lines(&mut output, &code, 2);
        }
        push_indented_lines(&mut output, "}", 1);
        output.push('}');
        Ok(output)
    }

    fn emit_for_clause_stmt(
        &self,
        init: Option<&IrForInit>,
        condition: Option<&IrExpr>,
        post: Option<&IrForPost>,
        body: &[IrStmt],
        cleanup: &[IrStmt],
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let mut output = String::new();
        output.push_str("{\n");

        if let Some(init) = init {
            let (prelude, code) = self.emit_for_init(init, env)?;
            for line in prelude {
                push_indented_lines(&mut output, &line, 1);
            }
            push_indented_lines(&mut output, &format!("{code};"), 1);
        }

        let continue_label = post.map(for_post_label);
        output.push_str("    while (true) {\n");
        if let Some(condition) = condition {
            let CExpr { prelude, code } = self.emit_stmt_expr_with_env(condition, env)?;
            for line in prelude {
                push_indented_lines(&mut output, &line, 2);
            }
            push_indented_lines(
                &mut output,
                &format!("if (!({})) {{", c_condition(&code)),
                2,
            );
            push_indented_lines(&mut output, "break;", 3);
            push_indented_lines(&mut output, "}", 2);
        }

        push_indented_lines(&mut output, "{", 2);
        for stmt in body {
            let code =
                self.emit_stmt_with_env_and_continue(stmt, env, continue_label.as_deref())?;
            push_indented_lines(&mut output, &code, 3);
        }
        push_indented_lines(&mut output, "}", 2);

        if let Some(post) = post {
            let label = continue_label
                .as_deref()
                .expect("for post label must exist when post exists");
            push_indented_lines(&mut output, &format!("{label}: {{"), 2);
            let code = self.emit_for_post_stmt(post, env)?;
            push_indented_lines(&mut output, &code, 3);
            push_indented_lines(&mut output, "}", 2);
        }

        output.push_str("    }\n");
        for stmt in cleanup {
            let code = self.emit_stmt_with_env(stmt, env)?;
            push_indented_lines(&mut output, &code, 1);
        }
        output.push('}');
        Ok(output)
    }

    fn emit_for_init(
        &self,
        init: &IrForInit,
        env: &HashMap<String, String>,
    ) -> Result<(Vec<String>, String), CompileError> {
        match init {
            IrForInit::Let { name, ty, expr, .. } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok((
                    prelude,
                    format!("{} {} = {}", ty.c_name(), c_ident(name), code),
                ))
            }
        }
    }

    fn emit_for_post_stmt(
        &self,
        post: &IrForPost,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        match post {
            IrForPost::Assign { target, expr } => {
                let target = self.emit_assignment_target_expr(target, env)?;
                let expr = self.emit_stmt_expr_with_env(expr, env)?;
                let mut prelude = target.prelude;
                prelude.extend(expr.prelude);
                Ok(finish_with_prelude(
                    prelude,
                    format!("{} = {};", target.code, expr.code),
                ))
            }
        }
    }

    fn emit_assignment_target_expr(
        &self,
        target: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        match &target.kind {
            IrExprKind::Var(name) => Ok(CExpr::simple(c_assignment_target(name, env))),
            IrExprKind::FieldAccess { base, field } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(base, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({}).{}", code, c_field(field)),
                })
            }
            IrExprKind::Index { base, index } => match &base.ty {
                Type::Array { len, .. } => {
                    let base = self.emit_stmt_expr_with_env(base, env)?;
                    let index = self.emit_stmt_expr_with_env(index, env)?;
                    let mut prelude = base.prelude;
                    prelude.extend(index.prelude);
                    Ok(CExpr {
                        prelude,
                        code: format!(
                            "({}).mlg_data[mallang_check_index({}, {len})]",
                            base.code, index.code
                        ),
                    })
                }
                Type::Slice(_) => {
                    let base = self.emit_stmt_expr_with_env(base, env)?;
                    let index = self.emit_stmt_expr_with_env(index, env)?;
                    let index_temp = index_value_temp_name(target);
                    let mut prelude = base.prelude;
                    prelude.extend(index.prelude);
                    prelude.push(format!("int64_t {index_temp} = {};", index.code));
                    prelude.push(format!(
                            "if ({index_temp} < 0 || {index_temp} >= ({}).{}) {{\n    fprintf(stderr, \"mallang runtime error: slice index out of bounds\\n\");\n    exit(1);\n}}",
                            base.code,
                            c_field("len")
                        ));
                    Ok(CExpr {
                        prelude,
                        code: format!("({}).{}[{index_temp}]", base.code, c_field("data")),
                    })
                }
                _ => Err(CompileError::new(
                    "IR invariant violation: index assignment base must be an array or slice",
                )),
            },
            _ => Err(CompileError::new(
                "IR invariant violation: invalid assignment target",
            )),
        }
    }

    fn emit_index_assign_stmt(
        &self,
        base: &IrExpr,
        index: &IrExpr,
        expr: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let IrExprKind::Var(name) = &base.kind else {
            return Err(CompileError::new(
                "IR invariant violation: index assignment base must be a variable",
            ));
        };

        let index_temp = index_assign_value_temp_name(index);
        let index = self.emit_stmt_expr_with_env(index, env)?;
        let value = self.emit_stmt_expr_with_env(expr, env)?;

        let mut prelude = index.prelude;
        prelude.push(format!("int64_t {index_temp} = {};", index.code));
        let target = c_assignment_target(name, env);
        match &base.ty {
            Type::Array { len, .. } => {
                prelude.push(format!(
                    "if ({index_temp} < 0 || {index_temp} >= {len}) {{\n    fprintf(stderr, \"mallang runtime error: array index out of bounds\\n\");\n    exit(1);\n}}"
                ));
            }
            Type::Slice(_) => {
                prelude.push(format!(
                    "if ({index_temp} < 0 || {index_temp} >= ({target}).{}) {{\n    fprintf(stderr, \"mallang runtime error: slice index out of bounds\\n\");\n    exit(1);\n}}",
                    c_field("len")
                ));
            }
            _ => {
                return Err(CompileError::new(
                    "IR invariant violation: index assignment base must be an array or slice",
                ));
            }
        }
        prelude.extend(value.prelude);
        let data_field = c_field("data");

        Ok(finish_with_prelude(
            prelude,
            format!("({target}).{data_field}[{index_temp}] = {};", value.code),
        ))
    }

    fn emit_match_stmt(
        &self,
        scrutinee: &IrExpr,
        arms: &[IrMatchBlockArm],
        env: &HashMap<String, String>,
        continue_label: Option<&str>,
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
            Type::Option(_) => {
                self.emit_option_match_stmt(&scrutinee_code, arms, env, continue_label, output)
            }
            Type::Result(_, _) => {
                self.emit_result_match_stmt(&scrutinee_code, arms, env, continue_label, output)
            }
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
        continue_label: Option<&str>,
        mut output: String,
    ) -> Result<String, CompileError> {
        output.push_str(&format!("switch (({scrutinee}).tag) {{\n"));
        for arm in arms {
            match &arm.pattern {
                IrMatchPattern::Some(binding) => {
                    output.push_str("case 1: {\n");
                    let mut arm_env = env.clone();
                    arm_env.insert(binding.clone(), format!("({scrutinee}).some"));
                    self.emit_match_stmt_body(&arm.body, &arm_env, continue_label, &mut output)?;
                    output.push_str("    break;\n");
                    output.push_str("}\n");
                }
                IrMatchPattern::None => {
                    output.push_str("case 0: {\n");
                    self.emit_match_stmt_body(&arm.body, env, continue_label, &mut output)?;
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
        continue_label: Option<&str>,
        mut output: String,
    ) -> Result<String, CompileError> {
        output.push_str(&format!("switch (({scrutinee}).tag) {{\n"));
        for arm in arms {
            match &arm.pattern {
                IrMatchPattern::Ok(binding) => {
                    output.push_str("case 0: {\n");
                    let mut arm_env = env.clone();
                    arm_env.insert(binding.clone(), format!("({scrutinee}).ok"));
                    self.emit_match_stmt_body(&arm.body, &arm_env, continue_label, &mut output)?;
                    output.push_str("    break;\n");
                    output.push_str("}\n");
                }
                IrMatchPattern::Err(binding) => {
                    output.push_str("case 1: {\n");
                    let mut arm_env = env.clone();
                    arm_env.insert(binding.clone(), format!("({scrutinee}).err"));
                    self.emit_match_stmt_body(&arm.body, &arm_env, continue_label, &mut output)?;
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
        continue_label: Option<&str>,
        output: &mut String,
    ) -> Result<(), CompileError> {
        for stmt in body {
            let code = self.emit_stmt_with_env_and_continue(stmt, env, continue_label)?;
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
            Type::Array { .. } => Err(CompileError::new(
                "fixed-size arrays are not supported by the C backend yet",
            )),
            Type::Slice(_) => Err(CompileError::new(
                "IR invariant violation: cannot print slice value before slice printability is defined",
            )),
            Type::Option(_) | Type::Result(_, _) | Type::Struct(_) => {
                self.emit_print_composite(arg, prelude, code)
            }
        }
    }

    fn emit_print_composite(
        &self,
        arg: &IrExpr,
        mut prelude: Vec<String>,
        code: String,
    ) -> Result<String, CompileError> {
        let temp = print_temp_name(arg);
        prelude.push(format!("{} {temp} = {code};", arg.ty.c_name()));

        let mut body = String::new();
        self.push_print_value_fragment(&arg.ty, &temp, &mut body, 0)?;
        push_indented_lines(&mut body, "printf(\"\\n\");", 0);

        Ok(finish_with_prelude(prelude, body))
    }

    fn push_print_value_fragment(
        &self,
        ty: &Type,
        code: &str,
        output: &mut String,
        level: usize,
    ) -> Result<(), CompileError> {
        match ty {
            Type::Int => {
                push_indented_lines(
                    output,
                    &format!("printf(\"%lld\", (long long)({code}));"),
                    level,
                );
                Ok(())
            }
            Type::Bool => {
                push_indented_lines(
                    output,
                    &format!("printf(\"%s\", ({code}) ? \"true\" : \"false\");"),
                    level,
                );
                Ok(())
            }
            Type::String => {
                push_indented_lines(output, &format!("printf(\"%s\", {code});"), level);
                Ok(())
            }
            Type::Option(inner) => {
                push_indented_lines(output, &format!("if (({code}).tag == 1) {{"), level);
                push_indented_lines(output, "printf(\"Some(\");", level + 1);
                self.push_print_value_fragment(
                    inner,
                    &format!("({code}).some"),
                    output,
                    level + 1,
                )?;
                push_indented_lines(output, "printf(\")\");", level + 1);
                push_indented_lines(output, "} else {", level);
                push_indented_lines(output, "printf(\"None\");", level + 1);
                push_indented_lines(output, "}", level);
                Ok(())
            }
            Type::Result(ok, err) => {
                push_indented_lines(output, &format!("if (({code}).tag == 0) {{"), level);
                push_indented_lines(output, "printf(\"Ok(\");", level + 1);
                self.push_print_value_fragment(ok, &format!("({code}).ok"), output, level + 1)?;
                push_indented_lines(output, "printf(\")\");", level + 1);
                push_indented_lines(output, "} else {", level);
                push_indented_lines(output, "printf(\"Err(\");", level + 1);
                self.push_print_value_fragment(err, &format!("({code}).err"), output, level + 1)?;
                push_indented_lines(output, "printf(\")\");", level + 1);
                push_indented_lines(output, "}", level);
                Ok(())
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                push_indented_lines(output, &format!("printf(\"{}{{\");", name), level);
                for (index, field) in struct_def.fields.iter().enumerate() {
                    if index > 0 {
                        push_indented_lines(output, "printf(\", \");", level);
                    }
                    push_indented_lines(output, &format!("printf(\"{}: \");", field.name), level);
                    self.push_print_value_fragment(
                        &field.ty,
                        &format!("({code}).{}", c_field(&field.name)),
                        output,
                        level,
                    )?;
                }
                push_indented_lines(output, "printf(\"}\");", level);
                Ok(())
            }
            Type::Unit => Err(CompileError::new(
                "IR invariant violation: cannot print unit",
            )),
            Type::Array { .. } => Err(CompileError::new(
                "fixed-size arrays are not supported by the C backend yet",
            )),
            Type::Slice(_) => Err(CompileError::new(
                "IR invariant violation: cannot print slice value before slice printability is defined",
            )),
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
                then_cleanup,
                else_branch,
                else_cleanup,
            } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(condition, env)?;
                let then_expr = self.emit_stmt_expr_with_env(then_branch, env)?;
                let else_expr = self.emit_stmt_expr_with_env(else_branch, env)?;
                if then_expr.prelude.is_empty()
                    && else_expr.prelude.is_empty()
                    && then_cleanup.is_empty()
                    && else_cleanup.is_empty()
                {
                    return Ok(CExpr {
                        prelude,
                        code: format!("(({}) ? ({}) : ({}))", code, then_expr.code, else_expr.code),
                    });
                }

                let temp = if_expr_temp_name(expr);
                let mut prelude = prelude;
                let then_cleanup = self.emit_cleanup_stmts(then_cleanup, env)?;
                let else_cleanup = self.emit_cleanup_stmts(else_cleanup, env)?;
                prelude.push(format!("{} {temp};", expr.ty.c_name()));
                prelude.push(if_expr_temp_block(
                    &code,
                    &temp,
                    then_expr,
                    then_cleanup,
                    else_expr,
                    else_cleanup,
                ));
                Ok(CExpr {
                    prelude,
                    code: temp,
                })
            }
            IrExprKind::AdtConstructor {
                constructor,
                payload,
            } => {
                self.emit_adt_constructor_stmt_expr(&expr.ty, *constructor, payload.as_deref(), env)
            }
            IrExprKind::Match { scrutinee, arms } => {
                self.emit_match_stmt_expr(expr, scrutinee, arms, env)
            }
            IrExprKind::StructLiteral { type_name, fields } => {
                self.emit_struct_literal_stmt_expr(type_name, fields, env)
            }
            IrExprKind::ArrayLiteral { elements } => {
                self.emit_array_literal_stmt_expr(expr, elements, env)
            }
            IrExprKind::FieldAccess { base, field } => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(base, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({}).{}", code, c_field(field)),
                })
            }
            IrExprKind::Index { base, index } => self.emit_index_stmt_expr(expr, base, index, env),
            IrExprKind::ArrayLen { array } => self.emit_array_len_stmt_expr(array, env),
            IrExprKind::SliceAppend { slice, item } => {
                self.emit_slice_append_stmt_expr(expr, slice, item, env)
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
                    let emitted = self.emit_call_arg_stmt_expr(arg, env)?;
                    prelude.extend(emitted.prelude);
                    arg_codes.push(emitted.code);
                }
                Ok(CExpr {
                    prelude,
                    code: format!("{}({})", c_ident(callee), arg_codes.join(", ")),
                })
            }
            IrExprKind::Unary { op, expr: inner } => {
                self.emit_unary_stmt_expr(expr, *op, inner, env)
            }
            IrExprKind::Binary { op, left, right } => {
                let operand_ty = left.ty.clone();
                self.emit_binary_stmt_expr(expr, *op, left, right, &operand_ty, env)
            }
        }
    }

    fn emit_cleanup_stmts(
        &self,
        cleanup: &[IrStmt],
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>, CompileError> {
        cleanup
            .iter()
            .map(|stmt| self.emit_stmt_with_env(stmt, env))
            .collect()
    }

    fn emit_unary_stmt_expr(
        &self,
        expr: &IrExpr,
        op: UnaryOp,
        inner: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let CExpr { mut prelude, code } = self.emit_stmt_expr_with_env(inner, env)?;

        if op == UnaryOp::Negate && expr.ty == Type::Int {
            let operand_temp = checked_unary_operand_temp_name(expr);
            let result_temp = checked_unary_result_temp_name(expr);
            prelude.push(format!("int64_t {operand_temp} = {code};"));
            prelude.push(format!("int64_t {result_temp};"));
            prelude.push(format!(
                "if (__builtin_sub_overflow((int64_t)0, {operand_temp}, &{result_temp})) {{\n    fprintf(stderr, \"mallang runtime error: integer overflow\\n\");\n    exit(1);\n}}"
            ));
            return Ok(CExpr {
                prelude,
                code: result_temp,
            });
        }

        Ok(CExpr {
            prelude,
            code: format!("({}{})", op.c_operator(), code),
        })
    }

    fn emit_binary_stmt_expr(
        &self,
        expr: &IrExpr,
        op: BinaryOp,
        left: &IrExpr,
        right: &IrExpr,
        operand_ty: &Type,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let left = self.emit_stmt_expr_with_env(left, env)?;
        let right = self.emit_stmt_expr_with_env(right, env)?;
        let mut prelude = left.prelude;
        prelude.extend(right.prelude);

        if let Some(builtin) = checked_int_binary_builtin(op).filter(|_| operand_ty == &Type::Int) {
            let left_temp = checked_binary_left_temp_name(expr);
            let right_temp = checked_binary_right_temp_name(expr);
            let result_temp = checked_binary_result_temp_name(expr);
            prelude.push(format!("int64_t {left_temp} = {};", left.code));
            prelude.push(format!("int64_t {right_temp} = {};", right.code));
            prelude.push(format!("int64_t {result_temp};"));
            prelude.push(format!(
                "if ({builtin}({left_temp}, {right_temp}, &{result_temp})) {{\n    fprintf(stderr, \"mallang runtime error: integer overflow\\n\");\n    exit(1);\n}}"
            ));
            return Ok(CExpr {
                prelude,
                code: result_temp,
            });
        }

        if matches!(op, BinaryOp::Divide | BinaryOp::Remainder) && operand_ty == &Type::Int {
            let dividend_temp = dividend_temp_name(expr);
            let divisor_temp = divisor_temp_name(expr);
            prelude.push(format!("int64_t {dividend_temp} = {};", left.code));
            prelude.push(format!("int64_t {divisor_temp} = {};", right.code));
            prelude.push(format!(
                "if ({divisor_temp} == 0) {{\n    fprintf(stderr, \"mallang runtime error: division by zero\\n\");\n    exit(1);\n}}"
            ));
            prelude.push(format!(
                "if ({dividend_temp} == INT64_MIN && {divisor_temp} == -1) {{\n    fprintf(stderr, \"mallang runtime error: integer overflow\\n\");\n    exit(1);\n}}"
            ));
            return Ok(CExpr {
                prelude,
                code: c_binary_expr(op, &expr.ty, operand_ty, dividend_temp, divisor_temp),
            });
        }

        Ok(CExpr {
            prelude,
            code: c_binary_expr(op, &expr.ty, operand_ty, left.code, right.code),
        })
    }

    fn emit_index_stmt_expr(
        &self,
        expr: &IrExpr,
        base: &IrExpr,
        index: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        match &base.ty {
            Type::Array { len, .. } => {
                let source_ty = base.ty.c_name();
                let base = self.emit_stmt_expr_with_env(base, env)?;
                let index = self.emit_stmt_expr_with_env(index, env)?;
                let source_temp = index_source_temp_name(expr);
                let index_temp = index_value_temp_name(expr);

                let mut prelude = base.prelude;
                prelude.push(format!("{source_ty} {source_temp} = {};", base.code));
                prelude.extend(index.prelude);
                prelude.push(format!("int64_t {index_temp} = {};", index.code));
                prelude.push(format!(
                    "if ({index_temp} < 0 || {index_temp} >= {len}) {{\n    fprintf(stderr, \"mallang runtime error: array index out of bounds\\n\");\n    exit(1);\n}}"
                ));

                Ok(CExpr {
                    prelude,
                    code: format!("({source_temp}).mlg_data[{index_temp}]"),
                })
            }
            Type::Slice(_) => {
                let source_ty = base.ty.c_name();
                let base = self.emit_stmt_expr_with_env(base, env)?;
                let index = self.emit_stmt_expr_with_env(index, env)?;
                let source_temp = index_source_temp_name(expr);
                let index_temp = index_value_temp_name(expr);

                let mut prelude = base.prelude;
                prelude.push(format!("{source_ty} {source_temp} = {};", base.code));
                prelude.extend(index.prelude);
                prelude.push(format!("int64_t {index_temp} = {};", index.code));
                prelude.push(format!(
                    "if ({index_temp} < 0 || {index_temp} >= ({source_temp}).{}) {{\n    fprintf(stderr, \"mallang runtime error: slice index out of bounds\\n\");\n    exit(1);\n}}",
                    c_field("len")
                ));

                Ok(CExpr {
                    prelude,
                    code: format!("({source_temp}).{}[{index_temp}]", c_field("data")),
                })
            }
            _ => Err(CompileError::new(
                "IR invariant violation: index base must be an array or slice",
            )),
        }
    }

    fn emit_array_len_stmt_expr(
        &self,
        array: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        match &array.ty {
            Type::Array { len, .. } => {
                let CExpr { mut prelude, code } = self.emit_stmt_expr_with_env(array, env)?;
                prelude.push(format!("(void)({code});"));

                Ok(CExpr {
                    prelude,
                    code: len.to_string(),
                })
            }
            Type::Slice(_) => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(array, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({code}).{}", c_field("len")),
                })
            }
            _ => Err(CompileError::new(
                "IR invariant violation: len source must be an array or slice",
            )),
        }
    }

    fn emit_slice_append_stmt_expr(
        &self,
        expr: &IrExpr,
        slice: &IrExpr,
        item: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let Type::Slice(element) = &expr.ty else {
            return Err(CompileError::new(
                "IR invariant violation: append result must be a slice",
            ));
        };
        if slice.ty != expr.ty || item.ty != **element {
            return Err(CompileError::new(
                "IR invariant violation: append operand type mismatch",
            ));
        }

        let slice = self.emit_stmt_expr_with_env(slice, env)?;
        let item = self.emit_stmt_expr_with_env(item, env)?;
        let temp = slice_append_temp_name(expr);
        let new_len = format!("{temp}_new_len");
        let new_cap = format!("{temp}_new_cap");
        let data_temp = format!("{temp}_data");
        let data_field = c_field("data");
        let len_field = c_field("len");
        let cap_field = c_field("cap");

        let mut prelude = slice.prelude;
        prelude.push(format!("{} {temp} = {};", expr.ty.c_name(), slice.code));
        prelude.extend(item.prelude);
        prelude.push(format!(
            "if ({temp}.{len_field} == INT64_MAX) {{\n    fprintf(stderr, \"mallang runtime error: slice length overflow\\n\");\n    exit(1);\n}}"
        ));
        prelude.push(format!("int64_t {new_len} = {temp}.{len_field} + 1;"));
        prelude.push(format!(
            "if ({temp}.{cap_field} < {new_len}) {{\n    int64_t {new_cap} = ({temp}.{cap_field} == 0) ? 1 : {temp}.{cap_field};\n    while ({new_cap} < {new_len}) {{\n        if ({new_cap} > INT64_MAX / 2) {{\n            {new_cap} = {new_len};\n            break;\n        }}\n        {new_cap} = {new_cap} * 2;\n    }}\n    if ((uint64_t){new_cap} > UINT64_MAX / sizeof({element_ty})) {{\n        fprintf(stderr, \"mallang runtime error: slice allocation size overflow\\n\");\n        exit(1);\n    }}\n    void *{data_temp} = realloc({temp}.{data_field}, sizeof({element_ty}) * (uint64_t){new_cap});\n    if ({data_temp} == NULL) {{\n        fprintf(stderr, \"mallang runtime error: slice allocation failed\\n\");\n        exit(1);\n    }}\n    {temp}.{data_field} = {data_temp};\n    {temp}.{cap_field} = {new_cap};\n}}",
            element_ty = element.c_name()
        ));
        prelude.push(format!(
            "{temp}.{data_field}[{temp}.{len_field}] = {};",
            item.code
        ));
        prelude.push(format!("{temp}.{len_field} = {new_len};"));

        Ok(CExpr {
            prelude,
            code: temp,
        })
    }

    fn emit_call_arg_stmt_expr(
        &self,
        arg: &IrArg,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let CExpr { prelude, code } = match arg.mode {
            ArgMode::Owned => self.emit_stmt_expr_with_env(&arg.expr, env)?,
            ArgMode::Con | ArgMode::Mut => self.emit_borrow_lvalue_expr(&arg.expr, env)?,
        };
        Ok(CExpr {
            prelude,
            code: c_arg_code(arg.mode, code),
        })
    }

    fn emit_borrow_lvalue_expr(
        &self,
        expr: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        match &expr.kind {
            IrExprKind::Var(name) => Ok(CExpr::simple(c_assignment_target(name, env))),
            IrExprKind::FieldAccess { base, field } => {
                let CExpr { prelude, code } = self.emit_borrow_lvalue_expr(base, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({}).{}", code, c_field(field)),
                })
            }
            IrExprKind::Index { base, index } => match &base.ty {
                Type::Array { len, .. } => {
                    let base = self.emit_borrow_lvalue_expr(base, env)?;
                    let index = self.emit_stmt_expr_with_env(index, env)?;
                    let mut prelude = base.prelude;
                    prelude.extend(index.prelude);
                    Ok(CExpr {
                        prelude,
                        code: format!(
                            "({}).mlg_data[mallang_check_index({}, {len})]",
                            base.code, index.code
                        ),
                    })
                }
                Type::Slice(_) => {
                    let base = self.emit_borrow_lvalue_expr(base, env)?;
                    let index = self.emit_stmt_expr_with_env(index, env)?;
                    let index_temp = index_value_temp_name(expr);
                    let mut prelude = base.prelude;
                    prelude.extend(index.prelude);
                    prelude.push(format!("int64_t {index_temp} = {};", index.code));
                    prelude.push(format!(
                            "if ({index_temp} < 0 || {index_temp} >= ({}).{}) {{\n    fprintf(stderr, \"mallang runtime error: slice index out of bounds\\n\");\n    exit(1);\n}}",
                            base.code,
                            c_field("len")
                        ));
                    Ok(CExpr {
                        prelude,
                        code: format!("({}).{}[{index_temp}]", base.code, c_field("data")),
                    })
                }
                _ => Err(CompileError::new(
                    "IR invariant violation: borrow index base must be an array or slice",
                )),
            },
            _ => Err(CompileError::new(
                "IR invariant violation: invalid borrow argument expression",
            )),
        }
    }

    fn emit_drop_stmt(
        &self,
        expr: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        if !expr.ty.needs_cleanup() {
            return Err(CompileError::new(format!(
                "IR invariant violation: drop requested for non-cleanup type `{}`",
                expr.ty.source_name()
            )));
        }

        let CExpr { prelude, code } = self.emit_borrow_lvalue_expr(expr, env)?;
        Ok(finish_with_prelude(
            prelude,
            format!("{}(&({code}));", drop_fn_name(&expr.ty)),
        ))
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

    fn emit_array_literal_stmt_expr(
        &self,
        expr: &IrExpr,
        elements: &[IrExpr],
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let ty = &expr.ty;
        match ty {
            Type::Array { len, .. } => {
                if elements.len() != *len {
                    return Err(CompileError::new(
                        "IR invariant violation: array literal length mismatch",
                    ));
                }

                let mut prelude = Vec::new();
                let mut element_codes = Vec::new();
                for element in elements {
                    let emitted = self.emit_stmt_expr_with_env(element, env)?;
                    prelude.extend(emitted.prelude);
                    element_codes.push(emitted.code);
                }

                let code = if *len == 0 {
                    format!("({}){{ .{} = 0 }}", ty.c_name(), c_field("empty"))
                } else {
                    format!(
                        "({}){{ .{} = {{ {} }} }}",
                        ty.c_name(),
                        c_field("data"),
                        element_codes.join(", ")
                    )
                };

                Ok(CExpr { prelude, code })
            }
            Type::Slice(element) => {
                let temp = slice_literal_temp_name(expr);
                let mut prelude = vec![format!("{} {temp};", ty.c_name())];
                if elements.is_empty() {
                    prelude.push(format!("{temp}.{} = NULL;", c_field("data")));
                    prelude.push(format!("{temp}.{} = 0;", c_field("len")));
                    prelude.push(format!("{temp}.{} = 0;", c_field("cap")));
                    return Ok(CExpr {
                        prelude,
                        code: temp,
                    });
                }

                prelude.push(format!(
                    "{temp}.{} = malloc(sizeof({}) * {});",
                    c_field("data"),
                    element.c_name(),
                    elements.len()
                ));
                prelude.push(format!(
                    "if ({temp}.{} == NULL) {{\n    fprintf(stderr, \"mallang runtime error: slice allocation failed\\n\");\n    exit(1);\n}}",
                    c_field("data")
                ));
                prelude.push(format!("{temp}.{} = {};", c_field("len"), elements.len()));
                prelude.push(format!("{temp}.{} = {};", c_field("cap"), elements.len()));

                for (index, element) in elements.iter().enumerate() {
                    let emitted = self.emit_stmt_expr_with_env(element, env)?;
                    prelude.extend(emitted.prelude);
                    prelude.push(format!(
                        "{temp}.{}[{index}] = {};",
                        c_field("data"),
                        emitted.code
                    ));
                }

                Ok(CExpr {
                    prelude,
                    code: temp,
                })
            }
            _ => Err(CompileError::new(
                "IR invariant violation: array literal without array or slice type",
            )),
        }
    }

    fn emit_match_stmt_expr(
        &self,
        expr: &IrExpr,
        scrutinee: &IrExpr,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let (prelude, scrutinee_code) = if let IrExprKind::Var(name) = &scrutinee.kind {
            (
                Vec::new(),
                env.get(name).cloned().unwrap_or_else(|| c_ident(name)),
            )
        } else {
            let CExpr { mut prelude, code } = self.emit_stmt_expr_with_env(scrutinee, env)?;
            let temp = match_scrutinee_temp_name(scrutinee);
            prelude.push(format!("{} {temp} = {code};", scrutinee.ty.c_name()));
            (prelude, temp)
        };

        match &scrutinee.ty {
            Type::Option(_) => {
                self.emit_option_match_stmt_expr(expr, &scrutinee_code, arms, env, prelude)
            }
            Type::Result(_, _) => {
                self.emit_result_match_stmt_expr(expr, &scrutinee_code, arms, env, prelude)
            }
            _ => Err(CompileError::new(
                "IR invariant violation: match on non-ADT value",
            )),
        }
    }

    fn emit_option_match_stmt_expr(
        &self,
        expr: &IrExpr,
        scrutinee: &str,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
        mut prelude: Vec<String>,
    ) -> Result<CExpr, CompileError> {
        let some_arm = arms
            .iter()
            .find_map(|arm| match &arm.pattern {
                IrMatchPattern::Some(binding) => Some((binding, arm)),
                _ => None,
            })
            .ok_or_else(|| CompileError::new("IR invariant violation: missing Some arm"))?;
        let none_arm = arms
            .iter()
            .find(|arm| matches!(arm.pattern, IrMatchPattern::None))
            .ok_or_else(|| CompileError::new("IR invariant violation: missing None arm"))?;

        let mut some_env = env.clone();
        some_env.insert(some_arm.0.clone(), format!("({scrutinee}).some"));
        let some_expr = self.emit_stmt_expr_with_env(&some_arm.1.expr, &some_env)?;
        let none_expr = self.emit_stmt_expr_with_env(&none_arm.expr, env)?;

        if some_expr.prelude.is_empty()
            && none_expr.prelude.is_empty()
            && some_arm.1.cleanup.is_empty()
            && none_arm.cleanup.is_empty()
        {
            return Ok(CExpr {
                prelude,
                code: format!(
                    "((({scrutinee}).tag == 1) ? ({}) : ({}))",
                    some_expr.code, none_expr.code
                ),
            });
        }

        let temp = match_expr_temp_name(expr);
        let some_cleanup = self.emit_cleanup_stmts(&some_arm.1.cleanup, &some_env)?;
        let none_cleanup = self.emit_cleanup_stmts(&none_arm.cleanup, env)?;
        prelude.push(format!("{} {temp};", expr.ty.c_name()));
        prelude.push(if_expr_temp_block(
            &format!("({scrutinee}).tag == 1"),
            &temp,
            some_expr,
            some_cleanup,
            none_expr,
            none_cleanup,
        ));
        Ok(CExpr {
            prelude,
            code: temp,
        })
    }

    fn emit_result_match_stmt_expr(
        &self,
        expr: &IrExpr,
        scrutinee: &str,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
        mut prelude: Vec<String>,
    ) -> Result<CExpr, CompileError> {
        let ok_arm = arms
            .iter()
            .find_map(|arm| match &arm.pattern {
                IrMatchPattern::Ok(binding) => Some((binding, arm)),
                _ => None,
            })
            .ok_or_else(|| CompileError::new("IR invariant violation: missing Ok arm"))?;
        let err_arm = arms
            .iter()
            .find_map(|arm| match &arm.pattern {
                IrMatchPattern::Err(binding) => Some((binding, arm)),
                _ => None,
            })
            .ok_or_else(|| CompileError::new("IR invariant violation: missing Err arm"))?;

        let mut ok_env = env.clone();
        ok_env.insert(ok_arm.0.clone(), format!("({scrutinee}).ok"));
        let mut err_env = env.clone();
        err_env.insert(err_arm.0.clone(), format!("({scrutinee}).err"));
        let ok_expr = self.emit_stmt_expr_with_env(&ok_arm.1.expr, &ok_env)?;
        let err_expr = self.emit_stmt_expr_with_env(&err_arm.1.expr, &err_env)?;

        if ok_expr.prelude.is_empty()
            && err_expr.prelude.is_empty()
            && ok_arm.1.cleanup.is_empty()
            && err_arm.1.cleanup.is_empty()
        {
            return Ok(CExpr {
                prelude,
                code: format!(
                    "((({scrutinee}).tag == 0) ? ({}) : ({}))",
                    ok_expr.code, err_expr.code
                ),
            });
        }

        let temp = match_expr_temp_name(expr);
        let ok_cleanup = self.emit_cleanup_stmts(&ok_arm.1.cleanup, &ok_env)?;
        let err_cleanup = self.emit_cleanup_stmts(&err_arm.1.cleanup, &err_env)?;
        prelude.push(format!("{} {temp};", expr.ty.c_name()));
        prelude.push(if_expr_temp_block(
            &format!("({scrutinee}).tag == 0"),
            &temp,
            ok_expr,
            ok_cleanup,
            err_expr,
            err_cleanup,
        ));
        Ok(CExpr {
            prelude,
            code: temp,
        })
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
            Type::Array { .. } => {
                output.push_str(&self.typedef_for_array(ty)?);
                output.push('\n');
            }
            Type::Slice(element) => {
                self.emit_type_def(element, emitted, visiting, output)?;
                output.push_str(&self.typedef_for_slice(ty)?);
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
            IrStmtKind::Break | IrStmtKind::Continue => {}
            IrStmtKind::FieldAssign { base, expr, .. } => {
                self.collect_expr_types(base, types);
                self.collect_expr_types(expr, types);
            }
            IrStmtKind::IndexAssign { base, index, expr } => {
                self.collect_expr_types(base, types);
                self.collect_expr_types(index, types);
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
            IrStmtKind::For {
                init,
                condition,
                post,
                body,
                cleanup,
            } => {
                if let Some(init) = init.as_deref() {
                    self.collect_for_init_types(init, types);
                }
                if let Some(condition) = condition.as_deref() {
                    self.collect_expr_types(condition, types);
                }
                if let Some(post) = post.as_deref() {
                    self.collect_for_post_types(post, types);
                }
                for stmt in body {
                    self.collect_stmt_types(stmt, types);
                }
                for stmt in cleanup {
                    self.collect_stmt_types(stmt, types);
                }
            }
            IrStmtKind::RangeFor {
                source,
                element_ty,
                body,
                ..
            } => {
                self.collect_expr_types(source, types);
                collect_type(element_ty, types);
                for stmt in body {
                    self.collect_stmt_types(stmt, types);
                }
            }
            IrStmtKind::Drop { expr } => self.collect_expr_types(expr, types),
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

    fn collect_for_init_types(&self, init: &IrForInit, types: &mut Vec<Type>) {
        match init {
            IrForInit::Let { ty, expr, .. } => {
                collect_type(ty, types);
                self.collect_expr_types(expr, types);
            }
        }
    }

    fn collect_for_post_types(&self, post: &IrForPost, types: &mut Vec<Type>) {
        match post {
            IrForPost::Assign { target, expr } => {
                self.collect_expr_types(target, types);
                self.collect_expr_types(expr, types);
            }
        }
    }

    fn collect_expr_types(&self, expr: &IrExpr, types: &mut Vec<Type>) {
        collect_type(&expr.ty, types);
        match &expr.kind {
            IrExprKind::If {
                condition,
                then_branch,
                then_cleanup,
                else_branch,
                else_cleanup,
            } => {
                self.collect_expr_types(condition, types);
                self.collect_expr_types(then_branch, types);
                for stmt in then_cleanup {
                    self.collect_stmt_types(stmt, types);
                }
                self.collect_expr_types(else_branch, types);
                for stmt in else_cleanup {
                    self.collect_stmt_types(stmt, types);
                }
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
                    for stmt in &arm.cleanup {
                        self.collect_stmt_types(stmt, types);
                    }
                }
            }
            IrExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.collect_expr_types(&field.expr, types);
                }
            }
            IrExprKind::ArrayLiteral { elements } => {
                for element in elements {
                    self.collect_expr_types(element, types);
                }
            }
            IrExprKind::FieldAccess { base, .. } => self.collect_expr_types(base, types),
            IrExprKind::Index { base, index } => {
                self.collect_expr_types(base, types);
                self.collect_expr_types(index, types);
            }
            IrExprKind::ArrayLen { array } => self.collect_expr_types(array, types),
            IrExprKind::SliceAppend { slice, item } => {
                self.collect_expr_types(slice, types);
                self.collect_expr_types(item, types);
            }
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

    fn typedef_for_array(&self, ty: &Type) -> Result<String, CompileError> {
        let Type::Array { len, element } = ty else {
            return Err(CompileError::new("internal error: expected array type"));
        };

        let mut output = String::new();
        output.push_str("typedef struct {\n");
        if *len == 0 {
            output.push_str("    char ");
            output.push_str(&c_field("empty"));
            output.push_str(";\n");
        } else {
            output.push_str("    ");
            output.push_str(&element.c_name());
            output.push(' ');
            output.push_str(&c_field("data"));
            output.push('[');
            output.push_str(&len.to_string());
            output.push_str("];\n");
        }
        output.push_str("} ");
        output.push_str(&ty.c_name());
        output.push_str(";\n");
        Ok(output)
    }

    fn typedef_for_slice(&self, ty: &Type) -> Result<String, CompileError> {
        let Type::Slice(element) = ty else {
            return Err(CompileError::new("internal error: expected slice type"));
        };

        Ok(format!(
            "typedef struct {{\n    {} *{};\n    int64_t {};\n    int64_t {};\n}} {};\n",
            element.c_name(),
            c_field("data"),
            c_field("len"),
            c_field("cap"),
            ty.c_name()
        ))
    }

    fn emit_drop_helper(
        &self,
        ty: &Type,
        emitted: &mut Vec<Type>,
        visiting: &mut Vec<Type>,
        output: &mut String,
    ) -> Result<(), CompileError> {
        if emitted.contains(ty) || !ty.needs_cleanup() {
            return Ok(());
        }
        if visiting.contains(ty) {
            return Err(CompileError::new(format!(
                "recursive cleanup helper involving `{}` is not supported in v0",
                ty.source_name()
            )));
        }

        visiting.push(ty.clone());
        match ty {
            Type::Option(inner) | Type::Array { element: inner, .. } | Type::Slice(inner) => {
                self.emit_drop_helper(inner, emitted, visiting, output)?;
            }
            Type::Result(ok, err) => {
                self.emit_drop_helper(ok, emitted, visiting, output)?;
                self.emit_drop_helper(err, emitted, visiting, output)?;
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                for field in &struct_def.fields {
                    self.emit_drop_helper(&field.ty, emitted, visiting, output)?;
                }
            }
            Type::Int | Type::Bool | Type::String | Type::Unit => {}
        }
        visiting.pop();

        output.push_str(&self.drop_helper_for_type(ty)?);
        output.push('\n');
        emitted.push(ty.clone());
        Ok(())
    }

    fn drop_helper_for_type(&self, ty: &Type) -> Result<String, CompileError> {
        let mut output = format!(
            "static void {}({} *mlg_value) {{\n",
            drop_fn_name(ty),
            ty.c_name()
        );
        let body = self.drop_helper_body(ty)?;
        if body.is_empty() {
            push_indented_lines(&mut output, "(void)mlg_value;", 1);
        } else {
            push_indented_lines(&mut output, &body, 1);
        }
        output.push_str("}\n");
        Ok(output)
    }

    fn drop_helper_body(&self, ty: &Type) -> Result<String, CompileError> {
        match ty {
            Type::Slice(element) => {
                let mut output = String::new();
                if element.needs_cleanup() {
                    output.push_str(&format!(
                        "for (int64_t mlg_i = 0; mlg_i < mlg_value->{}; mlg_i = mlg_i + 1) {{\n",
                        c_field("len")
                    ));
                    push_indented_lines(
                        &mut output,
                        &format!(
                            "{}(&(mlg_value->{}[mlg_i]));",
                            drop_fn_name(element),
                            c_field("data")
                        ),
                        1,
                    );
                    output.push_str("}\n");
                }
                output.push_str(&format!("free(mlg_value->{});\n", c_field("data")));
                output.push_str(&format!("mlg_value->{} = NULL;\n", c_field("data")));
                output.push_str(&format!("mlg_value->{} = 0;\n", c_field("len")));
                output.push_str(&format!("mlg_value->{} = 0;", c_field("cap")));
                Ok(output)
            }
            Type::Option(inner) => {
                if !inner.needs_cleanup() {
                    return Ok(String::new());
                }
                Ok(format!(
                    "if (mlg_value->tag == 1) {{\n    {}(&(mlg_value->some));\n}}",
                    drop_fn_name(inner)
                ))
            }
            Type::Result(ok, err) => {
                let mut output = String::new();
                if ok.needs_cleanup() {
                    output.push_str(&format!(
                        "if (mlg_value->tag == 0) {{\n    {}(&(mlg_value->ok));\n}}\n",
                        drop_fn_name(ok)
                    ));
                }
                if err.needs_cleanup() {
                    if !output.is_empty() {
                        output.push_str("else ");
                    }
                    output.push_str(&format!(
                        "if (mlg_value->tag == 1) {{\n    {}(&(mlg_value->err));\n}}",
                        drop_fn_name(err)
                    ));
                }
                Ok(output)
            }
            Type::Array { len, element } => {
                if *len == 0 || !element.needs_cleanup() {
                    return Ok(String::new());
                }
                let mut output =
                    format!("for (int64_t mlg_i = 0; mlg_i < {len}; mlg_i = mlg_i + 1) {{\n");
                push_indented_lines(
                    &mut output,
                    &format!(
                        "{}(&(mlg_value->{}[mlg_i]));",
                        drop_fn_name(element),
                        c_field("data")
                    ),
                    1,
                );
                output.push('}');
                Ok(output)
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                let mut output = String::new();
                for field in &struct_def.fields {
                    if !field.ty.needs_cleanup() {
                        continue;
                    }
                    output.push_str(&format!(
                        "{}(&(mlg_value->{}));\n",
                        drop_fn_name(&field.ty),
                        c_field(&field.name)
                    ));
                }
                if output.ends_with('\n') {
                    output.pop();
                }
                Ok(output)
            }
            Type::Int | Type::Bool | Type::String | Type::Unit => Err(CompileError::new(format!(
                "IR invariant violation: drop helper requested for non-cleanup type `{}`",
                ty.source_name()
            ))),
        }
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
            Self::Array { .. } | Self::Slice(_) => format!("mlg_{}", mangle_type(self)),
            Self::Struct(name) => format!("mlg_struct_{}", c_type_ident(name)),
        }
    }

    fn c_param_type(&self, mode: ParamMode) -> String {
        match mode {
            ParamMode::Owned => self.c_name(),
            ParamMode::Con => match self {
                Self::String => "const char * const *".to_string(),
                Self::Unit => "const void *".to_string(),
                _ => format!("const {} *", self.c_name()),
            },
            ParamMode::Mut => match self {
                Self::String => "const char **".to_string(),
                Self::Unit => "void *".to_string(),
                _ => format!("{} *", self.c_name()),
            },
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
        Type::Array { element, .. } => {
            collect_type(element, types);
            if !types.contains(ty) {
                types.push(ty.clone());
            }
        }
        Type::Slice(element) => {
            collect_type(element, types);
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

fn if_expr_temp_block(
    condition: &str,
    temp: &str,
    then_expr: CExpr,
    then_cleanup: Vec<String>,
    else_expr: CExpr,
    else_cleanup: Vec<String>,
) -> String {
    let mut output = String::new();
    output.push_str(&format!("if ({}) {{\n", c_condition(condition)));
    for line in then_expr.prelude {
        push_indented_lines(&mut output, &line, 1);
    }
    push_indented_lines(&mut output, &format!("{temp} = {};", then_expr.code), 1);
    for stmt in then_cleanup {
        push_indented_lines(&mut output, &stmt, 1);
    }
    output.push_str("} else {\n");
    for line in else_expr.prelude {
        push_indented_lines(&mut output, &line, 1);
    }
    push_indented_lines(&mut output, &format!("{temp} = {};", else_expr.code), 1);
    for stmt in else_cleanup {
        push_indented_lines(&mut output, &stmt, 1);
    }
    output.push('}');
    output
}

fn if_expr_temp_name(expr: &IrExpr) -> String {
    format!("mallang_if_tmp_{}", expr.span.start)
}

fn match_expr_temp_name(expr: &IrExpr) -> String {
    format!("mallang_match_value_tmp_{}", expr.span.start)
}

fn print_temp_name(expr: &IrExpr) -> String {
    format!("mallang_print_tmp_{}", expr.span.start)
}

fn slice_literal_temp_name(expr: &IrExpr) -> String {
    format!("mallang_slice_tmp_{}", expr.span.start)
}

fn index_source_temp_name(expr: &IrExpr) -> String {
    format!("mallang_index_src_{}", expr.span.start)
}

fn index_value_temp_name(expr: &IrExpr) -> String {
    format!("mallang_index_value_{}", expr.span.start)
}

fn index_assign_value_temp_name(expr: &IrExpr) -> String {
    format!("mallang_index_assign_value_{}", expr.span.start)
}

fn checked_unary_operand_temp_name(expr: &IrExpr) -> String {
    format!("mallang_checked_unary_operand_{}", expr.span.start)
}

fn checked_unary_result_temp_name(expr: &IrExpr) -> String {
    format!("mallang_checked_unary_result_{}", expr.span.start)
}

fn checked_binary_left_temp_name(expr: &IrExpr) -> String {
    format!("mallang_checked_left_{}_{}", expr.span.start, expr.span.end)
}

fn checked_binary_right_temp_name(expr: &IrExpr) -> String {
    format!(
        "mallang_checked_right_{}_{}",
        expr.span.start, expr.span.end
    )
}

fn checked_binary_result_temp_name(expr: &IrExpr) -> String {
    format!(
        "mallang_checked_result_{}_{}",
        expr.span.start, expr.span.end
    )
}

fn checked_int_binary_builtin(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Add => Some("__builtin_add_overflow"),
        BinaryOp::Subtract => Some("__builtin_sub_overflow"),
        BinaryOp::Multiply => Some("__builtin_mul_overflow"),
        _ => None,
    }
}

fn dividend_temp_name(expr: &IrExpr) -> String {
    format!("mallang_dividend_{}_{}", expr.span.start, expr.span.end)
}

fn divisor_temp_name(expr: &IrExpr) -> String {
    format!("mallang_divisor_{}_{}", expr.span.start, expr.span.end)
}

fn param_env(function: &IrFunction) -> HashMap<String, String> {
    function
        .params
        .iter()
        .filter(|param| !matches!(param.mode, ParamMode::Owned))
        .map(|param| (param.name.clone(), format!("(*{})", c_ident(&param.name))))
        .collect()
}

fn c_param_decl(param: &crate::ir::IrParam) -> String {
    format!(
        "{} {}",
        param.ty.c_param_type(param.mode),
        c_ident(&param.name)
    )
}

fn c_assignment_target(name: &str, env: &HashMap<String, String>) -> String {
    env.get(name).cloned().unwrap_or_else(|| c_ident(name))
}

fn c_arg_code(mode: ArgMode, code: String) -> String {
    match mode {
        ArgMode::Owned => code,
        ArgMode::Con | ArgMode::Mut => format!("&({code})"),
    }
}

fn c_condition(code: &str) -> String {
    strip_enclosing_parens(code).unwrap_or(code).to_string()
}

fn strip_enclosing_parens(code: &str) -> Option<&str> {
    let bytes = code.as_bytes();
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return None;
    }

    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 && index != bytes.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }

    if depth == 0 {
        Some(&code[1..code.len() - 1])
    } else {
        None
    }
}

fn c_binary_expr(
    op: BinaryOp,
    result_ty: &Type,
    operand_ty: &Type,
    left: String,
    right: String,
) -> String {
    if matches!(operand_ty, Type::String) && matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
        let comparison = match op {
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            _ => unreachable!("string comparison only supports equality operators"),
        };
        return format!("(strcmp({left}, {right}) {comparison} 0)");
    }

    debug_assert!(!matches!(result_ty, Type::String));
    format!("({left} {} {right})", op.c_operator())
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

fn for_post_label(post: &IrForPost) -> String {
    match post {
        IrForPost::Assign { target, .. } => format!("mallang_for_post_{}", target.span.start),
    }
}

fn range_source_temp_name(expr: &IrExpr) -> String {
    format!("mallang_range_src_{}", expr.span.start)
}

fn range_index_temp_name(expr: &IrExpr) -> String {
    format!("mallang_range_index_{}", expr.span.start)
}

fn slice_append_temp_name(expr: &IrExpr) -> String {
    format!("mallang_slice_append_tmp_{}", expr.span.start)
}

fn is_blank_identifier(name: &str) -> bool {
    name == "_"
}

fn mangle_type(ty: &Type) -> String {
    match ty {
        Type::Int => "int".to_string(),
        Type::Bool => "bool".to_string(),
        Type::String => "string".to_string(),
        Type::Unit => "unit".to_string(),
        Type::Option(inner) => format!("Option_{}", mangle_type(inner)),
        Type::Result(ok, err) => format!("Result_{}_{}", mangle_type(ok), mangle_type(err)),
        Type::Array { len, element } => format!("Array_{}_{}", len, mangle_type(element)),
        Type::Slice(element) => format!("Slice_{}", mangle_type(element)),
        Type::Struct(name) => format!("Struct_{}", c_type_ident(name)),
    }
}

fn drop_fn_name(ty: &Type) -> String {
    format!("mlg_drop_{}", mangle_type(ty))
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
            Self::LogicalAnd => "&&",
            Self::LogicalOr => "||",
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
    use crate::{
        check,
        ir::{lower, IrStruct, IrStructField},
        parse,
    };

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
    fn generates_c_for_internal_owned_slice_type_shell() {
        let program = IrProgram {
            structs: Vec::new(),
            functions: vec![
                IrFunction {
                    name: "consume".to_string(),
                    params: vec![crate::ir::IrParam {
                        name: "values".to_string(),
                        mode: ParamMode::Owned,
                        ty: Type::Slice(Box::new(Type::Int)),
                    }],
                    return_type: Type::Unit,
                    body: Vec::new(),
                },
                IrFunction {
                    name: "main".to_string(),
                    params: Vec::new(),
                    return_type: Type::Unit,
                    body: Vec::new(),
                },
            ],
        };

        let c = generate_c_from_ir(&program).unwrap();

        assert!(c.contains(
            "typedef struct {\n    int64_t *mlg_data;\n    int64_t mlg_len;\n    int64_t mlg_cap;\n} mlg_Slice_int;"
        ));
        assert!(c.contains("void mlg_consume(mlg_Slice_int mlg_values);"));
    }

    #[test]
    fn generates_c_drop_helpers_for_internal_cleanup_types() {
        let program = IrProgram {
            structs: vec![IrStruct {
                name: "Holder".to_string(),
                fields: vec![
                    IrStructField {
                        name: "values".to_string(),
                        ty: Type::Slice(Box::new(Type::Int)),
                    },
                    IrStructField {
                        name: "count".to_string(),
                        ty: Type::Int,
                    },
                ],
            }],
            functions: vec![
                IrFunction {
                    name: "consume".to_string(),
                    params: vec![crate::ir::IrParam {
                        name: "values".to_string(),
                        mode: ParamMode::Owned,
                        ty: Type::Option(Box::new(Type::Slice(Box::new(Type::Int)))),
                    }],
                    return_type: Type::Unit,
                    body: Vec::new(),
                },
                IrFunction {
                    name: "main".to_string(),
                    params: Vec::new(),
                    return_type: Type::Unit,
                    body: Vec::new(),
                },
            ],
        };

        let c = generate_c_from_ir(&program).unwrap();

        assert!(c.contains(
            "static void mlg_drop_Slice_int(mlg_Slice_int *mlg_value) {\n    free(mlg_value->mlg_data);\n    mlg_value->mlg_data = NULL;\n    mlg_value->mlg_len = 0;\n    mlg_value->mlg_cap = 0;\n}"
        ));
        assert!(c.contains(
            "static void mlg_drop_Option_Slice_int(mlg_Option_Slice_int *mlg_value) {\n    if (mlg_value->tag == 1) {\n        mlg_drop_Slice_int(&(mlg_value->some));\n    }\n}"
        ));
        assert!(c.contains(
            "static void mlg_drop_Struct_Holder(mlg_struct_Holder *mlg_value) {\n    mlg_drop_Slice_int(&(mlg_value->mlg_values));\n}"
        ));
    }

    #[test]
    fn generates_c_for_explicit_internal_drop_statement() {
        let slice_ty = Type::Slice(Box::new(Type::Int));
        let span = crate::token::Span { start: 0, end: 0 };
        let program = IrProgram {
            structs: Vec::new(),
            functions: vec![
                IrFunction {
                    name: "consume".to_string(),
                    params: vec![crate::ir::IrParam {
                        name: "values".to_string(),
                        mode: ParamMode::Owned,
                        ty: slice_ty.clone(),
                    }],
                    return_type: Type::Unit,
                    body: vec![IrStmt {
                        kind: IrStmtKind::Drop {
                            expr: IrExpr {
                                kind: IrExprKind::Var("values".to_string()),
                                ty: slice_ty,
                                span,
                            },
                        },
                        span,
                    }],
                },
                IrFunction {
                    name: "main".to_string(),
                    params: Vec::new(),
                    return_type: Type::Unit,
                    body: Vec::new(),
                },
            ],
        };

        let c = generate_c_from_ir(&program).unwrap();

        assert!(c.contains("void mlg_consume(mlg_Slice_int mlg_values) {\n    mlg_drop_Slice_int(&(mlg_values));\n}"));
    }

    #[test]
    fn generates_c_for_explicit_internal_cleanup_field_drop_statement() {
        let slice_ty = Type::Slice(Box::new(Type::Int));
        let span = crate::token::Span { start: 0, end: 0 };
        let program = IrProgram {
            structs: vec![IrStruct {
                name: "Holder".to_string(),
                fields: vec![IrStructField {
                    name: "values".to_string(),
                    ty: slice_ty.clone(),
                }],
            }],
            functions: vec![IrFunction {
                name: "consume".to_string(),
                params: vec![crate::ir::IrParam {
                    name: "holder".to_string(),
                    mode: ParamMode::Owned,
                    ty: Type::Struct("Holder".to_string()),
                }],
                return_type: Type::Unit,
                body: vec![IrStmt {
                    kind: IrStmtKind::Drop {
                        expr: IrExpr {
                            kind: IrExprKind::FieldAccess {
                                base: Box::new(IrExpr {
                                    kind: IrExprKind::Var("holder".to_string()),
                                    ty: Type::Struct("Holder".to_string()),
                                    span,
                                }),
                                field: "values".to_string(),
                            },
                            ty: slice_ty,
                            span,
                        },
                    },
                    span,
                }],
            }],
        };

        let c = generate_c_from_ir(&program).unwrap();

        assert!(c.contains("mlg_drop_Slice_int(&((mlg_holder).mlg_values));"));
    }

    #[test]
    fn generates_c_for_explicit_internal_cleanup_array_element_drop_statement() {
        let slice_ty = Type::Slice(Box::new(Type::Int));
        let array_ty = Type::Array {
            len: 2,
            element: Box::new(slice_ty.clone()),
        };
        let span = crate::token::Span { start: 0, end: 0 };
        let program = IrProgram {
            structs: Vec::new(),
            functions: vec![IrFunction {
                name: "consume".to_string(),
                params: vec![crate::ir::IrParam {
                    name: "values".to_string(),
                    mode: ParamMode::Owned,
                    ty: array_ty.clone(),
                }],
                return_type: Type::Unit,
                body: vec![IrStmt {
                    kind: IrStmtKind::Drop {
                        expr: IrExpr {
                            kind: IrExprKind::Index {
                                base: Box::new(IrExpr {
                                    kind: IrExprKind::Var("values".to_string()),
                                    ty: array_ty,
                                    span,
                                }),
                                index: Box::new(IrExpr {
                                    kind: IrExprKind::Int(0),
                                    ty: Type::Int,
                                    span,
                                }),
                            },
                            ty: slice_ty,
                            span,
                        },
                    },
                    span,
                }],
            }],
        };

        let c = generate_c_from_ir(&program).unwrap();

        assert!(
            c.contains("mlg_drop_Slice_int(&((mlg_values).mlg_data[mallang_check_index(0, 2)]));")
        );
    }

    #[test]
    fn generates_c_for_for_init_cleanup_trailer() {
        let slice_ty = Type::Slice(Box::new(Type::Int));
        let span = crate::token::Span { start: 0, end: 0 };
        let program = IrProgram {
            structs: Vec::new(),
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: vec![crate::ir::IrParam {
                    name: "seed".to_string(),
                    mode: ParamMode::Owned,
                    ty: slice_ty.clone(),
                }],
                return_type: Type::Unit,
                body: vec![IrStmt {
                    kind: IrStmtKind::For {
                        init: Some(Box::new(IrForInit::Let {
                            mutable: false,
                            name: "loop_values".to_string(),
                            ty: slice_ty.clone(),
                            expr: IrExpr {
                                kind: IrExprKind::Var("seed".to_string()),
                                ty: slice_ty.clone(),
                                span,
                            },
                        })),
                        condition: None,
                        post: None,
                        body: Vec::new(),
                        cleanup: vec![IrStmt {
                            kind: IrStmtKind::Drop {
                                expr: IrExpr {
                                    kind: IrExprKind::Var("loop_values".to_string()),
                                    ty: slice_ty,
                                    span,
                                },
                            },
                            span,
                        }],
                    },
                    span,
                }],
            }],
        };

        let c = generate_c_from_ir(&program).unwrap();

        assert!(c.contains("mlg_Slice_int mlg_loop_values = mlg_seed;"));
        assert!(c.contains("while (true) {"));
        assert!(c.contains("mlg_drop_Slice_int(&(mlg_loop_values));"));
    }

    #[test]
    fn rejects_explicit_internal_drop_for_non_cleanup_type() {
        let span = crate::token::Span { start: 0, end: 0 };
        let program = IrProgram {
            structs: Vec::new(),
            functions: vec![IrFunction {
                name: "main".to_string(),
                params: Vec::new(),
                return_type: Type::Unit,
                body: vec![IrStmt {
                    kind: IrStmtKind::Drop {
                        expr: IrExpr {
                            kind: IrExprKind::Var("value".to_string()),
                            ty: Type::Int,
                            span,
                        },
                    },
                    span,
                }],
            }],
        };

        let error = generate_c_from_ir(&program).unwrap_err();

        assert!(error
            .message
            .contains("drop requested for non-cleanup type `int`"));
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
    fn generates_c_for_logical_operators_from_ir() {
        let program = parse(
            r#"
func main() {
    print(check(7, true, false))
}

func check(score int, left bool, right bool) bool {
    return left || right && score > 5
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains(" || "));
        assert!(c.contains(" && "));
    }

    #[test]
    fn generates_c_guard_for_integer_division_and_remainder() {
        let program = parse(
            r#"
func main() {
    value := 20
    divisor := 6
    print(value / divisor)
    print(value % divisor)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("int64_t mallang_divisor_"));
        assert!(c.contains("int64_t mallang_dividend_"));
        assert!(c.contains("mallang runtime error: division by zero"));
        assert!(c.contains("mallang runtime error: integer overflow"));
        assert!(c.contains("if (mallang_divisor_"));
        assert!(c.contains(" == INT64_MIN && mallang_divisor_"));
        assert!(c.contains(" / mallang_divisor_"));
        assert!(c.contains(" % mallang_divisor_"));
    }

    #[test]
    fn generates_c_guards_for_checked_integer_arithmetic() {
        let program = parse(
            r#"
func main() {
    value := 20
    step := 3
    print(value + step)
    print(value - step)
    print(value * step)
    print(-step)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("__builtin_sub_overflow"));
        assert!(c.contains("__builtin_mul_overflow"));
        assert!(c.contains("int64_t mallang_checked_left_"));
        assert!(c.contains("int64_t mallang_checked_right_"));
        assert!(c.contains("int64_t mallang_checked_result_"));
        assert!(c.contains("int64_t mallang_checked_unary_operand_"));
        assert!(c.contains("int64_t mallang_checked_unary_result_"));
        assert!(c.contains("mallang runtime error: integer overflow"));
    }

    #[test]
    fn generates_c_for_bool_unary_not() {
        let program = parse(
            r#"
func main() {
    print(!false)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("(!false)"));
    }

    #[test]
    fn generates_c_for_for_statement() {
        let program = parse(
            r#"
func main() {
    mut count := 0
    for count < 3 {
        count = count + 1
    }
    print(count)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("while (mlg_count < 3) {"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("mlg_count = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_for_statement_without_condition() {
        let program = parse(
            r#"
func main() {
    for {
        break
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("while (true) {"));
        assert!(c.contains("break;"));
    }

    #[test]
    fn generates_c_for_for_clause_statement() {
        let program = parse(
            r#"
func main() {
    for mut i := 0; i < 3; i = i + 1 {
        print(i)
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("int64_t mlg_i = 0;"));
        assert!(c.contains("while (true) {"));
        assert!(c.contains("if (!(mlg_i < 3)) {"));
        assert!(c.contains("mallang_for_post_"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("mlg_i = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_initless_for_clause_statement() {
        let program = parse(
            r#"
func main() {
    mut i := 0
    for ; i < 3; i = i + 1 {
        print(i)
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("while (true) {"));
        assert!(c.contains("if (!(mlg_i < 3)) {"));
        assert!(c.contains("mallang_for_post_"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("mlg_i = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_for_clause_without_condition() {
        let program = parse(
            r#"
func main() {
    mut i := 0
    for ; ; i = i + 1 {
        if i == 3 {
            break
        }
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("while (true) {"));
        assert!(c.contains("mallang_for_post_"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("mlg_i = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_loop_control_statements() {
        let program = parse(
            r#"
func main() {
    for true {
        continue
        break
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("continue;"));
        assert!(c.contains("break;"));
    }

    #[test]
    fn generates_c_for_if_expression_with_branch_prelude() {
        let program = parse(
            r#"
func main() {
    print(pick(true))
}

func pick(flag bool) int {
    return if flag {
        match maybe(true) {
            case Some(inner) { inner }
            case None { 0 }
        }
    } else {
        match maybe(false) {
            case Some(inner) { inner }
            case None { 0 }
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

        assert!(c.contains("int64_t mallang_if_tmp_"));
        assert!(c.contains("if (mlg_flag) {"));
        assert!(c.contains("mlg_Option_int mallang_match_tmp_"));
        assert!(c.contains("mallang_if_tmp_"));
        assert!(c.contains("return mallang_if_tmp_"));
    }

    #[test]
    fn generates_c_for_if_expression_cleanup_trailer() {
        let slice_ty = Type::Slice(Box::new(Type::Int));
        let span = crate::token::Span { start: 0, end: 0 };
        let program = IrProgram {
            structs: Vec::new(),
            functions: vec![
                IrFunction {
                    name: "pick".to_string(),
                    params: vec![
                        crate::ir::IrParam {
                            name: "values".to_string(),
                            mode: ParamMode::Owned,
                            ty: slice_ty.clone(),
                        },
                        crate::ir::IrParam {
                            name: "replacement".to_string(),
                            mode: ParamMode::Owned,
                            ty: slice_ty.clone(),
                        },
                    ],
                    return_type: slice_ty.clone(),
                    body: vec![IrStmt {
                        kind: IrStmtKind::Return {
                            expr: IrExpr {
                                kind: IrExprKind::If {
                                    condition: Box::new(IrExpr {
                                        kind: IrExprKind::Bool(true),
                                        ty: Type::Bool,
                                        span,
                                    }),
                                    then_branch: Box::new(IrExpr {
                                        kind: IrExprKind::Var("values".to_string()),
                                        ty: slice_ty.clone(),
                                        span,
                                    }),
                                    then_cleanup: vec![IrStmt {
                                        kind: IrStmtKind::Drop {
                                            expr: IrExpr {
                                                kind: IrExprKind::Var("replacement".to_string()),
                                                ty: slice_ty.clone(),
                                                span,
                                            },
                                        },
                                        span,
                                    }],
                                    else_branch: Box::new(IrExpr {
                                        kind: IrExprKind::Var("replacement".to_string()),
                                        ty: slice_ty.clone(),
                                        span,
                                    }),
                                    else_cleanup: vec![IrStmt {
                                        kind: IrStmtKind::Drop {
                                            expr: IrExpr {
                                                kind: IrExprKind::Var("values".to_string()),
                                                ty: slice_ty.clone(),
                                                span,
                                            },
                                        },
                                        span,
                                    }],
                                },
                                ty: slice_ty,
                                span,
                            },
                        },
                        span,
                    }],
                },
                IrFunction {
                    name: "main".to_string(),
                    params: Vec::new(),
                    return_type: Type::Unit,
                    body: Vec::new(),
                },
            ],
        };

        let c = generate_c_from_ir(&program).unwrap();

        assert!(c.contains("mlg_Slice_int mallang_if_tmp_0;"));
        assert!(c.contains(
            "if (true) {\n        mallang_if_tmp_0 = mlg_values;\n        mlg_drop_Slice_int(&(mlg_replacement));\n    } else {\n        mallang_if_tmp_0 = mlg_replacement;\n        mlg_drop_Slice_int(&(mlg_values));\n    }\n    return mallang_if_tmp_0;"
        ));
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
    fn generates_c_for_adt_printing() {
        let program = parse(
            r#"
func main() {
    print(maybe(true))
    print(read(false))
}

func maybe(flag bool) Option[int] {
    return if flag { Some(7) } else { None }
}

func read(flag bool) Result[int, string] {
    return if flag { Ok(1) } else { Err("bad") }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mallang_print_tmp_"));
        assert!(c.contains("printf(\"Some(\");"));
        assert!(c.contains("printf(\"Err(\");"));
    }

    #[test]
    fn generates_c_for_struct_printing() {
        let program = parse(
            r#"
type User struct {
    name string
    age int
}

func main() {
    user := User{name: "kim", age: 30}
    print(user)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("printf(\"User{\");"));
        assert!(c.contains("printf(\"name: \");"));
        assert!(c.contains("printf(\"age: \");"));
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
    fn generates_c_for_match_expression_with_arm_prelude() {
        let program = parse(
            r#"
func main() {
    print(resolve(true))
}

func resolve(flag bool) int {
    value := maybe(flag)
    return match value {
        case Some(inner) {
            if inner == 7 {
                match maybe(true) {
                    case Some(nested) { nested }
                    case None { 0 }
                }
            } else {
                inner
            }
        }
        case None { 0 }
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

        assert!(c.contains("int64_t mallang_match_value_tmp_"));
        assert!(c.contains("if ((mlg_value).tag == 1) {"));
        assert!(c.contains("int64_t mallang_if_tmp_"));
        assert!(c.contains("mlg_Option_int mallang_match_tmp_"));
        assert!(c.contains("return mallang_match_value_tmp_"));
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
    fn generates_c_for_fixed_size_array_literals() {
        let program = parse(
            r#"
func consume(values [3]int) {
}

func main() {
    values := [3]int{1, 2, 3}
    consume(values)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("typedef struct"));
        assert!(c.contains("int64_t mlg_data[3];"));
        assert!(c.contains("mlg_Array_3_int"));
        assert!(c.contains("(mlg_Array_3_int){ .mlg_data = { 1, 2, 3 } }"));
        assert!(c.contains("mlg_consume(mlg_values);"));
    }

    #[test]
    fn generates_c_for_array_range_loops() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    mut total := 0
    for i, value := range values {
        total = total + i + value
    }
    print(total)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mlg_Array_3_int mallang_range_src_"));
        assert!(c.contains("for (int64_t mlg_i = 0; mlg_i < 3; mlg_i = (mlg_i + 1)) {"));
        assert!(c.contains("int64_t mlg_value = (mallang_range_src_"));
        assert!(c.contains(".mlg_data[mlg_i];"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("mlg_total = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_slice_range_loops() {
        let program = parse(
            r#"
func main() {
    values := []int{1, 2, 3}
    mut total := 0
    for i, value := range values {
        total = total + i + value
    }
    print(total)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mlg_Slice_int mallang_range_src_"));
        assert!(c.contains("for (int64_t mlg_i = 0; mlg_i < (mallang_range_src_"));
        assert!(c.contains(".mlg_len; mlg_i = (mlg_i + 1)) {"));
        assert!(c.contains("int64_t mlg_value = (mallang_range_src_"));
        assert!(c.contains(".mlg_data[mlg_i];"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("mlg_total = mallang_checked_result_"));
        assert!(c.contains("mlg_drop_Slice_int(&(mlg_values));"));
    }

    #[test]
    fn generates_c_for_array_range_blank_identifiers() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    values := [2]int{1, 2}
    for _, value := range values {
        print(value)
    }

    users := [1]User{User{age: 1}}
    for i, _ := range users {
        print(i)
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("for (int64_t mallang_range_index_"));
        assert!(c.contains("int64_t mlg_value = (mallang_range_src_"));
        assert!(c.contains("for (int64_t mlg_i = 0; mlg_i < 1; mlg_i = (mlg_i + 1)) {"));
        assert!(!c.contains("mlg__"));
    }

    #[test]
    fn generates_c_for_one_variable_array_range() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    users := [2]User{User{age: 1}, User{age: 2}}
    for i := range users {
        print(i)
    }
    for _ := range users {
        print(1)
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("for (int64_t mlg_i = 0; mlg_i < 2; mlg_i = (mlg_i + 1)) {"));
        assert!(c.contains("for (int64_t mallang_range_index_"));
        assert!(!c.contains("mlg__"));
    }

    #[test]
    fn generates_c_for_fixed_size_array_indexing_and_len() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    total := values[1] + len(values)
    print(total)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("#include <stdlib.h>"));
        assert!(c.contains("mlg_Array_3_int mallang_index_src_"));
        assert!(c.contains("int64_t mallang_index_value_"));
        assert!(c.contains("if (mallang_index_value_"));
        assert!(c.contains("mallang runtime error: array index out of bounds"));
        assert!(c.contains(".mlg_data[mallang_index_value_"));
        assert!(c.contains("(void)(mlg_values);"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("int64_t mlg_total = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_slice_literal_indexing_len_and_cleanup() {
        let program = parse(
            r#"
func main() {
    values := []int{1, 2, 3}
    total := values[1] + len(values)
    print(total)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("typedef struct {\n    int64_t *mlg_data;\n    int64_t mlg_len;\n    int64_t mlg_cap;\n} mlg_Slice_int;"));
        assert!(c.contains("mlg_Slice_int mallang_slice_tmp_"));
        assert!(c.contains(".mlg_data = malloc(sizeof(int64_t) * 3);"));
        assert!(c.contains(".mlg_len = 3;"));
        assert!(c.contains(".mlg_cap = 3;"));
        assert!(c.contains(".mlg_data[0] = 1;"));
        assert!(c.contains("mallang runtime error: slice index out of bounds"));
        assert!(c.contains(".mlg_data[mallang_index_value_"));
        assert!(c.contains(".mlg_len"));
        assert!(c.contains("mlg_drop_Slice_int(&(mlg_values));"));
    }

    #[test]
    fn generates_c_for_local_rooted_slice_field_reads() {
        let program = parse(
            r#"
type Bag struct {
    values []int
}

func main() {
    mut bag := Bag{values: []int{1, 2, 3}}
    print(len(bag.values))
    print(bag.values[1])
    show(con bag.values[0])
    bump(mut bag.values[2])
    mut total := 0
    for _, value := range bag.values {
        total = total + value
    }
    print(total)
}

func show(con value int) {
    print(value)
}

func bump(mut value int) {
    value = value + 10
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("((mlg_bag).mlg_values).mlg_len"));
        assert!(c.contains("((mlg_bag).mlg_values).mlg_data"));
        assert!(c.contains("mlg_show(&(((mlg_bag).mlg_values"));
        assert!(c.contains("mlg_bump"));
        assert!(c.contains("&(((mlg_bag).mlg_values).mlg_data[mallang_index_value_"));
        assert!(c.contains("mlg_Slice_int mallang_range_src_"));
        assert!(c.contains("mlg_drop_Struct_Bag(&(mlg_bag));"));
    }

    #[test]
    fn generates_c_for_slice_append_reassignment_and_cleanup() {
        let program = parse(
            r#"
func main() {
    mut values := []int{1, 2}
    values = append(values, 3)
    total := values[2] + len(values)
    print(total)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mlg_Slice_int mallang_slice_append_tmp_"));
        assert!(c.contains("int64_t mallang_slice_append_tmp_"));
        assert!(c.contains("_new_len = mallang_slice_append_tmp_"));
        assert!(c.contains("void *mallang_slice_append_tmp_"));
        assert!(c.contains("realloc(mallang_slice_append_tmp_"));
        assert!(c.contains("mallang runtime error: slice allocation failed"));
        assert!(c.contains(".mlg_data[mallang_slice_append_tmp_"));
        assert!(c.contains(" = 3;"));
        assert!(c.contains("mlg_values = mallang_slice_append_tmp_"));
        assert!(c.contains("mlg_drop_Slice_int(&(mlg_values));"));
    }

    #[test]
    fn generates_c_for_fixed_size_array_element_assignment() {
        let program = parse(
            r#"
func main() {
    mut values := [3]int{1, 2, 3}
    index := 1
    values[index] = 5
    print(values[index])
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("int64_t mallang_index_assign_value_"));
        assert!(c.contains("if (mallang_index_assign_value_"));
        assert!(c.contains("(mlg_values).mlg_data[mallang_index_assign_value_"));
        assert!(c.contains("] = 5;"));
    }

    #[test]
    fn generates_c_for_slice_element_assignment() {
        let program = parse(
            r#"
type User struct {
    name string
}

func main() {
    mut values := []int{1, 2, 3}
    values[1] = 5
    print(values[1])

    mut users := []User{User{name: "kim"}, User{name: "lee"}}
    users[1] = User{name: "park"}
    show(con users[1].name)
}

func show(con name string) {
    print(name)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("int64_t mallang_index_assign_value_"));
        assert!(c.contains("mallang runtime error: slice index out of bounds"));
        assert!(c.contains(">= (mlg_values).mlg_len"));
        assert!(c.contains("(mlg_values).mlg_data[mallang_index_assign_value_"));
        assert!(c.contains("] = 5;"));
        assert!(c.contains(">= (mlg_users).mlg_len"));
        assert!(c.contains("mlg_struct_User mlg_mallang_cleanup_assign_rhs_"));
        assert!(c.contains(" = (mlg_struct_User){ .mlg_name = \"park\" };"));
        assert!(c.contains("mlg_drop_Struct_User(&((mlg_users).mlg_data[mallang_index_value_"));
        assert!(c.contains("(mlg_users).mlg_data[mallang_index_assign_value_"));
        assert!(c.contains("] = mlg_mallang_cleanup_assign_rhs_"));
        assert!(c.contains("mlg_drop_Slice_int(&(mlg_values));"));
        assert!(c.contains("mlg_drop_Slice_Struct_User(&(mlg_users));"));
    }

    #[test]
    fn generates_c_for_cleanup_slice_element_assignment() {
        let program = parse(
            r#"
func main() {
    mut rows := [][]int{[]int{1}, []int{2}}
    rows[0] = []int{3}
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mlg_Slice_int mlg_mallang_cleanup_assign_rhs_"));
        assert!(c.contains("mlg_drop_Slice_int(&((mlg_rows).mlg_data[mallang_index_value_"));
        assert!(c.contains("(mlg_rows).mlg_data[mallang_index_assign_value_"));
        assert!(c.contains("] = mlg_mallang_cleanup_assign_rhs_"));
        assert!(c.contains("mlg_drop_Slice_Slice_int(&(mlg_rows));"));
    }

    #[test]
    fn generates_c_for_non_copy_array_element_assignment() {
        let program = parse(
            r#"
type User struct {
    name string
}

func main() {
    mut users := [2]User{User{name: "kim"}, User{name: "lee"}}
    users[1] = User{name: "park"}
    show(con users[1].name)
}

func show(con name string) {
    print(name)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("mlg_struct_User mlg_data[2];"));
        assert!(c.contains("mlg_struct_User mlg_mallang_cleanup_assign_rhs_"));
        assert!(c.contains(" = (mlg_struct_User){ .mlg_name = \"park\" };"));
        assert!(
            c.contains("mlg_drop_Struct_User(&((mlg_users).mlg_data[mallang_check_index(1, 2)]));")
        );
        assert!(c.contains("(mlg_users).mlg_data[mallang_index_assign_value_"));
        assert!(c.contains("] = mlg_mallang_cleanup_assign_rhs_"));
        assert!(
            c.contains("mlg_show(&(((mlg_users).mlg_data[mallang_check_index(1, 2)]).mlg_name));")
        );
    }

    #[test]
    fn generates_c_for_fixed_size_array_element_assignment_in_for_post() {
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
    print(values[0] + values[1] + values[2])
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("static int64_t mallang_check_index"));
        assert!(c.contains("while (true) {"));
        assert!(c.contains("mallang_for_post_"));
        assert!(c.contains("(mlg_values).mlg_data[mallang_check_index(mlg_slot, 3)] = mlg_i;"));
    }

    #[test]
    fn generates_c_for_for_clause_condition_and_post_preludes() {
        let program = parse(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    mut i := 0
    mut slot := 0
    mut total := 0
    for ; i < len(values); total = total + values[slot] {
        slot = i
        i = i + 1
    }
    print(total)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("(void)(mlg_values);"));
        assert!(c.contains("if (!(mlg_i < 3)) {"));
        assert!(c.contains("mallang_for_post_"));
        assert!(c.contains("mlg_Array_3_int mallang_index_src_"));
        assert!(c.contains("int64_t mallang_index_value_"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("mlg_total = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_struct_methods() {
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
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("int64_t mlg_User_age(const mlg_struct_User * mlg_self);"));
        assert!(c.contains("return ((*mlg_self)).mlg_age;"));
        assert!(c.contains("printf(\"%lld\\n\", (long long)(mlg_User_age(&(mlg_user))));"));
    }

    #[test]
    fn generates_c_for_mut_receiver_methods() {
        let program = parse(
            r#"
type Counter struct {
    value int
}

func (mut self Counter) inc() {
    self.value = self.value + 1
}

func main() {
    mut counter := Counter{value: 1}
    counter.inc()
    print(counter.value)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("void mlg_Counter_inc(mlg_struct_Counter * mlg_self);"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("((*mlg_self)).mlg_value = mallang_checked_result_"));
        assert!(c.contains("mlg_Counter_inc(&(mlg_counter));"));
    }

    #[test]
    fn generates_c_for_array_element_method_receivers() {
        let program = parse(
            r#"
type Counter struct {
    value int
}

func (mut self Counter) inc() {
    self.value = self.value + 1
}

func main() {
    mut counters := [2]Counter{Counter{value: 1}, Counter{value: 2}}
    counters[1].inc()
    show(con counters[1].value)
}

func show(con value int) {
    print(value)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("void mlg_Counter_inc(mlg_struct_Counter * mlg_self);"));
        assert!(
            c.contains("mlg_Counter_inc(&((mlg_counters).mlg_data[mallang_check_index(1, 2)]));")
        );
        assert!(c.contains(
            "mlg_show(&(((mlg_counters).mlg_data[mallang_check_index(1, 2)]).mlg_value));"
        ));
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

    #[test]
    fn generates_c_for_indexed_field_assignment() {
        let program = parse(
            r#"
type User struct {
    name string
    age int
}

func main() {
    mut arrayUsers := [1]User{User{name: "kim", age: 30}}
    arrayUsers[0].age = 31

    mut sliceUsers := []User{User{name: "lee", age: 20}}
    sliceUsers[0].name = "park"
    sliceUsers[0].age = 21
    showName(con sliceUsers[0].name)
    showAge(con sliceUsers[0].age)
}

func showName(con name string) {
    print(name)
}

func showAge(con age int) {
    print(age)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("((mlg_arrayUsers).mlg_data[mallang_check_index(0, 1)]).mlg_age = 31;"));
        assert!(c.contains("mallang runtime error: slice index out of bounds"));
        assert!(c.contains(">= (mlg_sliceUsers).mlg_len"));
        assert!(c.contains("((mlg_sliceUsers).mlg_data[mallang_index_value_"));
        assert!(c.contains("]).mlg_name = \"park\";"));
        assert!(c.contains("]).mlg_age = 21;"));
    }

    #[test]
    fn generates_c_for_string_equality() {
        let program = parse(
            r#"
func main() {
    word := "mallang"
    print(word == "mallang")
    print(word != "rust")
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("#include <string.h>"));
        assert!(c.contains("strcmp(mlg_word, \"mallang\") == 0"));
        assert!(c.contains("strcmp(mlg_word, \"rust\") != 0"));
    }

    #[test]
    fn generates_c_pointer_abi_for_mut_borrow_params() {
        let program = parse(
            r#"
type User struct {
    name string
    age int
}

func main() {
    mut user := User{name: "kim", age: 30}
    rename(mut user.name)
    bump(mut user.age)
    print(user.name)
    print(user.age)
}

func rename(mut name string) {
    name = "lee"
}

func bump(mut age int) {
    age = age + 1
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("void mlg_rename(const char ** mlg_name);"));
        assert!(c.contains("void mlg_bump(int64_t * mlg_age);"));
        assert!(c.contains("mlg_rename(&((mlg_user).mlg_name));"));
        assert!(c.contains("mlg_bump(&((mlg_user).mlg_age));"));
        assert!(c.contains("(*mlg_name) = \"lee\";"));
        assert!(c.contains("__builtin_add_overflow"));
        assert!(c.contains("(*mlg_age) = mallang_checked_result_"));
    }

    #[test]
    fn generates_c_for_array_element_borrow_arguments() {
        let program = parse(
            r#"
type User struct {
    name string
}

func main() {
    mut users := [2]User{User{name: "kim"}, User{name: "lee"}}
    show(con users[0].name)
    rename(mut users[1].name)
}

func show(con name string) {
    print(name)
}

func rename(mut name string) {
    name = "park"
    print(name)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("void mlg_show(const char * const * mlg_name);"));
        assert!(c.contains("void mlg_rename(const char ** mlg_name);"));
        assert!(
            c.contains("mlg_show(&(((mlg_users).mlg_data[mallang_check_index(0, 2)]).mlg_name));")
        );
        assert!(c.contains(
            "mlg_rename(&(((mlg_users).mlg_data[mallang_check_index(1, 2)]).mlg_name));"
        ));
        assert!(c.contains("(*mlg_name) = \"park\";"));
    }

    #[test]
    fn generates_c_for_slice_element_borrow_arguments() {
        let program = parse(
            r#"
type User struct {
    name string
}

func main() {
    mut users := []User{User{name: "kim"}, User{name: "lee"}}
    show(con users[0].name)
    rename(mut users[1].name)
}

func show(con name string) {
    print(name)
}

func rename(mut name string) {
    name = "park"
    print(name)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(c.contains("void mlg_show(const char * const * mlg_name);"));
        assert!(c.contains("void mlg_rename(const char ** mlg_name);"));
        assert!(c.contains("int64_t mallang_index_value_"));
        assert!(c.contains("mallang runtime error: slice index out of bounds"));
        assert!(c.contains("mlg_show(&(((mlg_users).mlg_data[mallang_index_value_"));
        assert!(c.contains("mlg_rename(&(((mlg_users).mlg_data[mallang_index_value_"));
        assert!(c.contains("]).mlg_name));"));
        assert!(c.contains("(*mlg_name) = \"park\";"));
        assert!(c.contains("mlg_drop_Slice_Struct_User(&(mlg_users));"));
    }
}
