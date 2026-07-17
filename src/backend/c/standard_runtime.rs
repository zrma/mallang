use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ast::ParamMode,
    ir::{IrArg, IrExpr, IrExprKind, IrForInit, IrForPost, IrProgram, IrStmt, IrStmtKind},
    semantic::{FunctionParamType, FunctionType, Type},
    standard::{StandardIntrinsic, StandardType},
};

use super::{
    names::{
        c_field, callable_thunk_name, drop_fn_name, mangle_type, map_entry_type_name, TypeCName,
    },
    platform_runtime::emit_platform_runtime,
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
        StandardIntrinsic::FsReadText => Some("mallang_std_fs_read_text"),
        StandardIntrinsic::FsWriteText => Some("mallang_std_fs_write_text"),
        StandardIntrinsic::IoReadStdin => Some("mallang_std_io_read_stdin"),
        StandardIntrinsic::IoWriteStdout => Some("mallang_std_io_write_stdout"),
        StandardIntrinsic::IoWriteStderr => Some("mallang_std_io_write_stderr"),
        StandardIntrinsic::OsArgs => Some("mallang_std_os_args"),
        StandardIntrinsic::OsEnv => Some("mallang_std_os_env"),
        StandardIntrinsic::OsExit => Some("mallang_std_os_exit"),
        _ => None,
    }
}

fn for_each_line_types_from_call(
    intrinsic: StandardIntrinsic,
    args: &[IrArg],
) -> Option<(&Type, &Type)> {
    if intrinsic != StandardIntrinsic::FsForEachLine {
        return None;
    }
    Some((&args.get(1)?.expr.ty, &args.get(2)?.expr.ty))
}

fn for_each_line_types_from_function(
    intrinsic: StandardIntrinsic,
    function: &FunctionType,
) -> Option<(&Type, &Type)> {
    if intrinsic != StandardIntrinsic::FsForEachLine {
        return None;
    }
    Some((&function.params.get(1)?.ty, &function.params.get(2)?.ty))
}

fn for_each_line_helper_name(context: &Type, state: &Type) -> String {
    format!(
        "mallang_std_fs_for_each_line_{}_{}",
        mangle_type(context),
        mangle_type(state)
    )
}

fn collection_map_type_from_call<'a>(
    intrinsic: StandardIntrinsic,
    result_ty: &'a Type,
    args: &'a [IrArg],
) -> Option<&'a Type> {
    match intrinsic {
        StandardIntrinsic::CollectionsNewMap => Some(result_ty),
        StandardIntrinsic::CollectionsCount
        | StandardIntrinsic::CollectionsInsert
        | StandardIntrinsic::CollectionsWith
        | StandardIntrinsic::CollectionsUpdate
        | StandardIntrinsic::CollectionsRemove => args.first().map(|arg| &arg.expr.ty),
        _ => None,
    }
}

fn collection_map_type_from_function(
    intrinsic: StandardIntrinsic,
    function: &FunctionType,
) -> Option<&Type> {
    match intrinsic {
        StandardIntrinsic::CollectionsNewMap => Some(&function.return_type),
        StandardIntrinsic::CollectionsCount
        | StandardIntrinsic::CollectionsInsert
        | StandardIntrinsic::CollectionsWith
        | StandardIntrinsic::CollectionsUpdate
        | StandardIntrinsic::CollectionsRemove => function.params.first().map(|param| &param.ty),
        _ => None,
    }
}

fn collection_helper_name(intrinsic: StandardIntrinsic, map_ty: &Type) -> String {
    format!(
        "mallang_std_collections_{}_{}",
        intrinsic.function_name(),
        map_ty.c_name()
    )
}

pub(super) fn intrinsic_helper_name_for_call(
    program: &IrProgram,
    intrinsic: StandardIntrinsic,
    result_ty: &Type,
    args: &[IrArg],
) -> Result<String, CompileError> {
    if let Some(helper) = intrinsic_helper_name(intrinsic) {
        return Ok(helper.to_string());
    }
    if let Some((context, state)) = for_each_line_types_from_call(intrinsic, args) {
        return Ok(for_each_line_helper_name(context, state));
    }
    let map_ty = collection_map_type_from_call(intrinsic, result_ty, args)
        .ok_or_else(|| unimplemented_intrinsic(intrinsic))?;
    map_type_args(program, map_ty)?;
    Ok(collection_helper_name(intrinsic, map_ty))
}

