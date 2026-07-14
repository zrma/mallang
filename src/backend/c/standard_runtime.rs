use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ir::{IrExpr, IrExprKind, IrForInit, IrForPost, IrProgram, IrStmt, IrStmtKind},
    semantic::{FunctionType, Type},
    standard::{StandardIntrinsic, StandardType},
};

use super::{
    names::{c_field, callable_thunk_name, TypeCName},
    CompileError,
};

pub(super) fn intrinsic_helper_name(intrinsic: StandardIntrinsic) -> Option<&'static str> {
    match intrinsic {
        StandardIntrinsic::StringsByteLen => Some("mallang_std_strings_byte_len"),
        StandardIntrinsic::StringsScalarCount => Some("mallang_std_strings_scalar_count"),
        StandardIntrinsic::StringsContains => Some("mallang_std_strings_contains"),
        StandardIntrinsic::StringsFind => Some("mallang_std_strings_find"),
        StandardIntrinsic::StringsSplit => Some("mallang_std_strings_split"),
        StandardIntrinsic::StringsJoin => Some("mallang_std_strings_join"),
        StandardIntrinsic::StringsFromInt => Some("mallang_std_strings_from_int"),
        StandardIntrinsic::StringsParseInt => Some("mallang_std_strings_parse_int"),
        StandardIntrinsic::StringsFromBool => Some("mallang_std_strings_from_bool"),
        StandardIntrinsic::StringsParseBool => Some("mallang_std_strings_parse_bool"),
        _ => None,
    }
}

pub(super) fn emit_standard_runtime(program: &IrProgram) -> Result<String, CompileError> {
    let used = standard_uses(program);
    if used
        .intrinsics
        .iter()
        .all(|intrinsic| intrinsic_helper_name(*intrinsic).is_none())
    {
        return Ok(String::new());
    }

    let mut output = String::new();
    let needs_find = [
        StandardIntrinsic::StringsContains,
        StandardIntrinsic::StringsFind,
        StandardIntrinsic::StringsSplit,
    ]
    .iter()
    .any(|intrinsic| used.intrinsics.contains(intrinsic));
    if needs_find {
        output.push_str(BYTE_FIND_HELPER);
    }
    if used.intrinsics.contains(&StandardIntrinsic::StringsByteLen) {
        output.push_str(BYTE_LEN_HELPER);
    }
    if used
        .intrinsics
        .contains(&StandardIntrinsic::StringsScalarCount)
    {
        output.push_str(SCALAR_COUNT_HELPER);
    }
    if used
        .intrinsics
        .contains(&StandardIntrinsic::StringsContains)
    {
        output.push_str(CONTAINS_HELPER);
    }
    if used.intrinsics.contains(&StandardIntrinsic::StringsFind) {
        output.push_str(&render(
            FIND_HELPER,
            &[
                ("<OPTION_INT>", Type::Option(Box::new(Type::Int)).c_name()),
                ("<FIELD_PAYLOAD>", c_field("payload")),
                ("<FIELD_SOME>", c_field("Some")),
            ],
        ));
    }
    if used.intrinsics.contains(&StandardIntrinsic::StringsSplit) {
        output.push_str(&render(
            SPLIT_HELPER,
            &[
                (
                    "<SLICE_STRING>",
                    Type::Slice(Box::new(Type::String)).c_name(),
                ),
                ("<FIELD_DATA>", c_field("data")),
                ("<FIELD_LEN>", c_field("len")),
                ("<FIELD_CAP>", c_field("cap")),
            ],
        ));
    }
    if used.intrinsics.contains(&StandardIntrinsic::StringsJoin) {
        output.push_str(&render(
            JOIN_HELPER,
            &[
                (
                    "<SLICE_STRING>",
                    Type::Slice(Box::new(Type::String)).c_name(),
                ),
                ("<FIELD_DATA>", c_field("data")),
                ("<FIELD_LEN>", c_field("len")),
                ("<FIELD_CAP>", c_field("cap")),
            ],
        ));
    }
    if used.intrinsics.contains(&StandardIntrinsic::StringsFromInt) {
        output.push_str(FROM_INT_HELPER);
    }
    if used
        .intrinsics
        .contains(&StandardIntrinsic::StringsFromBool)
    {
        output.push_str(FROM_BOOL_HELPER);
    }
    if used
        .intrinsics
        .contains(&StandardIntrinsic::StringsParseInt)
        || used
            .intrinsics
            .contains(&StandardIntrinsic::StringsParseBool)
    {
        output.push_str(&emit_error_helper(program)?);
    }
    if used
        .intrinsics
        .contains(&StandardIntrinsic::StringsParseInt)
    {
        let error = standard_error_type(program)?;
        output.push_str(&render(
            PARSE_INT_HELPER,
            &[
                (
                    "<RESULT_INT_ERROR>",
                    Type::Result(Box::new(Type::Int), Box::new(error)).c_name(),
                ),
                ("<FIELD_PAYLOAD>", c_field("payload")),
                ("<FIELD_OK>", c_field("Ok")),
                ("<FIELD_ERR>", c_field("Err")),
            ],
        ));
    }
    if used
        .intrinsics
        .contains(&StandardIntrinsic::StringsParseBool)
    {
        let error = standard_error_type(program)?;
        output.push_str(&render(
            PARSE_BOOL_HELPER,
            &[
                (
                    "<RESULT_BOOL_ERROR>",
                    Type::Result(Box::new(Type::Bool), Box::new(error)).c_name(),
                ),
                ("<FIELD_PAYLOAD>", c_field("payload")),
                ("<FIELD_OK>", c_field("Ok")),
                ("<FIELD_ERR>", c_field("Err")),
            ],
        ));
    }
    for (intrinsic, function) in &used.function_values {
        output.push_str(&emit_callable_thunk(*intrinsic, function)?);
    }
    Ok(output)
}

