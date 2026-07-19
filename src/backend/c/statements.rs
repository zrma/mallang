use std::collections::HashMap;

use crate::{
    ir::{IrArg, IrExpr, IrExprKind, IrForInit, IrForPost, IrMatchBlockArm, IrStmt, IrStmtKind},
    semantic::Type,
    standard::StandardType,
};

use super::{
    names::{
        c_assignment_target, c_condition, c_field, c_ident, c_string_literal, drop_fn_name,
        TypeCName,
    },
    utils::{
        condition_temp_name, finish_with_full_expr, finish_with_prelude, for_post_label,
        index_assign_value_temp_name, index_value_temp_name, is_blank_identifier,
        is_pattern_binding_temp_name, match_scrutinee_temp_name, overwrite_target_temp_name,
        pattern_binding_env_key, print_temp_name, push_indented_lines, range_index_temp_name,
        range_source_temp_name, return_expr_temp_name, runtime_error_call, runtime_guard,
    },
    CExpr, CGenerator, CompileError,
};

impl<'a> CGenerator<'a> {
    pub(super) fn emit_stmt_with_env(
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
                let CExpr {
                    prelude,
                    code,
                    postlude,
                } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_full_expr(
                    prelude,
                    format!("{} {} = {};", ty.c_name(), c_ident(name), code),
                    postlude,
                ))
            }
            IrStmtKind::Assign { name, expr } => {
                let CExpr {
                    prelude,
                    code,
                    postlude,
                } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_full_expr(
                    prelude,
                    format!("{} = {};", c_assignment_target(name, env), code),
                    postlude,
                ))
            }
            IrStmtKind::FieldAssign { base, field, expr } => {
                let base = self.emit_assignment_target_expr(base, env)?;
                let expr = self.emit_stmt_expr_with_env(expr, env)?;
                let mut prelude = base.prelude;
                prelude.extend(expr.prelude);
                let mut postlude = expr.postlude;
                postlude.extend(base.postlude);
                Ok(finish_with_full_expr(
                    prelude,
                    format!("({}).{} = {};", base.code, c_field(field), expr.code),
                    postlude,
                ))
            }
            IrStmtKind::IndexAssign { base, index, expr } => {
                self.emit_index_assign_stmt(base, index, expr, env)
            }
            IrStmtKind::Overwrite { target, expr } => self.emit_overwrite_stmt(target, expr, env),
            IrStmtKind::Return { expr } => {
                let CExpr {
                    mut prelude,
                    code,
                    postlude,
                } = self.emit_stmt_expr_with_env(expr, env)?;
                if postlude.is_empty() {
                    return Ok(finish_with_prelude(prelude, format!("return {};", code)));
                }
                let temp = return_expr_temp_name(expr);
                prelude.push(format!("{} {temp} = {code};", expr.ty.c_name()));
                prelude.extend(postlude);
                Ok(finish_with_prelude(prelude, format!("return {temp};")))
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
            range @ IrStmtKind::RangeFor { .. } => self.emit_range_for_stmt(range, env),
            IrStmtKind::Drop { expr } => self.emit_drop_stmt(expr, env),
            IrStmtKind::Break => Ok("break;".to_string()),
            IrStmtKind::Continue => Ok(continue_label
                .map(|label| format!("goto {label};"))
                .unwrap_or_else(|| "continue;".to_string())),
            IrStmtKind::Match { scrutinee, arms } => {
                self.emit_match_stmt(scrutinee, arms, env, continue_label)
            }
            IrStmtKind::Assert {
                condition,
                source_id,
                offset,
            } => self.emit_assert_stmt(condition, *source_id, *offset, env),
            IrStmtKind::Expr { expr } => {
                if let IrExprKind::Call { callee, args } = &expr.kind {
                    if callee == "print" {
                        return self.emit_print(args, env);
                    }
                }

                let CExpr {
                    prelude,
                    code,
                    postlude,
                } = self.emit_stmt_expr_with_env(expr, env)?;
                let body = if expr.ty.needs_cleanup() {
                    format!("(void)({code});")
                } else {
                    format!("{code};")
                };
                Ok(finish_with_full_expr(prelude, body, postlude))
            }
        }
    }

    fn emit_assert_stmt(
        &self,
        condition: &IrExpr,
        source_id: usize,
        offset: usize,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let CExpr {
            prelude,
            mut code,
            postlude,
        } = self.emit_stmt_expr_with_env(condition, env)?;
        let mut output = String::new();
        for line in prelude {
            output.push_str(&line);
            output.push('\n');
        }
        if !postlude.is_empty() {
            let temp = condition_temp_name(condition);
            output.push_str(&format!("bool {temp} = {};\n", c_condition(&code)));
            for line in postlude {
                output.push_str(&line);
                output.push('\n');
            }
            code = temp;
        }
        output.push_str(&format!("if (!({})) {{\n", c_condition(&code)));
        push_indented_lines(
            &mut output,
            &format!("mallang_test_assertion_failed({source_id}, {offset});"),
            1,
        );
        output.push('}');
        Ok(output)
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
        let CExpr {
            prelude,
            mut code,
            postlude,
        } = self.emit_stmt_expr_with_env(condition, env)?;
        for line in prelude {
            output.push_str(&line);
            output.push('\n');
        }
        if !postlude.is_empty() {
            let temp = condition_temp_name(condition);
            output.push_str(&format!("bool {temp} = {};\n", c_condition(&code)));
            for line in postlude {
                output.push_str(&line);
                output.push('\n');
            }
            code = temp;
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

        let (prelude, code, postlude) = if let Some(condition) = condition {
            let CExpr {
                prelude,
                code,
                postlude,
            } = self.emit_stmt_expr_with_env(condition, env)?;
            (prelude, c_condition(&code), postlude)
        } else {
            (Vec::new(), "true".to_string(), Vec::new())
        };
        let mut output = String::new();

        if prelude.is_empty() && postlude.is_empty() {
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
        let code = if postlude.is_empty() {
            code
        } else {
            let condition = condition.expect("condition postlude requires condition");
            let temp = condition_temp_name(condition);
            push_indented_lines(&mut output, &format!("bool {temp} = {code};"), 1);
            for line in postlude {
                push_indented_lines(&mut output, &line, 1);
            }
            temp
        };
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
        range: &IrStmtKind,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        let IrStmtKind::RangeFor {
            index_name,
            value_name,
            source,
            element_ty,
            body,
            cleanup,
        } = range
        else {
            return Err(CompileError::new(
                "IR invariant violation: expected range statement",
            ));
        };
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

        let CExpr {
            prelude,
            code,
            mut postlude,
        } = self.emit_stmt_expr_with_env(source, env)?;
        if let Some((name, ty)) = full_expr_owner(source) {
            let owner_drop = format!("{}(&({}));", drop_fn_name(ty), c_ident(name));
            let Some(index) = postlude.iter().position(|line| line == &owner_drop) else {
                return Err(CompileError::new(
                    "IR invariant violation: range full-expression owner is missing cleanup",
                ));
            };
            postlude.remove(index);
        }
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
        output.push_str(&format!("(void)&{source_temp};\n"));
        for line in postlude {
            output.push_str(&line);
            output.push('\n');
        }
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
            push_indented_lines(&mut output, &format!("(void)&{value_ident};"), 1);
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
        for stmt in cleanup {
            output.push('\n');
            output.push_str(&self.emit_stmt_with_env(stmt, env)?);
        }
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
            let code = self.emit_for_init(init, env)?;
            push_indented_lines(&mut output, &code, 1);
        }

        let continue_label = post
            .filter(|_| contains_outer_continue(body))
            .map(for_post_label);
        output.push_str("    while (true) {\n");
        if let Some(condition) = condition {
            let CExpr {
                prelude,
                mut code,
                postlude,
            } = self.emit_stmt_expr_with_env(condition, env)?;
            for line in prelude {
                push_indented_lines(&mut output, &line, 2);
            }
            if !postlude.is_empty() {
                let temp = condition_temp_name(condition);
                push_indented_lines(
                    &mut output,
                    &format!("bool {temp} = {};", c_condition(&code)),
                    2,
                );
                for line in postlude {
                    push_indented_lines(&mut output, &line, 2);
                }
                code = temp;
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
            if let Some(label) = continue_label.as_deref() {
                push_indented_lines(&mut output, &format!("{label}: {{"), 2);
            } else {
                push_indented_lines(&mut output, "{", 2);
            }
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
    ) -> Result<String, CompileError> {
        match init {
            IrForInit::Let { name, ty, expr, .. } => {
                let CExpr {
                    prelude,
                    code,
                    postlude,
                } = self.emit_stmt_expr_with_env(expr, env)?;
                Ok(finish_with_full_expr(
                    prelude,
                    format!("{} {} = {};", ty.c_name(), c_ident(name), code),
                    postlude,
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
                let mut postlude = expr.postlude;
                postlude.extend(target.postlude);
                Ok(finish_with_full_expr(
                    prelude,
                    format!("{} = {};", target.code, expr.code),
                    postlude,
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
                let CExpr {
                    prelude,
                    code,
                    postlude,
                } = self.emit_stmt_expr_with_env(base, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!("({}).{}", code, c_field(field)),
                    postlude,
                })
            }
            IrExprKind::Index { base, index } => match &base.ty {
                Type::Array { len, .. } => {
                    let base = self.emit_stmt_expr_with_env(base, env)?;
                    let index = self.emit_stmt_expr_with_env(index, env)?;
                    let mut prelude = base.prelude;
                    prelude.extend(index.prelude);
                    let mut postlude = index.postlude;
                    postlude.extend(base.postlude);
                    Ok(CExpr {
                        prelude,
                        code: format!(
                            "({}).mlg_data[mallang_check_index({}, {len})]",
                            base.code, index.code
                        ),
                        postlude,
                    })
                }
                Type::Slice(_) => {
                    let base = self.emit_stmt_expr_with_env(base, env)?;
                    let index = self.emit_stmt_expr_with_env(index, env)?;
                    let index_temp = index_value_temp_name(target);
                    let mut prelude = base.prelude;
                    prelude.extend(index.prelude);
                    prelude.push(format!("int64_t {index_temp} = {};", index.code));
                    prelude.push(runtime_guard(
                        format!(
                            "{index_temp} < 0 || {index_temp} >= ({}).{}",
                            base.code,
                            c_field("len")
                        ),
                        "slice index out of bounds",
                    ));
                    let mut postlude = index.postlude;
                    postlude.extend(base.postlude);
                    Ok(CExpr {
                        prelude,
                        code: format!("({}).{}[{index_temp}]", base.code, c_field("data")),
                        postlude,
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
        let base_ty = base.ty.clone();
        let base = self.emit_assignment_target_expr(base, env)?;
        let index_temp = index_assign_value_temp_name(index);
        let index = self.emit_stmt_expr_with_env(index, env)?;
        let value = self.emit_stmt_expr_with_env(expr, env)?;

        let mut prelude = base.prelude;
        prelude.extend(index.prelude);
        prelude.push(format!("int64_t {index_temp} = {};", index.code));
        match &base_ty {
            Type::Array { len, .. } => {
                prelude.push(runtime_guard(
                    format!("{index_temp} < 0 || {index_temp} >= {len}"),
                    "array index out of bounds",
                ));
            }
            Type::Slice(_) => {
                prelude.push(runtime_guard(
                    format!(
                        "{index_temp} < 0 || {index_temp} >= ({}).{}",
                        base.code,
                        c_field("len")
                    ),
                    "slice index out of bounds",
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
        let mut postlude = value.postlude;
        postlude.extend(index.postlude);
        postlude.extend(base.postlude);

        Ok(finish_with_full_expr(
            prelude,
            format!(
                "({}).{data_field}[{index_temp}] = {};",
                base.code, value.code
            ),
            postlude,
        ))
    }

    fn emit_overwrite_stmt(
        &self,
        target: &IrExpr,
        expr: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<String, CompileError> {
        if !target.ty.needs_cleanup() || target.ty != expr.ty {
            return Err(CompileError::new(
                "IR invariant violation: overwrite requires one cleanup type",
            ));
        }
        if !matches!(expr.kind, IrExprKind::Var(_)) {
            return Err(CompileError::new(
                "IR invariant violation: overwrite value must be an evaluated temporary",
            ));
        }

        let target_expr = self.emit_assignment_target_expr(target, env)?;
        let value = self.emit_stmt_expr_with_env(expr, env)?;
        if !value.prelude.is_empty() || !value.postlude.is_empty() {
            return Err(CompileError::new(
                "IR invariant violation: overwrite value temporary cannot have cleanup code",
            ));
        }
        let target_temp = overwrite_target_temp_name(target);
        let mut prelude = target_expr.prelude;
        prelude.push(format!(
            "{} *{target_temp} = &({});",
            target.ty.c_name(),
            target_expr.code
        ));
        let body = format!(
            "{}({target_temp});\n*{target_temp} = {};",
            drop_fn_name(&target.ty),
            value.code
        );
        Ok(finish_with_full_expr(prelude, body, target_expr.postlude))
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
            let CExpr {
                prelude,
                code,
                postlude,
            } = self.emit_stmt_expr_with_env(scrutinee, env)?;
            for line in prelude {
                output.push_str(&line);
                output.push('\n');
            }
            let temp = match_scrutinee_temp_name(scrutinee);
            output.push_str(&format!("{} {temp} = {code};\n", scrutinee.ty.c_name()));
            for line in postlude {
                output.push_str(&line);
                output.push('\n');
            }
            temp
        };

        match &scrutinee.ty {
            Type::Option(_) | Type::Result(_, _) | Type::Enum(_) => self.emit_adt_match_stmt(
                &scrutinee.ty,
                &scrutinee_code,
                arms,
                env,
                continue_label,
                output,
            ),
            _ => Err(CompileError::new(
                "IR invariant violation: match on non-ADT value",
            )),
        }
    }

    fn emit_adt_match_stmt(
        &self,
        scrutinee_ty: &Type,
        scrutinee: &str,
        arms: &[IrMatchBlockArm],
        env: &HashMap<String, String>,
        continue_label: Option<&str>,
        mut output: String,
    ) -> Result<String, CompileError> {
        if arms.is_empty() {
            return Err(CompileError::new(
                "IR invariant violation: user enum match requires at least one arm",
            ));
        }

        for (index, arm) in arms.iter().enumerate() {
            let plan =
                self.plan_adt_pattern(&arm.pattern, scrutinee_ty, scrutinee, arm.span, env)?;
            if index == 0 {
                output.push_str(&format!("if ({}) {{\n", plan.condition));
            } else {
                output.push_str(&format!(" else if ({}) {{\n", plan.condition));
            }
            for line in &plan.setup {
                push_indented_lines(&mut output, line, 1);
            }
            self.emit_match_stmt_body(&arm.body, &plan.env, continue_label, &mut output)?;
            output.push('}');
        }
        output.push_str(" else {\n");
        push_indented_lines(&mut output, &runtime_error_call("invalid enum tag"), 1);
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
        let CExpr {
            prelude,
            code,
            postlude,
        } = self.emit_stmt_expr_with_env(arg, env)?;
        match &arg.ty {
            Type::Int => Ok(finish_with_full_expr(
                prelude,
                format!("printf(\"%lld\\n\", (long long)({code}));"),
                postlude,
            )),
            Type::Bool => Ok(finish_with_full_expr(
                prelude,
                format!("printf(\"%s\\n\", ({code}) ? \"true\" : \"false\");"),
                postlude,
            )),
            Type::String => Ok(finish_with_full_expr(
                prelude,
                format!("mallang_print_string({code});\nprintf(\"\\n\");"),
                postlude,
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
            Type::Function(_) => Err(CompileError::new(
                "IR invariant violation: cannot print function value",
            )),
            Type::Option(_) | Type::Result(_, _) | Type::Struct(_) => {
                self.emit_print_composite(arg, prelude, code, postlude)
            }
            Type::Enum(_) => self.emit_print_composite(arg, prelude, code, postlude),
        }
    }

    fn emit_print_composite(
        &self,
        arg: &IrExpr,
        mut prelude: Vec<String>,
        code: String,
        postlude: Vec<String>,
    ) -> Result<String, CompileError> {
        let temp = print_temp_name(arg);
        prelude.push(format!("{} {temp} = {code};", arg.ty.c_name()));

        let mut body = String::new();
        self.push_print_value_fragment(&arg.ty, &temp, &mut body, 0)?;
        push_indented_lines(&mut body, "printf(\"\\n\");", 0);

        Ok(finish_with_full_expr(prelude, body, postlude))
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
                push_indented_lines(output, &format!("mallang_print_string({code});"), level);
                Ok(())
            }
            Type::Option(inner) => {
                push_indented_lines(output, &format!("if (({code}).tag == 1) {{"), level);
                push_indented_lines(output, "printf(\"Some(\");", level + 1);
                self.push_print_value_fragment(
                    inner,
                    &format!(
                        "({code}).{}.{}",
                        c_field("payload"),
                        c_field("Some")
                    ),
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
                self.push_print_value_fragment(
                    ok,
                    &format!("({code}).{}.{}", c_field("payload"), c_field("Ok")),
                    output,
                    level + 1,
                )?;
                push_indented_lines(output, "printf(\")\");", level + 1);
                push_indented_lines(output, "} else {", level);
                push_indented_lines(output, "printf(\"Err(\");", level + 1);
                self.push_print_value_fragment(
                    err,
                    &format!("({code}).{}.{}", c_field("payload"), c_field("Err")),
                    output,
                    level + 1,
                )?;
                push_indented_lines(output, "printf(\")\");", level + 1);
                push_indented_lines(output, "}", level);
                Ok(())
            }
            Type::Struct(name) => {
                let struct_def = self.struct_def(name)?;
                let display_name = match struct_def.intrinsic {
                    Some(StandardType::Error) => "Error",
                    Some(StandardType::Map) => {
                        return Err(CompileError::new(
                            "IR invariant violation: cannot print opaque standard Map",
                        ));
                    }
                    Some(StandardType::ErrorKind) => {
                        return Err(CompileError::new(
                            "IR invariant violation: errors.Kind must use enum lowering",
                        ));
                    }
                    None => name,
                };
                push_indented_lines(
                    output,
                    &format!("printf(\"{}{{\");", display_name),
                    level,
                );
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
            Type::Enum(name) => {
                let enum_def = self.enum_def(name)?;
                if enum_def.intrinsic != Some(StandardType::ErrorKind)
                    || enum_def
                        .variants
                        .iter()
                        .any(|variant| !variant.payloads.is_empty())
                {
                    return Err(CompileError::new(
                        "IR invariant violation: cannot print user-defined enum",
                    ));
                }

                push_indented_lines(output, &format!("switch (({code}).tag) {{"), level);
                for (tag, variant) in enum_def.variants.iter().enumerate() {
                    push_indented_lines(output, &format!("case {tag}:"), level + 1);
                    push_indented_lines(
                        output,
                        &format!("printf(\"%s\", {});", c_string_literal(&variant.name)),
                        level + 2,
                    );
                    push_indented_lines(output, "break;", level + 2);
                }
                push_indented_lines(output, "default:", level + 1);
                push_indented_lines(output, &runtime_error_call("invalid enum tag"), level + 2);
                push_indented_lines(output, "}", level);
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
            Type::Function(_) => Err(CompileError::new(
                "IR invariant violation: cannot print function value",
            )),
        }
    }

    pub(super) fn emit_cleanup_stmts(
        &self,
        cleanup: &[IrStmt],
        env: &HashMap<String, String>,
    ) -> Result<Vec<String>, CompileError> {
        cleanup
            .iter()
            .map(|stmt| self.emit_stmt_with_env(stmt, env))
            .collect()
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

        let mut cleanup_env = None;
        if let IrExprKind::Var(name) = &expr.kind {
            let identity_key = pattern_binding_env_key(name, expr.span);
            if let Some(binding) = env.get(&identity_key) {
                let mut resolved = env.clone();
                resolved.insert(name.clone(), binding.clone());
                cleanup_env = Some(resolved);
            } else if env
                .get(name)
                .is_some_and(|binding| is_pattern_binding_temp_name(binding))
            {
                let mut resolved = env.clone();
                resolved.remove(name);
                cleanup_env = Some(resolved);
            }
        }
        let cleanup_env = cleanup_env.as_ref().unwrap_or(env);

        let CExpr {
            prelude,
            code,
            postlude,
        } = self.emit_borrow_lvalue_expr(expr, cleanup_env)?;
        if !postlude.is_empty() {
            return Err(CompileError::new(
                "IR invariant violation: drop target cannot own full-expression temporaries",
            ));
        }
        Ok(finish_with_prelude(
            prelude,
            format!("{}(&({code}));", drop_fn_name(&expr.ty)),
        ))
    }
}

fn full_expr_owner(expr: &IrExpr) -> Option<(&str, &Type)> {
    match &expr.kind {
        IrExprKind::FullExprTemporary { name, .. } if expr.ty.needs_cleanup() => {
            Some((name, &expr.ty))
        }
        IrExprKind::FieldAccess { base, .. } | IrExprKind::Index { base, .. } => {
            full_expr_owner(base)
        }
        _ => None,
    }
}

fn contains_outer_continue(stmts: &[IrStmt]) -> bool {
    stmts.iter().any(stmt_contains_outer_continue)
}

fn stmt_contains_outer_continue(stmt: &IrStmt) -> bool {
    match &stmt.kind {
        IrStmtKind::Continue => true,
        IrStmtKind::If {
            then_body,
            else_body,
            ..
        } => contains_outer_continue(then_body) || contains_outer_continue(else_body),
        IrStmtKind::Match { arms, .. } => arms.iter().any(|arm| contains_outer_continue(&arm.body)),
        IrStmtKind::Let { .. }
        | IrStmtKind::Assign { .. }
        | IrStmtKind::FieldAssign { .. }
        | IrStmtKind::IndexAssign { .. }
        | IrStmtKind::Overwrite { .. }
        | IrStmtKind::Return { .. }
        | IrStmtKind::For { .. }
        | IrStmtKind::RangeFor { .. }
        | IrStmtKind::Drop { .. }
        | IrStmtKind::Assert { .. }
        | IrStmtKind::Break
        | IrStmtKind::Expr { .. } => false,
    }
}
