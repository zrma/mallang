use std::collections::HashMap;

use crate::{
    ast::{ArgMode, BinaryOp, UnaryOp},
    ir::{IrArg, IrExpr, IrExprKind, IrMatchArm},
    semantic::Type,
};

use super::{
    names::{
        c_arg_code, c_assignment_target, c_binary_expr, c_field, c_ident, c_string,
        callable_thunk_name, closure_call_name, closure_drop_name, closure_env_type_name,
        empty_slice_value_code, COperator, TypeCName,
    },
    utils::{
        callable_temp_name, checked_binary_left_temp_name, checked_binary_result_temp_name,
        checked_binary_right_temp_name, checked_int_binary_builtin,
        checked_unary_operand_temp_name, checked_unary_result_temp_name, closure_env_temp_name,
        dividend_temp_name, divisor_temp_name, if_expr_temp_block, if_expr_temp_name,
        index_source_temp_name, index_value_temp_name, match_expr_temp_name,
        match_scrutinee_temp_name, push_indented_lines, runtime_error_call, runtime_guard,
        slice_append_temp_name, slice_field_take_temp_name, slice_literal_temp_name,
    },
    AppendSourceExpr, CExpr, CGenerator, CompileError,
};

impl<'a> CGenerator<'a> {
    pub(super) fn emit_stmt_expr_with_env(
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
            IrExprKind::FunctionValue { function } => {
                self.emit_function_value_stmt_expr(expr, function)
            }
            IrExprKind::ClosureValue { closure, captures } => {
                self.emit_closure_value_stmt_expr(expr, closure, captures, env)
            }
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
            IrExprKind::VariantConstructor { variant, payload } => {
                self.emit_variant_constructor_stmt_expr(&expr.ty, variant, payload.as_deref(), env)
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
            IrExprKind::SliceFieldTake { source } => {
                self.emit_slice_field_take_stmt_expr(expr, source, env)
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
            IrExprKind::IndirectCall { callee, args } => {
                self.emit_indirect_call_stmt_expr(expr, callee, args, env)
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

    fn emit_function_value_stmt_expr(
        &self,
        expr: &IrExpr,
        function: &str,
    ) -> Result<CExpr, CompileError> {
        if !matches!(expr.ty, Type::Function(_)) {
            return Err(CompileError::new(
                "IR invariant violation: function value must have function type",
            ));
        }
        let function_def = self.function_def(function)?;
        if expr.ty != Self::callable_type(function_def) {
            return Err(CompileError::new(
                "IR invariant violation: named function value signature mismatch",
            ));
        }
        Ok(CExpr::simple(format!(
            "({}){{ .mlg_env = NULL, .mlg_drop = NULL, .mlg_call = {} }}",
            expr.ty.c_name(),
            callable_thunk_name(function)
        )))
    }

    fn emit_closure_value_stmt_expr(
        &self,
        expr: &IrExpr,
        closure_name: &str,
        captures: &[crate::ir::IrClosureCaptureValue],
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let closure = self.closure_def(closure_name)?;
        if expr.ty != Self::closure_callable_type(closure)
            || captures.len() != closure.captures.len()
        {
            return Err(CompileError::new(
                "IR invariant violation: closure value signature mismatch",
            ));
        }
        if captures.is_empty() {
            return Ok(CExpr::simple(format!(
                "({}){{ .mlg_env = NULL, .mlg_drop = NULL, .mlg_call = {} }}",
                expr.ty.c_name(),
                closure_call_name(closure_name)
            )));
        }

        let env_type = closure_env_type_name(closure_name);
        let env_temp = closure_env_temp_name(expr);
        let mut prelude = vec![format!(
            "{env_type} *{env_temp} = malloc(sizeof({env_type}));"
        )];
        prelude.push(runtime_guard(
            format!("{env_temp} == NULL"),
            "closure environment allocation failed",
        ));
        for (capture, expected) in captures.iter().zip(closure.captures.iter()) {
            if capture.name != expected.name || capture.expr.ty != expected.ty {
                return Err(CompileError::new(
                    "IR invariant violation: closure capture mismatch",
                ));
            }
            let emitted = self.emit_stmt_expr_with_env(&capture.expr, env)?;
            prelude.extend(emitted.prelude);
            prelude.push(format!(
                "{env_temp}->{} = {};",
                c_field(&capture.name),
                emitted.code
            ));
        }

        Ok(CExpr {
            prelude,
            code: format!(
                "({}){{ .mlg_env = {env_temp}, .mlg_drop = {}, .mlg_call = {} }}",
                expr.ty.c_name(),
                closure_drop_name(closure_name),
                closure_call_name(closure_name)
            ),
        })
    }

    fn emit_indirect_call_stmt_expr(
        &self,
        expr: &IrExpr,
        callee: &IrExpr,
        args: &[IrArg],
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let Type::Function(function) = &callee.ty else {
            return Err(CompileError::new(
                "IR invariant violation: indirect call target must have function type",
            ));
        };
        if &expr.ty != function.return_type.as_ref() || args.len() != function.params.len() {
            return Err(CompileError::new(
                "IR invariant violation: indirect call signature mismatch",
            ));
        }

        let callable_type = callee.ty.c_name();
        let callee = self.emit_stmt_expr_with_env(callee, env)?;
        let temp = callable_temp_name(expr);
        let mut prelude = callee.prelude;
        prelude.push(format!("{callable_type} {temp} = {};", callee.code));
        let mut arg_codes = Vec::new();
        for (arg, param) in args.iter().zip(function.params.iter()) {
            let mode_matches = matches!(
                (arg.mode, param.mode),
                (ArgMode::Owned, crate::ast::ParamMode::Owned)
                    | (ArgMode::Con, crate::ast::ParamMode::Con)
                    | (ArgMode::Mut, crate::ast::ParamMode::Mut)
            );
            if !mode_matches || arg.expr.ty != param.ty {
                return Err(CompileError::new(
                    "IR invariant violation: indirect call argument mismatch",
                ));
            }
            let emitted = self.emit_call_arg_stmt_expr(arg, env)?;
            prelude.extend(emitted.prelude);
            arg_codes.push(emitted.code);
        }

        let mut call_args = vec![format!("{temp}.mlg_env")];
        call_args.extend(arg_codes);
        Ok(CExpr {
            prelude,
            code: format!("{temp}.mlg_call({})", call_args.join(", ")),
        })
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
            prelude.push(runtime_guard(
                format!("__builtin_sub_overflow((int64_t)0, {operand_temp}, &{result_temp})"),
                "integer overflow",
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
            prelude.push(runtime_guard(
                format!("{builtin}({left_temp}, {right_temp}, &{result_temp})"),
                "integer overflow",
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
            prelude.push(runtime_guard(
                format!("{divisor_temp} == 0"),
                "division by zero",
            ));
            prelude.push(runtime_guard(
                format!("{dividend_temp} == INT64_MIN && {divisor_temp} == -1"),
                "integer overflow",
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
                prelude.push(runtime_guard(
                    format!("{index_temp} < 0 || {index_temp} >= {len}"),
                    "array index out of bounds",
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
                prelude.push(runtime_guard(
                    format!(
                        "{index_temp} < 0 || {index_temp} >= ({source_temp}).{}",
                        c_field("len")
                    ),
                    "slice index out of bounds",
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

    fn emit_slice_field_take_stmt_expr(
        &self,
        expr: &IrExpr,
        source: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        if !matches!(expr.ty, Type::Slice(_)) || source.ty != expr.ty {
            return Err(CompileError::new(
                "IR invariant violation: slice field take source must be a slice",
            ));
        }

        let CExpr { mut prelude, code } = self.emit_borrow_lvalue_expr(source, env)?;
        let temp = slice_field_take_temp_name(expr);
        let empty = empty_slice_value_code(&expr.ty).ok_or_else(|| {
            CompileError::new("IR invariant violation: slice field take source must be a slice")
        })?;
        prelude.push(format!("{} {temp} = {code};", expr.ty.c_name()));
        prelude.push(format!("{code} = {empty};"));

        Ok(CExpr {
            prelude,
            code: temp,
        })
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

        let slice = self.emit_slice_append_source_stmt_expr(slice, env)?;
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
        if let Some(clear_source) = slice.clear_source {
            prelude.push(clear_source);
        }
        prelude.push(runtime_guard(
            format!("{temp}.{len_field} == INT64_MAX"),
            "slice length overflow",
        ));
        prelude.push(format!("int64_t {new_len} = {temp}.{len_field} + 1;"));
        prelude.push(format!(
            "if ({temp}.{cap_field} < {new_len}) {{\n    int64_t {new_cap} = ({temp}.{cap_field} == 0) ? 1 : {temp}.{cap_field};\n    while ({new_cap} < {new_len}) {{\n        if ({new_cap} > INT64_MAX / 2) {{\n            {new_cap} = {new_len};\n            break;\n        }}\n        {new_cap} = {new_cap} * 2;\n    }}\n    if ((uint64_t){new_cap} > UINT64_MAX / sizeof({element_ty})) {{\n        {allocation_size_error}\n    }}\n    void *{data_temp} = realloc({temp}.{data_field}, sizeof({element_ty}) * (uint64_t){new_cap});\n    if ({data_temp} == NULL) {{\n        {allocation_failed_error}\n    }}\n    {temp}.{data_field} = {data_temp};\n    {temp}.{cap_field} = {new_cap};\n}}",
            allocation_size_error = runtime_error_call("slice allocation size overflow"),
            allocation_failed_error = runtime_error_call("slice allocation failed"),
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

    fn emit_slice_append_source_stmt_expr(
        &self,
        slice: &IrExpr,
        env: &HashMap<String, String>,
    ) -> Result<AppendSourceExpr, CompileError> {
        if matches!(slice.kind, IrExprKind::FieldAccess { .. }) {
            let CExpr { prelude, code } = self.emit_borrow_lvalue_expr(slice, env)?;
            let clear_source = format!(
                "{code} = {};",
                empty_slice_value_code(&slice.ty).ok_or_else(|| {
                    CompileError::new("IR invariant violation: append field source must be a slice")
                })?
            );
            return Ok(AppendSourceExpr {
                prelude,
                code,
                clear_source: Some(clear_source),
            });
        }

        let CExpr { prelude, code } = self.emit_stmt_expr_with_env(slice, env)?;
        Ok(AppendSourceExpr {
            prelude,
            code,
            clear_source: None,
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

    pub(super) fn emit_borrow_lvalue_expr(
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
                    prelude.push(runtime_guard(
                        format!(
                            "{index_temp} < 0 || {index_temp} >= ({}).{}",
                            base.code,
                            c_field("len")
                        ),
                        "slice index out of bounds",
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

    fn emit_variant_constructor_stmt_expr(
        &self,
        ty: &Type,
        variant_name: &str,
        payload: Option<&IrExpr>,
        env: &HashMap<String, String>,
    ) -> Result<CExpr, CompileError> {
        let (tag, expected_payload) = self.adt_variant(ty, variant_name)?;
        match (expected_payload, payload) {
            (None, None) => Ok(CExpr::simple(format!(
                "({}){{ .tag = {tag} }}",
                ty.c_name()
            ))),
            (Some(expected), Some(payload)) if expected == &payload.ty => {
                let CExpr { prelude, code } = self.emit_stmt_expr_with_env(payload, env)?;
                Ok(CExpr {
                    prelude,
                    code: format!(
                        "({}){{ .tag = {tag}, .{} = {{ .{} = {code} }} }}",
                        ty.c_name(),
                        c_field("payload"),
                        c_field(variant_name)
                    ),
                })
            }
            _ => Err(CompileError::new(
                "IR invariant violation: ADT constructor payload mismatch",
            )),
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
                let data_field = c_field("data");
                let len_field = c_field("len");
                let cap_field = c_field("cap");
                if elements.is_empty() {
                    prelude.push(format!("{temp}.{data_field} = NULL;"));
                    prelude.push(format!("{temp}.{len_field} = 0;"));
                    prelude.push(format!("{temp}.{cap_field} = 0;"));
                    return Ok(CExpr {
                        prelude,
                        code: temp,
                    });
                }

                let element_ty = element.c_name();
                prelude.push(format!(
                    "if ((uint64_t){} > UINT64_MAX / sizeof({element_ty})) {{\n    {allocation_size_error}\n}}",
                    elements.len(),
                    allocation_size_error = runtime_error_call("slice allocation size overflow"),
                ));
                prelude.push(format!(
                    "{temp}.{data_field} = malloc(sizeof({element_ty}) * {});",
                    elements.len()
                ));
                prelude.push(runtime_guard(
                    format!("{temp}.{data_field} == NULL"),
                    "slice allocation failed",
                ));
                prelude.push(format!("{temp}.{len_field} = {};", elements.len()));
                prelude.push(format!("{temp}.{cap_field} = {};", elements.len()));

                for (index, element) in elements.iter().enumerate() {
                    let emitted = self.emit_stmt_expr_with_env(element, env)?;
                    prelude.extend(emitted.prelude);
                    prelude.push(format!("{temp}.{data_field}[{index}] = {};", emitted.code));
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
            Type::Option(_) | Type::Result(_, _) | Type::Enum(_) => self.emit_adt_match_stmt_expr(
                expr,
                &scrutinee.ty,
                &scrutinee_code,
                arms,
                env,
                prelude,
            ),
            _ => Err(CompileError::new(
                "IR invariant violation: match on non-ADT value",
            )),
        }
    }

    fn emit_adt_match_stmt_expr(
        &self,
        expr: &IrExpr,
        scrutinee_ty: &Type,
        scrutinee: &str,
        arms: &[IrMatchArm],
        env: &HashMap<String, String>,
        mut prelude: Vec<String>,
    ) -> Result<CExpr, CompileError> {
        if arms.is_empty() {
            return Err(CompileError::new(
                "IR invariant violation: user enum match requires at least one arm",
            ));
        }

        let temp = match_expr_temp_name(expr);
        let mut block = String::new();
        prelude.push(format!("{} {temp};", expr.ty.c_name()));
        for (index, arm) in arms.iter().enumerate() {
            let plan = self.plan_adt_pattern(&arm.pattern, scrutinee_ty, scrutinee, env)?;
            let emitted = self.emit_stmt_expr_with_env(&arm.expr, &plan.env)?;
            let cleanup = self.emit_cleanup_stmts(&arm.cleanup, &plan.env)?;
            if index == 0 {
                block.push_str(&format!("if ({}) {{\n", plan.condition));
            } else {
                block.push_str(&format!(" else if ({}) {{\n", plan.condition));
            }
            for line in emitted.prelude {
                push_indented_lines(&mut block, &line, 1);
            }
            push_indented_lines(&mut block, &format!("{temp} = {};", emitted.code), 1);
            for line in cleanup {
                push_indented_lines(&mut block, &line, 1);
            }
            block.push('}');
        }
        block.push_str(" else {\n");
        push_indented_lines(&mut block, &runtime_error_call("invalid enum tag"), 1);
        block.push('}');
        prelude.push(block);

        Ok(CExpr {
            prelude,
            code: temp,
        })
    }
}