fn emit_callable_thunk(
    intrinsic: StandardIntrinsic,
    function: &FunctionType,
) -> Result<String, CompileError> {
    let helper = intrinsic_helper_name(intrinsic).ok_or_else(|| {
        CompileError::new(format!(
            "standard intrinsic `{}` is not implemented in this compiler milestone",
            intrinsic.source_name()
        ))
    })?;
    let mut params = vec!["void *mlg_env".to_string()];
    params.extend(
        function
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| format!("{} mlg_arg_{index}", param.ty.c_param_type(param.mode))),
    );
    let args = (0..function.params.len())
        .map(|index| format!("mlg_arg_{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "static {} MLG_UNUSED {}({}) {{\n    (void)mlg_env;\n",
        function.return_type.c_name(),
        callable_thunk_name(&intrinsic.internal_name()),
        params.join(", ")
    );
    if *function.return_type == Type::Unit {
        output.push_str(&format!("    {helper}({args});\n"));
    } else {
        output.push_str(&format!("    return {helper}({args});\n"));
    }
    output.push_str("}\n\n");
    Ok(output)
}

fn emit_error_helper(program: &IrProgram) -> Result<String, CompileError> {
    let error = program
        .structs
        .iter()
        .find(|declaration| declaration.intrinsic == Some(StandardType::Error))
        .ok_or_else(|| {
            CompileError::new("IR invariant violation: standard Error type is missing")
        })?;
    let kind = program
        .enums
        .iter()
        .find(|declaration| declaration.intrinsic == Some(StandardType::ErrorKind))
        .ok_or_else(|| {
            CompileError::new("IR invariant violation: standard errors.Kind type is missing")
        })?;
    let invalid_data_tag = kind
        .variants
        .iter()
        .position(|variant| variant.name == "InvalidData")
        .ok_or_else(|| {
            CompileError::new("IR invariant violation: errors.Kind.InvalidData is missing")
        })?;

    Ok(render(
        ERROR_HELPER,
        &[
            ("<ERROR_TYPE>", Type::Struct(error.name.clone()).c_name()),
            ("<KIND_TYPE>", Type::Enum(kind.name.clone()).c_name()),
            ("<INVALID_DATA_TAG>", invalid_data_tag.to_string()),
            ("<FIELD_KIND>", c_field("kind")),
            ("<FIELD_MESSAGE>", c_field("message")),
        ],
    ))
}

fn standard_error_type(program: &IrProgram) -> Result<Type, CompileError> {
    program
        .structs
        .iter()
        .find(|declaration| declaration.intrinsic == Some(StandardType::Error))
        .map(|declaration| Type::Struct(declaration.name.clone()))
        .ok_or_else(|| CompileError::new("IR invariant violation: standard Error type is missing"))
}

fn render(template: &str, replacements: &[(&str, String)]) -> String {
    replacements
        .iter()
        .fold(template.to_string(), |rendered, (key, value)| {
            rendered.replace(key, value)
        })
}

#[derive(Default)]
struct StandardUses {
    intrinsics: BTreeSet<StandardIntrinsic>,
    function_values: BTreeMap<StandardIntrinsic, FunctionType>,
}

fn standard_uses(program: &IrProgram) -> StandardUses {
    let mut used = StandardUses::default();
    for function in &program.functions {
        for statement in &function.body {
            collect_stmt_intrinsics(statement, &mut used);
        }
    }
    for closure in &program.closures {
        for statement in &closure.body {
            collect_stmt_intrinsics(statement, &mut used);
        }
    }
    used
}

fn collect_stmt_intrinsics(statement: &IrStmt, used: &mut StandardUses) {
    match &statement.kind {
        IrStmtKind::Let { expr, .. }
        | IrStmtKind::Assign { expr, .. }
        | IrStmtKind::Return { expr }
        | IrStmtKind::Drop { expr }
        | IrStmtKind::Expr { expr } => collect_expr_intrinsics(expr, used),
        IrStmtKind::FieldAssign { base, expr, .. } => {
            collect_expr_intrinsics(base, used);
            collect_expr_intrinsics(expr, used);
        }
        IrStmtKind::IndexAssign {
            base, index, expr, ..
        } => {
            collect_expr_intrinsics(base, used);
            collect_expr_intrinsics(index, used);
            collect_expr_intrinsics(expr, used);
        }
        IrStmtKind::Overwrite { target, expr } => {
            collect_expr_intrinsics(target, used);
            collect_expr_intrinsics(expr, used);
        }
        IrStmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_expr_intrinsics(condition, used);
            for statement in then_body.iter().chain(else_body) {
                collect_stmt_intrinsics(statement, used);
            }
        }
        IrStmtKind::For {
            init,
            condition,
            post,
            body,
            cleanup,
        } => {
            if let Some(init) = init {
                match init.as_ref() {
                    IrForInit::Let { expr, .. } => collect_expr_intrinsics(expr, used),
                }
            }
            if let Some(condition) = condition {
                collect_expr_intrinsics(condition, used);
            }
            if let Some(post) = post {
                match post.as_ref() {
                    IrForPost::Assign { target, expr } => {
                        collect_expr_intrinsics(target, used);
                        collect_expr_intrinsics(expr, used);
                    }
                }
            }
            for statement in body.iter().chain(cleanup) {
                collect_stmt_intrinsics(statement, used);
            }
        }
        IrStmtKind::RangeFor {
            source,
            body,
            cleanup,
            ..
        } => {
            collect_expr_intrinsics(source, used);
            for statement in body.iter().chain(cleanup) {
                collect_stmt_intrinsics(statement, used);
            }
        }
        IrStmtKind::Match { scrutinee, arms } => {
            collect_expr_intrinsics(scrutinee, used);
            for arm in arms {
                for statement in &arm.body {
                    collect_stmt_intrinsics(statement, used);
                }
            }
        }
        IrStmtKind::Break | IrStmtKind::Continue => {}
    }
}

fn collect_expr_intrinsics(expression: &IrExpr, used: &mut StandardUses) {
    match &expression.kind {
        IrExprKind::IntrinsicCall { intrinsic, args } => {
            used.intrinsics.insert(*intrinsic);
            for arg in args {
                collect_expr_intrinsics(&arg.expr, used);
            }
        }
        IrExprKind::IntrinsicFunctionValue { intrinsic } => {
            used.intrinsics.insert(*intrinsic);
            if let Type::Function(function) = &expression.ty {
                used.function_values
                    .entry(*intrinsic)
                    .or_insert_with(|| function.clone());
            }
        }
        IrExprKind::FullExprTemporary { expr, .. }
        | IrExprKind::Unary { expr, .. }
        | IrExprKind::SliceFieldTake { source: expr }
        | IrExprKind::ArrayLen { array: expr } => collect_expr_intrinsics(expr, used),
        IrExprKind::If {
            condition,
            then_branch,
            then_cleanup,
            else_branch,
            else_cleanup,
        } => {
            collect_expr_intrinsics(condition, used);
            collect_expr_intrinsics(then_branch, used);
            collect_expr_intrinsics(else_branch, used);
            for statement in then_cleanup.iter().chain(else_cleanup) {
                collect_stmt_intrinsics(statement, used);
            }
        }
        IrExprKind::VariantConstructor { payloads, .. }
        | IrExprKind::ArrayLiteral { elements: payloads } => {
            for payload in payloads {
                collect_expr_intrinsics(payload, used);
            }
        }
        IrExprKind::Match { scrutinee, arms } => {
            collect_expr_intrinsics(scrutinee, used);
            for arm in arms {
                collect_expr_intrinsics(&arm.expr, used);
                for statement in &arm.cleanup {
                    collect_stmt_intrinsics(statement, used);
                }
            }
        }
        IrExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_expr_intrinsics(&field.expr, used);
            }
        }
        IrExprKind::FieldAccess { base, .. } => collect_expr_intrinsics(base, used),
        IrExprKind::Index { base, index }
        | IrExprKind::Binary {
            left: base,
            right: index,
            ..
        } => {
            collect_expr_intrinsics(base, used);
            collect_expr_intrinsics(index, used);
        }
        IrExprKind::SliceAppend { slice, item } => {
            collect_expr_intrinsics(slice, used);
            collect_expr_intrinsics(item, used);
        }
        IrExprKind::Call { args, .. } => {
            for arg in args {
                collect_expr_intrinsics(&arg.expr, used);
            }
        }
        IrExprKind::IndirectCall { callee, args } => {
            collect_expr_intrinsics(callee, used);
            for arg in args {
                collect_expr_intrinsics(&arg.expr, used);
            }
        }
        IrExprKind::ClosureValue { captures, .. } => {
            for capture in captures {
                collect_expr_intrinsics(&capture.expr, used);
            }
        }
        IrExprKind::Int(_)
        | IrExprKind::String(_)
        | IrExprKind::Bool(_)
        | IrExprKind::Var(_)
        | IrExprKind::FunctionValue { .. } => {}
    }
}

