use std::collections::HashMap;

use crate::{
    ast::{BinaryOp, ParamMode},
    ir::{IrExpr, IrForPost, IrFunction},
};

use super::{
    names::{c_condition, c_ident},
    CExpr,
};

pub(super) fn finish_with_prelude(prelude: Vec<String>, body: String) -> String {
    let mut output = String::new();
    for line in prelude {
        output.push_str(&line);
        output.push('\n');
    }
    output.push_str(&body);
    output
}

pub(super) fn finish_with_full_expr(
    prelude: Vec<String>,
    body: String,
    postlude: Vec<String>,
) -> String {
    let mut output = finish_with_prelude(prelude, body);
    for line in postlude {
        output.push('\n');
        output.push_str(&line);
    }
    output
}

pub(super) fn runtime_error_call(message: &str) -> String {
    format!("mallang_runtime_error(\"{message}\");")
}

pub(super) fn runtime_guard(condition: impl AsRef<str>, message: &str) -> String {
    format!(
        "if ({}) {{\n    {}\n}}",
        condition.as_ref(),
        runtime_error_call(message)
    )
}

pub(super) fn if_expr_temp_block(
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
    for line in then_expr.postlude {
        push_indented_lines(&mut output, &line, 1);
    }
    for stmt in then_cleanup {
        push_indented_lines(&mut output, &stmt, 1);
    }
    output.push_str("} else {\n");
    for line in else_expr.prelude {
        push_indented_lines(&mut output, &line, 1);
    }
    push_indented_lines(&mut output, &format!("{temp} = {};", else_expr.code), 1);
    for line in else_expr.postlude {
        push_indented_lines(&mut output, &line, 1);
    }
    for stmt in else_cleanup {
        push_indented_lines(&mut output, &stmt, 1);
    }
    output.push('}');
    output
}

pub(super) fn if_expr_temp_name(expr: &IrExpr) -> String {
    format!("mallang_if_tmp_{}", expr.span.start)
}

pub(super) fn if_condition_temp_name(expr: &IrExpr) -> String {
    format!("mallang_if_condition_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn logical_temp_name(expr: &IrExpr) -> String {
    format!("mallang_logical_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn match_expr_temp_name(expr: &IrExpr) -> String {
    format!("mallang_match_value_tmp_{}", expr.span.start)
}

pub(super) fn print_temp_name(expr: &IrExpr) -> String {
    format!("mallang_print_tmp_{}", expr.span.start)
}

pub(super) fn slice_literal_temp_name(expr: &IrExpr) -> String {
    format!("mallang_slice_tmp_{}", expr.span.start)
}

pub(super) fn variant_constructor_temp_name(expr: &IrExpr) -> String {
    format!("mallang_variant_tmp_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn variant_payload_temp_name(expr: &IrExpr, index: usize) -> String {
    format!(
        "mallang_variant_payload_{}_{}_{}",
        expr.span.start, expr.span.end, index
    )
}

pub(super) fn index_source_temp_name(expr: &IrExpr) -> String {
    format!("mallang_index_src_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn index_value_temp_name(expr: &IrExpr) -> String {
    format!("mallang_index_value_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn index_assign_value_temp_name(expr: &IrExpr) -> String {
    format!(
        "mallang_index_assign_value_{}_{}",
        expr.span.start, expr.span.end
    )
}

pub(super) fn overwrite_target_temp_name(expr: &IrExpr) -> String {
    format!(
        "mallang_overwrite_target_{}_{}",
        expr.span.start, expr.span.end
    )
}

pub(super) fn checked_unary_operand_temp_name(expr: &IrExpr) -> String {
    format!("mallang_checked_unary_operand_{}", expr.span.start)
}

pub(super) fn checked_unary_result_temp_name(expr: &IrExpr) -> String {
    format!("mallang_checked_unary_result_{}", expr.span.start)
}

pub(super) fn checked_binary_left_temp_name(expr: &IrExpr) -> String {
    format!("mallang_checked_left_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn checked_binary_right_temp_name(expr: &IrExpr) -> String {
    format!(
        "mallang_checked_right_{}_{}",
        expr.span.start, expr.span.end
    )
}

pub(super) fn checked_binary_result_temp_name(expr: &IrExpr) -> String {
    format!(
        "mallang_checked_result_{}_{}",
        expr.span.start, expr.span.end
    )
}

pub(super) fn checked_int_binary_builtin(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Add => Some("__builtin_add_overflow"),
        BinaryOp::Subtract => Some("__builtin_sub_overflow"),
        BinaryOp::Multiply => Some("__builtin_mul_overflow"),
        _ => None,
    }
}

pub(super) fn dividend_temp_name(expr: &IrExpr) -> String {
    format!("mallang_dividend_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn divisor_temp_name(expr: &IrExpr) -> String {
    format!("mallang_divisor_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn param_env(function: &IrFunction) -> HashMap<String, String> {
    param_env_from_params(&function.params)
}

pub(super) fn param_env_from_params(params: &[crate::ir::IrParam]) -> HashMap<String, String> {
    params
        .iter()
        .filter(|param| !matches!(param.mode, ParamMode::Owned))
        .map(|param| (param.name.clone(), format!("(*{})", c_ident(&param.name))))
        .collect()
}

pub(super) fn push_indented_lines(output: &mut String, code: &str, level: usize) {
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

pub(super) fn match_scrutinee_temp_name(expr: &IrExpr) -> String {
    format!("mallang_match_tmp_{}", expr.span.start)
}

pub(super) fn for_post_label(post: &IrForPost) -> String {
    match post {
        IrForPost::Assign { target, .. } => format!("mallang_for_post_{}", target.span.start),
    }
}

pub(super) fn range_source_temp_name(expr: &IrExpr) -> String {
    format!("mallang_range_src_{}", expr.span.start)
}

pub(super) fn condition_temp_name(expr: &IrExpr) -> String {
    format!("mallang_condition_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn return_expr_temp_name(expr: &IrExpr) -> String {
    format!("mallang_return_expr_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn range_index_temp_name(expr: &IrExpr) -> String {
    format!("mallang_range_index_{}", expr.span.start)
}

pub(super) fn slice_append_temp_name(expr: &IrExpr) -> String {
    format!("mallang_slice_append_tmp_{}", expr.span.start)
}

pub(super) fn slice_field_take_temp_name(expr: &IrExpr) -> String {
    format!("mallang_slice_take_tmp_{}", expr.span.start)
}

pub(super) fn callable_temp_name(expr: &IrExpr) -> String {
    format!("mallang_callable_tmp_{}_{}", expr.span.start, expr.span.end)
}

pub(super) fn closure_env_temp_name(expr: &IrExpr) -> String {
    format!(
        "mallang_closure_env_tmp_{}_{}",
        expr.span.start, expr.span.end
    )
}

pub(super) fn is_blank_identifier(name: &str) -> bool {
    name == "_"
}