pub(super) fn intrinsic_helper_name_for_function(
    program: &IrProgram,
    intrinsic: StandardIntrinsic,
    function: &FunctionType,
) -> Result<String, CompileError> {
    if let Some(helper) = intrinsic_helper_name(intrinsic) {
        return Ok(helper.to_string());
    }
    if let Some((context, state)) = for_each_line_types_from_function(intrinsic, function) {
        return Ok(for_each_line_helper_name(context, state));
    }
    let map_ty = collection_map_type_from_function(intrinsic, function)
        .ok_or_else(|| unimplemented_intrinsic(intrinsic))?;
    map_type_args(program, map_ty)?;
    Ok(collection_helper_name(intrinsic, map_ty))
}

pub(super) fn intrinsic_callable_thunk_name(
    intrinsic: StandardIntrinsic,
    function: &FunctionType,
) -> String {
    let mut name = intrinsic.internal_name();
    if let Some((context, state)) = for_each_line_types_from_function(intrinsic, function) {
        name.push('_');
        name.push_str(&mangle_type(context));
        name.push('_');
        name.push_str(&mangle_type(state));
    }
    if let Some(map_ty) = collection_map_type_from_function(intrinsic, function) {
        name.push('_');
        name.push_str(&mangle_type(map_ty));
    }
    callable_thunk_name(&name)
}

fn unimplemented_intrinsic(intrinsic: StandardIntrinsic) -> CompileError {
    CompileError::new(format!(
        "standard intrinsic `{}` is not implemented in this compiler milestone",
        intrinsic.source_name()
    ))
}

pub(super) fn emit_standard_runtime(program: &IrProgram) -> Result<String, CompileError> {
    let used = standard_uses(program);
    if used.map_uses.is_empty()
        && used.for_each_line_uses.is_empty()
        && used
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
    let for_each_line_uses = used
        .for_each_line_uses
        .values()
        .map(|line_use| {
            (
                for_each_line_helper_name(&line_use.context, &line_use.state),
                line_use.context.clone(),
                line_use.state.clone(),
            )
        })
        .collect::<Vec<_>>();
    output.push_str(&emit_platform_runtime(
        program,
        &used.intrinsics,
        &for_each_line_uses,
    )?);
    for map_use in used.map_uses.values() {
        output.push_str(&emit_map_runtime(program, map_use)?);
    }
    for (intrinsic, function) in used.function_values.values() {
        output.push_str(&emit_callable_thunk(program, *intrinsic, function)?);
    }
    Ok(output)
}

pub(super) fn program_uses_intrinsic(program: &IrProgram, intrinsic: StandardIntrinsic) -> bool {
    standard_uses(program).intrinsics.contains(&intrinsic)
}

fn emit_callable_thunk(
    program: &IrProgram,
    intrinsic: StandardIntrinsic,
    function: &FunctionType,
) -> Result<String, CompileError> {
    let helper = intrinsic_helper_name_for_function(program, intrinsic, function)?;
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
        intrinsic_callable_thunk_name(intrinsic, function),
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
    function_values: BTreeMap<String, (StandardIntrinsic, FunctionType)>,
    map_uses: BTreeMap<String, MapUse>,
    for_each_line_uses: BTreeMap<String, ForEachLineUse>,
}

struct MapUse {
    ty: Type,
    intrinsics: BTreeSet<StandardIntrinsic>,
}