const BYTE_FIND_HELPER: &str = r#"static bool MLG_UNUSED mallang_std_find_bytes(
    const char *mlg_haystack,
    size_t mlg_haystack_len,
    const char *mlg_needle,
    size_t mlg_needle_len,
    size_t *mlg_offset_out
) {
    if (mlg_needle_len == 0) {
        *mlg_offset_out = 0;
        return true;
    }
    if (mlg_needle_len > mlg_haystack_len) {
        return false;
    }
    size_t mlg_limit = mlg_haystack_len - mlg_needle_len;
    for (size_t mlg_cursor = 0; mlg_cursor <= mlg_limit; mlg_cursor = mlg_cursor + 1) {
        if (memcmp(mlg_haystack + mlg_cursor, mlg_needle, mlg_needle_len) == 0) {
            *mlg_offset_out = mlg_cursor;
            return true;
        }
    }
    return false;
}

"#;

const BYTE_LEN_HELPER: &str = r#"static int64_t MLG_UNUSED mallang_std_strings_byte_len(
    const mlg_String *mlg_text
) {
    mallang_validate_string(*mlg_text);
    if (mlg_text->mlg_len > INT64_MAX) {
        mallang_runtime_error("string byte length overflow");
    }
    return (int64_t)mlg_text->mlg_len;
}

