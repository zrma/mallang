use std::collections::HashMap;

use crate::{
    ast::{ArgMode, BinaryOp, ParamMode, UnaryOp},
    ir::IrParam,
    semantic::Type,
};

pub(super) trait TypeCName {
    fn c_name(&self) -> String;
    fn c_param_type(&self, mode: ParamMode) -> String;
}

impl TypeCName for Type {
    fn c_name(&self) -> String {
        match self {
            Self::Int => "int64_t".to_string(),
            Self::Bool => "bool".to_string(),
            Self::String => "mlg_String".to_string(),
            Self::Unit => "void".to_string(),
            Self::Option(_) | Self::Result(_, _) => format!("mlg_{}", mangle_type(self)),
            Self::Array { .. } | Self::Slice(_) => format!("mlg_{}", mangle_type(self)),
            Self::Struct(name) => format!("mlg_struct_{}", c_type_ident(name)),
            Self::Enum(name) => format!("mlg_enum_{}", c_type_ident(name)),
            Self::Function(_) => format!("mlg_{}", mangle_type(self)),
        }
    }

    fn c_param_type(&self, mode: ParamMode) -> String {
        match mode {
            ParamMode::Owned => self.c_name(),
            ParamMode::Con => match self {
                Self::Unit => "const void *".to_string(),
                _ => format!("const {} *", self.c_name()),
            },
            ParamMode::Mut => match self {
                Self::Unit => "void *".to_string(),
                _ => format!("{} *", self.c_name()),
            },
        }
    }
}

pub(super) trait COperator {
    fn c_operator(self) -> &'static str;
}

impl COperator for UnaryOp {
    fn c_operator(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

impl COperator for BinaryOp {
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

pub(super) fn c_param_decl(param: &IrParam) -> String {
    format!(
        "{} {}",
        param.ty.c_param_type(param.mode),
        c_ident(&param.name)
    )
}

pub(super) fn c_assignment_target(name: &str, env: &HashMap<String, String>) -> String {
    env.get(name).cloned().unwrap_or_else(|| c_ident(name))
}

pub(super) fn c_arg_code(mode: ArgMode, code: String) -> String {
    match mode {
        ArgMode::Owned => code,
        ArgMode::Con | ArgMode::Mut => format!("&({code})"),
    }
}

pub(super) fn c_condition(code: &str) -> String {
    strip_enclosing_parens(code).unwrap_or(code).to_string()
}

pub(super) fn c_binary_expr(
    op: BinaryOp,
    result_ty: &Type,
    operand_ty: &Type,
    left: String,
    right: String,
) -> String {
    if matches!(operand_ty, Type::String) && matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
        let negation = match op {
            BinaryOp::Equal => "",
            BinaryOp::NotEqual => "!",
            _ => unreachable!("string comparison only supports equality operators"),
        };
        return format!("({negation}mallang_string_equal({left}, {right}))");
    }

    debug_assert!(!matches!(result_ty, Type::String));
    format!("({left} {} {right})", op.c_operator())
}

pub(super) fn mangle_type(ty: &Type) -> String {
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
        Type::Enum(name) => format!("Enum_{}", c_type_ident(name)),
        Type::Function(function) => {
            let mutable = if function.mutable { "mut" } else { "con" };
            let params = function
                .params
                .iter()
                .map(|param| {
                    let mode = match param.mode {
                        ParamMode::Owned => "owned",
                        ParamMode::Con => "con",
                        ParamMode::Mut => "mut",
                    };
                    format!("{mode}_{}", mangle_type(&param.ty))
                })
                .collect::<Vec<_>>()
                .join("_");
            format!(
                "Function_{mutable}_{params}_ret_{}",
                mangle_type(&function.return_type)
            )
        }
    }
}

pub(super) fn drop_fn_name(ty: &Type) -> String {
    format!("mlg_drop_{}", mangle_type(ty))
}

pub(super) fn enum_node_type_name(name: &str) -> String {
    format!("{}_node", Type::Enum(name.to_string()).c_name())
}

pub(super) fn map_entry_type_name(ty: &Type) -> String {
    format!("{}_entry", ty.c_name())
}

pub(super) fn variant_payload_field(index: usize) -> String {
    c_field(&format!("payload_{index}"))
}

pub(super) fn variant_payload_member(variant: &str, payload_count: usize, index: usize) -> String {
    let mut member = format!("{}.{}", c_field("payload"), c_field(variant));
    if payload_count > 1 {
        member.push('.');
        member.push_str(&variant_payload_field(index));
    }
    member
}

pub(super) fn callable_thunk_name(function: &str) -> String {
    format!("mallang_callable_thunk_{}", c_ident(function))
}

pub(super) fn closure_env_type_name(closure: &str) -> String {
    format!("mallang_{}_env", c_type_ident(closure))
}

pub(super) fn closure_call_name(closure: &str) -> String {
    format!("mallang_{}_call", c_type_ident(closure))
}

pub(super) fn closure_drop_name(closure: &str) -> String {
    format!("mallang_{}_drop", c_type_ident(closure))
}

pub(super) fn c_ident(name: &str) -> String {
    if name == "main" {
        return name.to_string();
    }
    format!("mlg_{}", c_type_ident(name))
}

pub(super) fn c_field(name: &str) -> String {
    format!("mlg_{name}")
}

pub(super) fn empty_slice_value_code(ty: &Type) -> Option<String> {
    if !matches!(ty, Type::Slice(_)) {
        return None;
    }
    Some(format!(
        "({}){{ .{} = NULL, .{} = 0, .{} = 0 }}",
        ty.c_name(),
        c_field("data"),
        c_field("len"),
        c_field("cap")
    ))
}

pub(super) fn c_string_literal(value: &str) -> String {
    let mut literal = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => literal.push_str("\\\\"),
            '"' => literal.push_str("\\\""),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            _ => literal.push(ch),
        }
    }
    literal.push('"');
    literal
}

pub(super) fn c_string(value: &str) -> String {
    let literal = c_string_literal(value);
    format!(
        "(mlg_String){{ .{} = {literal}, .{} = {}, .{} = MLG_STRING_STATIC }}",
        c_field("data"),
        c_field("len"),
        value.len(),
        c_field("storage")
    )
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