struct ForEachLineUse {
    context: Type,
    state: Type,
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
        | IrStmtKind::Assert {
            condition: expr, ..
        }
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
            if let Some((context, state)) = for_each_line_types_from_call(*intrinsic, args) {
                record_for_each_line_use(used, context.clone(), state.clone());
            }
            if let Some(map_ty) = collection_map_type_from_call(*intrinsic, &expression.ty, args) {
                record_map_use(used, map_ty.clone(), *intrinsic);
            }
            for arg in args {
                collect_expr_intrinsics(&arg.expr, used);
            }
        }
        IrExprKind::IntrinsicFunctionValue { intrinsic } => {
            used.intrinsics.insert(*intrinsic);
            if let Type::Function(function) = &expression.ty {
                if let Some((context, state)) =
                    for_each_line_types_from_function(*intrinsic, function)
                {
                    record_for_each_line_use(used, context.clone(), state.clone());
                }
                if let Some(map_ty) = collection_map_type_from_function(*intrinsic, function) {
                    record_map_use(used, map_ty.clone(), *intrinsic);
                }
                let key = format!(
                    "{}:{}",
                    intrinsic.source_name(),
                    mangle_type(&expression.ty)
                );
                used.function_values
                    .entry(key)
                    .or_insert_with(|| (*intrinsic, function.clone()));
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

fn record_for_each_line_use(used: &mut StandardUses, context: Type, state: Type) {
    let key = for_each_line_helper_name(&context, &state);
    used.for_each_line_uses
        .entry(key)
        .or_insert(ForEachLineUse { context, state });
}

fn record_map_use(used: &mut StandardUses, map_ty: Type, intrinsic: StandardIntrinsic) {
    let key = map_ty.c_name();
    used.map_uses
        .entry(key)
        .or_insert_with(|| MapUse {
            ty: map_ty,
            intrinsics: BTreeSet::new(),
        })
        .intrinsics
        .insert(intrinsic);
}

fn map_type_args<'a>(
    program: &'a IrProgram,
    map_ty: &Type,
) -> Result<(&'a Type, &'a Type), CompileError> {
    let Type::Struct(name) = map_ty else {
        return Err(CompileError::new(
            "IR invariant violation: collection intrinsic requires a Map type",
        ));
    };
    let declaration = program
        .structs
        .iter()
        .find(|declaration| declaration.name == *name)
        .ok_or_else(|| {
            CompileError::new(format!("IR invariant violation: unknown struct `{name}`"))
        })?;
    if declaration.intrinsic != Some(StandardType::Map) {
        return Err(CompileError::new(
            "IR invariant violation: collection intrinsic requires a standard Map type",
        ));
    }
    let [key, value] = declaration.intrinsic_args.as_slice() else {
        return Err(CompileError::new(
            "IR invariant violation: standard Map must have two type arguments",
        ));
    };
    Ok((key, value))
}

fn map_internal_helper_name(map_ty: &Type, operation: &str) -> String {
    format!(
        "mallang_std_collections_map_{}_{}",
        operation,
        map_ty.c_name()
    )
}

fn emit_map_runtime(program: &IrProgram, map_use: &MapUse) -> Result<String, CompileError> {
    let map_ty = &map_use.ty;
    let (key, value) = map_type_args(program, map_ty)?;
    let map_name = map_ty.c_name();
    let entry_name = map_entry_type_name(map_ty);
    let validate = map_internal_helper_name(map_ty, "validate");
    let hash = map_internal_helper_name(map_ty, "hash");
    let equal = map_internal_helper_name(map_ty, "equal");
    let find = map_internal_helper_name(map_ty, "find");
    let ensure = map_internal_helper_name(map_ty, "ensure_insert_capacity");
    let buckets = c_field("buckets");
    let len = c_field("len");
    let cap = c_field("cap");
    let hash_field = c_field("hash");
    let key_field = c_field("key");
    let next = c_field("next");

    let hash_body = match key {
        Type::Int => r#"uint64_t mlg_hash = (uint64_t)(*mlg_key);
    mlg_hash = (mlg_hash ^ (mlg_hash >> 30)) * UINT64_C(0xbf58476d1ce4e5b9);
    mlg_hash = (mlg_hash ^ (mlg_hash >> 27)) * UINT64_C(0x94d049bb133111eb);
    return mlg_hash ^ (mlg_hash >> 31);"#
            .to_string(),
        Type::Bool => {
            "return *mlg_key ? UINT64_C(0x9e3779b97f4a7c15) : UINT64_C(0x243f6a8885a308d3);"
                .to_string()
        }
        Type::String => r#"mallang_validate_string(*mlg_key);
    uint64_t mlg_hash = UINT64_C(14695981039346656037);
    const unsigned char *mlg_bytes = (const unsigned char *)mlg_key->mlg_data;
    for (size_t mlg_i = 0; mlg_i < mlg_key->mlg_len; mlg_i = mlg_i + 1) {
        mlg_hash = (mlg_hash ^ (uint64_t)mlg_bytes[mlg_i]) * UINT64_C(1099511628211);
    }
    return mlg_hash;"#
            .to_string(),
        _ => {
            return Err(CompileError::new(format!(
                "IR invariant violation: unsupported Map key type `{}`",
                key.source_name()
            )))
        }
    };
    let equal_body = match key {
        Type::String => "return mallang_string_equal(*mlg_left, *mlg_right);".to_string(),
        Type::Int | Type::Bool => "return *mlg_left == *mlg_right;".to_string(),
        _ => unreachable!("Map key type was validated above"),
    };

    let mut output = format!(
        r#"static void MLG_UNUSED {validate}(const {map_name} *mlg_map) {{
    if (mlg_map == NULL || mlg_map->{len} < 0 || mlg_map->{cap} < 0 ||
        mlg_map->{len} > mlg_map->{cap} ||
        (mlg_map->{cap} == 0 && mlg_map->{buckets} != NULL) ||
        (mlg_map->{cap} > 0 && mlg_map->{buckets} == NULL)) {{
        mallang_runtime_error("invalid map storage");
    }}
}}

static uint64_t MLG_UNUSED {hash}(const {key_type} *mlg_key) {{
    if (mlg_key == NULL) {{
        mallang_runtime_error("invalid map key");
    }}
    {hash_body}
}}

static bool MLG_UNUSED {equal}(
    const {key_type} *mlg_left,
    const {key_type} *mlg_right
) {{
    if (mlg_left == NULL || mlg_right == NULL) {{
        mallang_runtime_error("invalid map key");
    }}
    {equal_body}
}}

static {entry_name} *MLG_UNUSED {find}(
    const {map_name} *mlg_map,
    const {key_type} *mlg_key,
    uint64_t mlg_hash
) {{
    {validate}(mlg_map);
    if (mlg_map->{cap} == 0) {{
        return NULL;
    }}
    uint64_t mlg_bucket = mlg_hash % (uint64_t)mlg_map->{cap};
    {entry_name} *mlg_entry = mlg_map->{buckets}[mlg_bucket];
    while (mlg_entry != NULL) {{
        if (mlg_entry->{hash_field} == mlg_hash &&
            {equal}(&(mlg_entry->{key_field}), mlg_key)) {{
            return mlg_entry;
        }}
        mlg_entry = mlg_entry->{next};
    }}
    return NULL;
}}

static void MLG_UNUSED {ensure}({map_name} *mlg_map) {{
    {validate}(mlg_map);
    if (mlg_map->{len} == INT64_MAX) {{
        mallang_runtime_error("map capacity overflow");
    }}
    int64_t mlg_required = mlg_map->{len} + 1;
    if (mlg_map->{cap} > 0 &&
        mlg_required <= mlg_map->{cap} - (mlg_map->{cap} / 4)) {{
        return;
    }}
    int64_t mlg_new_cap = 8;
    if (mlg_map->{cap} > 0) {{
        if (mlg_map->{cap} > INT64_MAX / 2) {{
            mallang_runtime_error("map capacity overflow");
        }}
        mlg_new_cap = mlg_map->{cap} * 2;
    }}
    if ((uint64_t)mlg_new_cap > SIZE_MAX / sizeof({entry_name} *)) {{
        mallang_runtime_error("map allocation size overflow");
    }}
    {entry_name} **mlg_new_buckets = mallang_alloc(
        sizeof({entry_name} *) * (size_t)mlg_new_cap,
        "map bucket allocation failed"
    );
    memset(
        mlg_new_buckets,
        0,
        sizeof({entry_name} *) * (size_t)mlg_new_cap
    );
    for (int64_t mlg_i = 0; mlg_i < mlg_map->{cap}; mlg_i = mlg_i + 1) {{
        {entry_name} *mlg_entry = mlg_map->{buckets}[mlg_i];
        while (mlg_entry != NULL) {{
            {entry_name} *mlg_next = mlg_entry->{next};
            uint64_t mlg_bucket = mlg_entry->{hash_field} % (uint64_t)mlg_new_cap;
            mlg_entry->{next} = mlg_new_buckets[mlg_bucket];
            mlg_new_buckets[mlg_bucket] = mlg_entry;
            mlg_entry = mlg_next;
        }}
    }}
    mallang_dealloc(mlg_map->{buckets});
    mlg_map->{buckets} = mlg_new_buckets;
    mlg_map->{cap} = mlg_new_cap;
}}

"#,
        key_type = key.c_name(),
    );

    for intrinsic in &map_use.intrinsics {
        match intrinsic {
            StandardIntrinsic::CollectionsNewMap => output.push_str(&format!(
                "static {map_name} MLG_UNUSED {}(void) {{\n    return ({map_name}){{ .{buckets} = NULL, .{len} = 0, .{cap} = 0 }};\n}}\n\n",
                collection_helper_name(*intrinsic, map_ty)
            )),
            StandardIntrinsic::CollectionsCount => output.push_str(&format!(
                "static int64_t MLG_UNUSED {}(const {map_name} *mlg_map) {{\n    {validate}(mlg_map);\n    return mlg_map->{len};\n}}\n\n",
                collection_helper_name(*intrinsic, map_ty)
            )),
            StandardIntrinsic::CollectionsInsert => output.push_str(&emit_map_insert(
                map_ty,
                key,
                value,
                &entry_name,
                &hash,
                &find,
                &ensure,
            )),
            StandardIntrinsic::CollectionsWith => output.push_str(&emit_map_callback(
                *intrinsic,
                map_ty,
                key,
                value,
                ParamMode::Con,
                &hash,
                &find,
            )),
            StandardIntrinsic::CollectionsUpdate => output.push_str(&emit_map_callback(
                *intrinsic,
                map_ty,
                key,
                value,
                ParamMode::Mut,
                &hash,
                &find,
            )),
            StandardIntrinsic::CollectionsRemove => output.push_str(&emit_map_remove(
                map_ty,
                key,
                value,
                &entry_name,
                &validate,
                &hash,
                &equal,
            )),
            _ => {
                return Err(CompileError::new(
                    "IR invariant violation: non-collection intrinsic recorded as Map use",
                ))
            }
        }
    }
    Ok(output)
}