"#;

const SCALAR_COUNT_HELPER: &str = r#"static int64_t MLG_UNUSED mallang_std_strings_scalar_count(
    const mlg_String *mlg_text
) {
    mallang_validate_string(*mlg_text);
    int64_t mlg_count = 0;
    if (!mallang_utf8_scalar_count_bytes(mlg_text->mlg_data, mlg_text->mlg_len, &mlg_count)) {
        mallang_runtime_error("invalid UTF-8 string data");
    }
    return mlg_count;
}

"#;

const CONTAINS_HELPER: &str = r#"static bool MLG_UNUSED mallang_std_strings_contains(
    const mlg_String *mlg_text,
    const mlg_String *mlg_needle
) {
    mallang_validate_string(*mlg_text);
    mallang_validate_string(*mlg_needle);
    size_t mlg_offset = 0;
    return mallang_std_find_bytes(
        mlg_text->mlg_data,
        mlg_text->mlg_len,
        mlg_needle->mlg_data,
        mlg_needle->mlg_len,
        &mlg_offset
    );
}

"#;

const FIND_HELPER: &str = r#"static <OPTION_INT> MLG_UNUSED mallang_std_strings_find(
    const mlg_String *mlg_text,
    const mlg_String *mlg_needle
) {
    mallang_validate_string(*mlg_text);
    mallang_validate_string(*mlg_needle);
    size_t mlg_offset = 0;
    if (!mallang_std_find_bytes(
            mlg_text->mlg_data,
            mlg_text->mlg_len,
            mlg_needle->mlg_data,
            mlg_needle->mlg_len,
            &mlg_offset)) {
        return (<OPTION_INT>){ .tag = 0 };
    }
    if (mlg_offset > INT64_MAX) {
        mallang_runtime_error("string byte offset overflow");
    }
    return (<OPTION_INT>){
        .tag = 1,
        .<FIELD_PAYLOAD> = { .<FIELD_SOME> = (int64_t)mlg_offset }
    };
}

