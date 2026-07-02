use std::collections::HashMap;

use crate::{
    ir::{
        IrArg, IrExpr, IrExprKind, IrForInit, IrForPost, IrMatchBlockArm, IrMatchPattern, IrStmt,
        IrStmtKind,
    },
    semantic::Type,
};

use super::{
    names::{c_assignment_target, c_condition, c_field, c_ident, drop_fn_name, TypeCName},
    utils::{
        finish_with_prelude, for_post_label, index_assign_value_temp_name, index_value_temp_name,
        is_blank_identifier, match_scrutinee_temp_name, print_temp_name, push_indented_lines,
        range_index_temp_name, range_source_temp_name,
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
                prelude.push(format!(
                    "if ({index_temp} < 0 || {index_temp} >= {len}) {{\n    fprintf(stderr, \"mallang runtime error: array index out of bounds\\n\");\n    exit(1);\n}}"
                ));
            }
            Type::Slice(_) => {
                prelude.push(format!(
                    "if ({index_temp} < 0 || {index_temp} >= ({}).{}) {{\n    fprintf(stderr, \"mallang runtime error: slice index out of bounds\\n\");\n    exit(1);\n}}",
                    base.code,
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
            format!(
                "({}).{data_field}[{index_temp}] = {};",
                base.code, value.code
            ),
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

        let CExpr { prelude, code } = self.emit_borrow_lvalue_expr(expr, env)?;
        Ok(finish_with_prelude(
            prelude,
            format!("{}(&({code}));", drop_fn_name(&expr.ty)),
        ))
    }
}