fn emit_map_insert(
    map_ty: &Type,
    key: &Type,
    value: &Type,
    entry_name: &str,
    hash: &str,
    find: &str,
    ensure: &str,
) -> String {
    let map_name = map_ty.c_name();
    let helper = collection_helper_name(StandardIntrinsic::CollectionsInsert, map_ty);
    let option = Type::Option(Box::new(value.clone()));
    let buckets = c_field("buckets");
    let len = c_field("len");
    let cap = c_field("cap");
    let hash_field = c_field("hash");
    let key_field = c_field("key");
    let value_field = c_field("value");
    let next = c_field("next");
    let payload = c_field("payload");
    let some = c_field("Some");
    let drop_key = if key.needs_cleanup() {
        format!("        {}(&mlg_key);\n", drop_fn_name(key))
    } else {
        String::new()
    };
    format!(
        r#"static {option_type} MLG_UNUSED {helper}(
    {map_name} *mlg_map,
    {key_type} mlg_key,
    {value_type} mlg_value
) {{
    uint64_t mlg_hash = {hash}(&mlg_key);
    {entry_name} *mlg_existing = {find}(mlg_map, &mlg_key, mlg_hash);
    if (mlg_existing != NULL) {{
{drop_key}        {option_type} mlg_result = {{ .tag = 1 }};
        mlg_result.{payload}.{some} = mlg_existing->{value_field};
        mlg_existing->{value_field} = mlg_value;
        return mlg_result;
    }}
    {ensure}(mlg_map);
    uint64_t mlg_bucket = mlg_hash % (uint64_t)mlg_map->{cap};
    {entry_name} *mlg_entry = mallang_alloc(
        sizeof({entry_name}),
        "map entry allocation failed"
    );
    mlg_entry->{hash_field} = mlg_hash;
    mlg_entry->{key_field} = mlg_key;
    mlg_entry->{value_field} = mlg_value;
    mlg_entry->{next} = mlg_map->{buckets}[mlg_bucket];
    mlg_map->{buckets}[mlg_bucket] = mlg_entry;
    mlg_map->{len} = mlg_map->{len} + 1;
    return ({option_type}){{ .tag = 0 }};
}}

"#,
        option_type = option.c_name(),
        key_type = key.c_name(),
        value_type = value.c_name(),
    )
}