"#;

const SPLIT_HELPER: &str = r#"static <SLICE_STRING> MLG_UNUSED mallang_std_strings_split(
    const mlg_String *mlg_text,
    const mlg_String *mlg_separator
) {
    mallang_validate_string(*mlg_text);
    mallang_validate_string(*mlg_separator);
    if (mlg_separator->mlg_len == 0) {
        int64_t mlg_count = 0;
        if (!mallang_utf8_scalar_count_bytes(
                mlg_text->mlg_data,
                mlg_text->mlg_len,
                &mlg_count)) {
            mallang_runtime_error("invalid UTF-8 string data");
        }
        if (mlg_count == 0) {
            return (<SLICE_STRING>){
                .<FIELD_DATA> = NULL,
                .<FIELD_LEN> = 0,
                .<FIELD_CAP> = 0
            };
        }
        size_t mlg_count_size = (size_t)mlg_count;
        if (mlg_count_size > SIZE_MAX / sizeof(mlg_String)) {
            mallang_runtime_error("split allocation size overflow");
        }
        mlg_String *mlg_parts = mallang_alloc(
            mlg_count_size * sizeof(mlg_String),
            "split allocation failed"
        );
        size_t mlg_cursor = 0;
        for (int64_t mlg_index = 0; mlg_index < mlg_count; mlg_index = mlg_index + 1) {
            size_t mlg_width = mallang_utf8_sequence_length(
                mlg_text->mlg_data + mlg_cursor,
                mlg_text->mlg_len - mlg_cursor
            );
            if (mlg_width == 0) {
                mallang_runtime_error("invalid UTF-8 string data");
            }
            mlg_parts[mlg_index] = mallang_string_owned_from_bytes(
                mlg_text->mlg_data + mlg_cursor,
                mlg_width
            );
            mlg_cursor = mlg_cursor + mlg_width;
        }
        return (<SLICE_STRING>){
            .<FIELD_DATA> = mlg_parts,
            .<FIELD_LEN> = mlg_count,
            .<FIELD_CAP> = mlg_count
        };
    }

    size_t mlg_piece_count = 1;
    size_t mlg_cursor = 0;
    size_t mlg_relative = 0;
    while (mallang_std_find_bytes(
            mlg_text->mlg_data + mlg_cursor,
            mlg_text->mlg_len - mlg_cursor,
            mlg_separator->mlg_data,
            mlg_separator->mlg_len,
            &mlg_relative)) {
        if (mlg_piece_count == SIZE_MAX) {
            mallang_runtime_error("split element count overflow");
        }
        mlg_piece_count = mlg_piece_count + 1;
        mlg_cursor = mlg_cursor + mlg_relative + mlg_separator->mlg_len;
    }
    if (mlg_piece_count > INT64_MAX ||
        mlg_piece_count > SIZE_MAX / sizeof(mlg_String)) {
        mallang_runtime_error("split allocation size overflow");
    }
    mlg_String *mlg_parts = mallang_alloc(
        mlg_piece_count * sizeof(mlg_String),
        "split allocation failed"
    );
    size_t mlg_start = 0;
    size_t mlg_output_index = 0;
    while (mallang_std_find_bytes(
            mlg_text->mlg_data + mlg_start,
            mlg_text->mlg_len - mlg_start,
            mlg_separator->mlg_data,
            mlg_separator->mlg_len,
            &mlg_relative)) {
        size_t mlg_end = mlg_start + mlg_relative;
        mlg_parts[mlg_output_index] = mallang_string_owned_from_bytes(
            mlg_text->mlg_data + mlg_start,
            mlg_end - mlg_start
        );
        mlg_output_index = mlg_output_index + 1;
        mlg_start = mlg_end + mlg_separator->mlg_len;
    }
    mlg_parts[mlg_output_index] = mallang_string_owned_from_bytes(
        mlg_text->mlg_data + mlg_start,
        mlg_text->mlg_len - mlg_start
    );
    return (<SLICE_STRING>){
        .<FIELD_DATA> = mlg_parts,
        .<FIELD_LEN> = (int64_t)mlg_piece_count,
        .<FIELD_CAP> = (int64_t)mlg_piece_count
    };
}

"#;

const JOIN_HELPER: &str = r#"static mlg_String MLG_UNUSED mallang_std_strings_join(
    const <SLICE_STRING> *mlg_parts,
    const mlg_String *mlg_separator
) {
    mallang_validate_string(*mlg_separator);
    if (mlg_parts-><FIELD_LEN> < 0 ||
        mlg_parts-><FIELD_CAP> < mlg_parts-><FIELD_LEN> ||
        (mlg_parts-><FIELD_LEN> > 0 && mlg_parts-><FIELD_DATA> == NULL)) {
        mallang_runtime_error("invalid string slice storage");
    }
    size_t mlg_total = 0;
    for (int64_t mlg_index = 0; mlg_index < mlg_parts-><FIELD_LEN>; mlg_index = mlg_index + 1) {
        mlg_String mlg_part = mlg_parts-><FIELD_DATA>[mlg_index];
        mallang_validate_string(mlg_part);
        if (mlg_index > 0) {
            if (mlg_total > SIZE_MAX - mlg_separator->mlg_len) {
                mallang_runtime_error("joined string size overflow");
            }
            mlg_total = mlg_total + mlg_separator->mlg_len;
        }
        if (mlg_total > SIZE_MAX - mlg_part.mlg_len) {
            mallang_runtime_error("joined string size overflow");
        }
        mlg_total = mlg_total + mlg_part.mlg_len;
    }
    if (mlg_total == SIZE_MAX) {
        mallang_runtime_error("joined string size overflow");
    }
    char *mlg_data = mallang_alloc(mlg_total + 1, "joined string allocation failed");
    size_t mlg_cursor = 0;
    for (int64_t mlg_index = 0; mlg_index < mlg_parts-><FIELD_LEN>; mlg_index = mlg_index + 1) {
        mlg_String mlg_part = mlg_parts-><FIELD_DATA>[mlg_index];
        if (mlg_index > 0 && mlg_separator->mlg_len > 0) {
            memcpy(
                mlg_data + mlg_cursor,
                mlg_separator->mlg_data,
                mlg_separator->mlg_len
            );
            mlg_cursor = mlg_cursor + mlg_separator->mlg_len;
        }
        if (mlg_part.mlg_len > 0) {
            memcpy(mlg_data + mlg_cursor, mlg_part.mlg_data, mlg_part.mlg_len);
            mlg_cursor = mlg_cursor + mlg_part.mlg_len;
        }
    }
    mlg_data[mlg_total] = '\0';
    return (mlg_String){
        .mlg_data = mlg_data,
        .mlg_len = mlg_total,
        .mlg_storage = MLG_STRING_OWNED
    };
}

"#;

const FROM_INT_HELPER: &str = r#"static mlg_String MLG_UNUSED mallang_std_strings_from_int(
    int64_t mlg_value
) {
    char mlg_buffer[32];
    int mlg_written = snprintf(
        mlg_buffer,
        sizeof(mlg_buffer),
        "%lld",
        (long long)mlg_value
    );
    if (mlg_written < 0 || (size_t)mlg_written >= sizeof(mlg_buffer)) {
        mallang_runtime_error("integer formatting failed");
    }
    return mallang_string_owned_from_bytes(mlg_buffer, (size_t)mlg_written);
}