fn emit_map_callback(
    intrinsic: StandardIntrinsic,
    map_ty: &Type,
    key: &Type,
    value: &Type,
    value_mode: ParamMode,
    hash: &str,
    find: &str,
) -> String {
    let helper = collection_helper_name(intrinsic, map_ty);
    let callback = Type::Function(FunctionType {
        mutable: false,
        params: vec![FunctionParamType {
            mode: value_mode,
            ty: value.clone(),
        }],
        return_type: Box::new(Type::Unit),
    });
    let value_field = c_field("value");
    format!(
        r#"static bool MLG_UNUSED {helper}(
    {map_param} mlg_map,
    const {key_type} *mlg_key,
    const {callback_type} *mlg_callback
) {{
    uint64_t mlg_hash = {hash}(mlg_key);
    {entry_type} *mlg_entry = {find}(mlg_map, mlg_key, mlg_hash);
    if (mlg_entry == NULL) {{
        return false;
    }}
    if (mlg_callback == NULL || mlg_callback->mlg_call == NULL) {{
        mallang_runtime_error("invalid map callback");
    }}
    mlg_callback->mlg_call(mlg_callback->mlg_env, &(mlg_entry->{value_field}));
    return true;
}}

"#,
        map_param = map_ty.c_param_type(if value_mode == ParamMode::Con {
            ParamMode::Con
        } else {
            ParamMode::Mut
        }),
        key_type = key.c_name(),
        callback_type = callback.c_name(),
        entry_type = map_entry_type_name(map_ty),
    )
}