"#;

const FROM_BOOL_HELPER: &str = r#"static mlg_String MLG_UNUSED mallang_std_strings_from_bool(
    bool mlg_value
) {
    const char *mlg_text = mlg_value ? "true" : "false";
    size_t mlg_len = mlg_value ? 4 : 5;
    return mallang_string_owned_from_bytes(mlg_text, mlg_len);
}

"#;

const ERROR_HELPER: &str = r#"static <ERROR_TYPE> MLG_UNUSED mallang_std_invalid_data_error(
    const char *mlg_message
) {
    return (<ERROR_TYPE>){
        .<FIELD_KIND> = (<KIND_TYPE>){ .tag = <INVALID_DATA_TAG> },
        .<FIELD_MESSAGE> = mallang_string_owned_from_bytes(
            mlg_message,
            strlen(mlg_message)
        )
    };
}

"#;

const PARSE_INT_HELPER: &str = r#"static <RESULT_INT_ERROR> MLG_UNUSED mallang_std_strings_parse_int(
    const mlg_String *mlg_text
) {
    mallang_validate_string(*mlg_text);
    if (mlg_text->mlg_len == 0) {
        return (<RESULT_INT_ERROR>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_invalid_data_error("invalid integer text")
            }
        };
    }
    size_t mlg_cursor = 0;
    bool mlg_negative = false;
    if (mlg_text->mlg_data[0] == '-') {
        mlg_negative = true;
        mlg_cursor = 1;
        if (mlg_cursor == mlg_text->mlg_len) {
            return (<RESULT_INT_ERROR>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_invalid_data_error("invalid integer text")
                }
            };
        }
    }
    uint64_t mlg_limit = mlg_negative
        ? (uint64_t)INT64_MAX + UINT64_C(1)
        : (uint64_t)INT64_MAX;
    uint64_t mlg_value = 0;
    while (mlg_cursor < mlg_text->mlg_len) {
        unsigned char mlg_byte = (unsigned char)mlg_text->mlg_data[mlg_cursor];
        if (mlg_byte < '0' || mlg_byte > '9') {
            return (<RESULT_INT_ERROR>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_invalid_data_error("invalid integer text")
                }
            };
        }
        uint64_t mlg_digit = (uint64_t)(mlg_byte - '0');
        if (mlg_value > (mlg_limit - mlg_digit) / UINT64_C(10)) {
            return (<RESULT_INT_ERROR>){
                .tag = 1,
                .<FIELD_PAYLOAD> = {
                    .<FIELD_ERR> = mallang_std_invalid_data_error("integer value out of range")
                }
            };
        }
        mlg_value = mlg_value * UINT64_C(10) + mlg_digit;
        mlg_cursor = mlg_cursor + 1;
    }
    int64_t mlg_result;
    if (mlg_negative && mlg_value == (uint64_t)INT64_MAX + UINT64_C(1)) {
        mlg_result = INT64_MIN;
    } else if (mlg_negative) {
        mlg_result = -(int64_t)mlg_value;
    } else {
        mlg_result = (int64_t)mlg_value;
    }
    return (<RESULT_INT_ERROR>){
        .tag = 0,
        .<FIELD_PAYLOAD> = { .<FIELD_OK> = mlg_result }
    };
}

"#;

const PARSE_BOOL_HELPER: &str = r#"static <RESULT_BOOL_ERROR> MLG_UNUSED mallang_std_strings_parse_bool(
    const mlg_String *mlg_text
) {
    mallang_validate_string(*mlg_text);
    bool mlg_value;
    if (mlg_text->mlg_len == 4 && memcmp(mlg_text->mlg_data, "true", 4) == 0) {
        mlg_value = true;
    } else if (mlg_text->mlg_len == 5 && memcmp(mlg_text->mlg_data, "false", 5) == 0) {
        mlg_value = false;
    } else {
        return (<RESULT_BOOL_ERROR>){
            .tag = 1,
            .<FIELD_PAYLOAD> = {
                .<FIELD_ERR> = mallang_std_invalid_data_error("invalid boolean text")
            }
        };
    }
    return (<RESULT_BOOL_ERROR>){
        .tag = 0,
        .<FIELD_PAYLOAD> = { .<FIELD_OK> = mlg_value }
    };
}

"#;