fn emit_map_remove(
    map_ty: &Type,
    key: &Type,
    value: &Type,
    entry_name: &str,
    validate: &str,
    hash: &str,
    equal: &str,
) -> String {
    let map_name = map_ty.c_name();
    let helper = collection_helper_name(StandardIntrinsic::CollectionsRemove, map_ty);
    let option = Type::Option(Box::new(value.clone()));
    let buckets = c_field("buckets");
    let len = c_field("len");
    let cap = c_field("cap");
    let hash_field = c_field("hash");
    let key_field = c_field("key");
    let value_field = c_field("value");
    let next = c_field("next");
    let payload = c_field("payload");
    let some = c_field("Some");
    let drop_key = if key.needs_cleanup() {
        format!(
            "            {}(&(mlg_removed->{}));\n",
            drop_fn_name(key),
            key_field
        )
    } else {
        String::new()
    };
    format!(
        r#"static {option_type} MLG_UNUSED {helper}(
    {map_name} *mlg_map,
    const {key_type} *mlg_key
) {{
    {validate}(mlg_map);
    if (mlg_map->{cap} == 0) {{
        return ({option_type}){{ .tag = 0 }};
    }}
    uint64_t mlg_hash = {hash}(mlg_key);
    uint64_t mlg_bucket = mlg_hash % (uint64_t)mlg_map->{cap};
    {entry_name} **mlg_link = &(mlg_map->{buckets}[mlg_bucket]);
    while (*mlg_link != NULL) {{
        {entry_name} *mlg_entry = *mlg_link;
        if (mlg_entry->{hash_field} == mlg_hash &&
            {equal}(&(mlg_entry->{key_field}), mlg_key)) {{
            *mlg_link = mlg_entry->{next};
            {entry_name} *mlg_removed = mlg_entry;
{drop_key}            {option_type} mlg_result = {{ .tag = 1 }};
            mlg_result.{payload}.{some} = mlg_removed->{value_field};
            mallang_dealloc(mlg_removed);
            mlg_map->{len} = mlg_map->{len} - 1;
            return mlg_result;
        }}
        mlg_link = &(mlg_entry->{next});
    }}
    return ({option_type}){{ .tag = 0 }};
}}

"#,
        option_type = option.c_name(),
        key_type = key.c_name(),
    )
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
