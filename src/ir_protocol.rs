use crate::{
    ast::{ArgMode, BinaryOp, ParamMode, UnaryOp},
    ir::{
        IrArg, IrClosureCaptureValue, IrEnumStorage, IrExpr, IrExprKind, IrFieldValue, IrForInit,
        IrForPost, IrMatchArm, IrMatchBlockArm, IrMatchPattern, IrProgram, IrStmt, IrStmtKind,
    },
    token::Span,
};

pub fn normalize_ir(program: &IrProgram) -> String {
    let mut lines = vec![format!("IR|{}", program.functions.len())];
    for function in &program.functions {
        lines.push(format!(
            "FUNCTION|{}|{}|{}|{}",
            function.name,
            function.return_type.source_name(),
            function.params.len(),
            function.body.len()
        ));
        for param in &function.params {
            lines.push(format!(
                "IPARAM|{}|{}|{}|{}",
                function.name,
                normalize_param_mode(param.mode),
                param.name,
                param.ty.source_name()
            ));
        }
        lines.extend(
            function
                .body
                .iter()
                .map(|statement| normalize_statement(statement, 0)),
        );
    }
    lines.push(format!("CLOSURES|{}", program.closures.len()));
    for closure in &program.closures {
        lines.push(format!(
            "CLOSURE|{}|{}|{}|{}|{}|{}",
            closure.name,
            closure.mutable,
            closure.return_type.source_name(),
            closure.params.len(),
            closure.captures.len(),
            closure.body.len()
        ));
        for capture in &closure.captures {
            lines.push(format!(
                "CCAPTURE|{}|{}|{}|{}",
                closure.name,
                capture.mutable,
                capture.name,
                capture.ty.source_name()
            ));
        }
        for param in &closure.params {
            lines.push(format!(
                "CPARAM|{}|{}|{}|{}",
                closure.name,
                normalize_param_mode(param.mode),
                param.name,
                param.ty.source_name()
            ));
        }
        lines.extend(
            closure
                .body
                .iter()
                .map(|statement| normalize_statement(statement, 0)),
        );
    }
    lines.join("\n")
}

fn normalize_param_mode(mode: ParamMode) -> &'static str {
    match mode {
        ParamMode::Owned => "owned",
        ParamMode::Con => "con",
        ParamMode::Mut => "mut",
    }
}

fn normalize_statement(statement: &IrStmt, depth: usize) -> String {
    let (kind, value, ty, children) = match &statement.kind {
        IrStmtKind::Let {
            mutable,
            name,
            ty,
            expr,
        } => (
            if *mutable {
                "Stmt.Let.Mutable"
            } else {
                "Stmt.Let.Immutable"
            },
            name.as_str(),
            ty.source_name(),
            vec![normalize_expression(expr, depth + 1)],
        ),
        IrStmtKind::Assign { name, expr } => (
            "Stmt.Assign",
            name.as_str(),
            "unit".to_string(),
            vec![normalize_expression(expr, depth + 1)],
        ),
        IrStmtKind::Return { expr } => (
            "Stmt.Return",
            "",
            "unit".to_string(),
            vec![normalize_expression(expr, depth + 1)],
        ),
        IrStmtKind::If {
            condition,
            then_body,
            else_body,
        } => (
            "Stmt.If",
            "",
            "unit".to_string(),
            vec![
                normalize_expression(condition, depth + 1),
                normalize_block("Block.Then", then_body, statement.span, depth + 1),
                normalize_block("Block.Else", else_body, statement.span, depth + 1),
            ],
        ),
        IrStmtKind::For {
            init,
            condition,
            post,
            body,
            cleanup,
        } => {
            let mut children = Vec::new();
            children.extend(init.iter().map(|init| normalize_for_init(init, depth + 1)));
            children.extend(condition.iter().map(|condition| {
                normalize_line(
                    depth + 1,
                    "F",
                    "ForCondition",
                    condition.span,
                    "",
                    "bool",
                    &[normalize_expression(condition, depth + 2)],
                )
            }));
            children.extend(post.iter().map(|post| normalize_for_post(post, depth + 1)));
            children.push(normalize_block(
                "Block.For",
                body,
                statement.span,
                depth + 1,
            ));
            children.push(normalize_block(
                "Block.ForCleanup",
                cleanup,
                statement.span,
                depth + 1,
            ));
            ("Stmt.For", "", "unit".to_string(), children)
        }
        IrStmtKind::RangeFor {
            index_name,
            value_name,
            source,
            element_ty,
            body,
            cleanup,
        } => {
            let bindings = format!("{index_name},{value_name}");
            let element_ty = element_ty.source_name();
            (
                "Stmt.RangeFor",
                "",
                "unit".to_string(),
                vec![
                    normalize_line(
                        depth + 1,
                        "F",
                        "RangeBindings",
                        statement.span,
                        &bindings,
                        &element_ty,
                        &[],
                    ),
                    normalize_expression(source, depth + 1),
                    normalize_block("Block.RangeFor", body, statement.span, depth + 1),
                    normalize_block("Block.RangeCleanup", cleanup, statement.span, depth + 1),
                ],
            )
        }
        IrStmtKind::Break => ("Stmt.Break", "", "unit".to_string(), Vec::new()),
        IrStmtKind::Continue => ("Stmt.Continue", "", "unit".to_string(), Vec::new()),
        IrStmtKind::Match { scrutinee, arms } => {
            let mut children = vec![normalize_expression(scrutinee, depth + 1)];
            children.extend(
                arms.iter()
                    .map(|arm| normalize_match_block_arm(arm, depth + 1)),
            );
            ("Stmt.Match", "", "unit".to_string(), children)
        }
        IrStmtKind::Assert { condition, .. } => (
            "Stmt.Assert",
            "",
            "unit".to_string(),
            vec![normalize_expression(condition, depth + 1)],
        ),
        IrStmtKind::Expr { expr } => (
            "Stmt.Expr",
            "",
            "unit".to_string(),
            vec![normalize_expression(expr, depth + 1)],
        ),
        IrStmtKind::FieldAssign { base, field, expr } => (
            "Stmt.FieldAssign",
            field.as_str(),
            "unit".to_string(),
            vec![
                normalize_expression(base, depth + 1),
                normalize_expression(expr, depth + 1),
            ],
        ),
        IrStmtKind::IndexAssign { base, index, expr } => (
            "Stmt.IndexAssign",
            "",
            "unit".to_string(),
            vec![
                normalize_expression(base, depth + 1),
                normalize_expression(index, depth + 1),
                normalize_expression(expr, depth + 1),
            ],
        ),
        IrStmtKind::Overwrite { target, expr } => (
            "Stmt.Overwrite",
            "",
            "unit".to_string(),
            vec![
                normalize_expression(target, depth + 1),
                normalize_expression(expr, depth + 1),
            ],
        ),
        IrStmtKind::Drop { expr } => (
            "Stmt.Drop",
            "",
            "unit".to_string(),
            vec![normalize_expression(expr, depth + 1)],
        ),
    };
    normalize_line(depth, "S", kind, statement.span, value, &ty, &children)
}

fn normalize_block(kind: &str, body: &[IrStmt], span: Span, depth: usize) -> String {
    let children = body
        .iter()
        .map(|statement| normalize_statement(statement, depth + 1))
        .collect::<Vec<_>>();
    normalize_line(depth, "B", kind, span, "", "unit", &children)
}

fn normalize_for_init(init: &IrForInit, depth: usize) -> String {
    match init {
        IrForInit::Let {
            mutable,
            name,
            ty,
            expr,
        } => normalize_line(
            depth,
            "F",
            if *mutable {
                "ForInit.Let.Mutable"
            } else {
                "ForInit.Let.Immutable"
            },
            expr.span,
            name,
            &ty.source_name(),
            &[normalize_expression(expr, depth + 1)],
        ),
    }
}

fn normalize_for_post(post: &IrForPost, depth: usize) -> String {
    match post {
        IrForPost::Assign { target, expr } => normalize_line(
            depth,
            "F",
            "ForPost.Assign",
            target.span.join(expr.span),
            "",
            "unit",
            &[
                normalize_expression(target, depth + 1),
                normalize_expression(expr, depth + 1),
            ],
        ),
    }
}

fn normalize_expression(expression: &IrExpr, depth: usize) -> String {
    let (kind, value, children) = match &expression.kind {
        IrExprKind::Int(value) => ("Expr.Int", value.to_string(), Vec::new()),
        IrExprKind::String(value) => ("Expr.String", value.clone(), Vec::new()),
        IrExprKind::Bool(value) => ("Expr.Bool", value.to_string(), Vec::new()),
        IrExprKind::Var(value) => ("Expr.Var", value.clone(), Vec::new()),
        IrExprKind::FullExprTemporary { name, expr } => (
            "Expr.FullExprTemporary",
            name.clone(),
            vec![normalize_expression(expr, depth + 1)],
        ),
        IrExprKind::FunctionValue { function } => {
            ("Expr.FunctionValue", function.clone(), Vec::new())
        }
        IrExprKind::IntrinsicFunctionValue { intrinsic } => (
            "Expr.IntrinsicFunctionValue",
            format!("{intrinsic:?}"),
            Vec::new(),
        ),
        IrExprKind::ClosureValue { closure, captures } => (
            "Expr.ClosureValue",
            closure.clone(),
            captures
                .iter()
                .map(|capture| normalize_capture(capture, depth + 1))
                .collect(),
        ),
        IrExprKind::Call { callee, args } => (
            "Expr.Call",
            callee.clone(),
            args.iter()
                .map(|arg| normalize_argument(arg, depth + 1))
                .collect(),
        ),
        IrExprKind::IntrinsicCall { intrinsic, args } => (
            "Expr.IntrinsicCall",
            format!("{intrinsic:?}"),
            args.iter()
                .map(|arg| normalize_argument(arg, depth + 1))
                .collect(),
        ),
        IrExprKind::IndirectCall { callee, args } => {
            let mut children = vec![normalize_expression(callee, depth + 1)];
            children.extend(args.iter().map(|arg| normalize_argument(arg, depth + 1)));
            ("Expr.IndirectCall", String::new(), children)
        }
        IrExprKind::StructLiteral { type_name, fields } => (
            "Expr.StructLiteral",
            type_name.clone(),
            fields
                .iter()
                .map(|field| normalize_field(field, depth + 1))
                .collect(),
        ),
        IrExprKind::ArrayLiteral { elements } => (
            "Expr.ArrayLiteral",
            String::new(),
            elements
                .iter()
                .map(|element| normalize_expression(element, depth + 1))
                .collect(),
        ),
        IrExprKind::VariantConstructor {
            variant,
            storage,
            payloads,
        } => (
            match storage {
                IrEnumStorage::Inline => "Expr.VariantConstructor.Inline",
                IrEnumStorage::Owned => "Expr.VariantConstructor.Owned",
            },
            variant.clone(),
            payloads
                .iter()
                .map(|payload| normalize_expression(payload, depth + 1))
                .collect(),
        ),
        IrExprKind::Match { scrutinee, arms } => {
            let mut children = vec![normalize_expression(scrutinee, depth + 1)];
            children.extend(arms.iter().map(|arm| normalize_match_arm(arm, depth + 1)));
            ("Expr.Match", String::new(), children)
        }
        IrExprKind::FieldAccess { base, field } => (
            "Expr.FieldAccess",
            field.clone(),
            vec![normalize_expression(base, depth + 1)],
        ),
        IrExprKind::SliceFieldTake { source } => {
            return normalize_expression(source, depth);
        }
        IrExprKind::Index { base, index } => (
            "Expr.Index",
            String::new(),
            vec![
                normalize_expression(base, depth + 1),
                normalize_expression(index, depth + 1),
            ],
        ),
        IrExprKind::ArrayLen { array } => (
            "Expr.ArrayLen",
            String::new(),
            vec![normalize_expression(array, depth + 1)],
        ),
        IrExprKind::SliceAppend { slice, item } => (
            "Expr.SliceAppend",
            String::new(),
            vec![
                normalize_expression(slice, depth + 1),
                normalize_expression(item, depth + 1),
            ],
        ),
        IrExprKind::If {
            condition,
            then_branch,
            then_cleanup,
            else_branch,
            else_cleanup,
        } => (
            "Expr.If",
            String::new(),
            vec![
                normalize_expression(condition, depth + 1),
                normalize_expression(then_branch, depth + 1),
                normalize_block(
                    "Block.IfThenCleanup",
                    then_cleanup,
                    expression.span,
                    depth + 1,
                ),
                normalize_expression(else_branch, depth + 1),
                normalize_block(
                    "Block.IfElseCleanup",
                    else_cleanup,
                    expression.span,
                    depth + 1,
                ),
            ],
        ),
        IrExprKind::Unary { op, expr } => (
            match op {
                UnaryOp::Negate => "Expr.Unary.Negate",
                UnaryOp::Not => "Expr.Unary.Not",
            },
            String::new(),
            vec![normalize_expression(expr, depth + 1)],
        ),
        IrExprKind::Binary { op, left, right } => (
            match op {
                BinaryOp::Add => "Expr.Binary.Add",
                BinaryOp::Subtract => "Expr.Binary.Subtract",
                BinaryOp::Multiply => "Expr.Binary.Multiply",
                BinaryOp::Divide => "Expr.Binary.Divide",
                BinaryOp::Remainder => "Expr.Binary.Remainder",
                BinaryOp::Equal => "Expr.Binary.Equal",
                BinaryOp::NotEqual => "Expr.Binary.NotEqual",
                BinaryOp::LogicalAnd => "Expr.Binary.LogicalAnd",
                BinaryOp::LogicalOr => "Expr.Binary.LogicalOr",
                BinaryOp::Less => "Expr.Binary.Less",
                BinaryOp::LessEqual => "Expr.Binary.LessEqual",
                BinaryOp::Greater => "Expr.Binary.Greater",
                BinaryOp::GreaterEqual => "Expr.Binary.GreaterEqual",
            },
            String::new(),
            vec![
                normalize_expression(left, depth + 1),
                normalize_expression(right, depth + 1),
            ],
        ),
    };
    normalize_line(
        depth,
        "E",
        kind,
        expression.span,
        &value,
        &expression.ty.source_name(),
        &children,
    )
}

fn normalize_field(field: &IrFieldValue, depth: usize) -> String {
    normalize_line(
        depth,
        "F",
        "Field.Value",
        field.span,
        &field.name,
        &field.expr.ty.source_name(),
        &[normalize_expression(&field.expr, depth + 1)],
    )
}

fn normalize_match_arm(arm: &IrMatchArm, depth: usize) -> String {
    let mut children = vec![
        normalize_match_pattern(&arm.pattern, arm.span, depth + 1),
        normalize_expression(&arm.expr, depth + 1),
    ];
    children.extend(
        arm.cleanup
            .iter()
            .map(|statement| normalize_statement(statement, depth + 1)),
    );
    normalize_line(
        depth,
        "M",
        "Match.Arm",
        arm.span,
        "",
        &arm.expr.ty.source_name(),
        &children,
    )
}

fn normalize_match_block_arm(arm: &IrMatchBlockArm, depth: usize) -> String {
    normalize_line(
        depth,
        "M",
        "Match.BlockArm",
        arm.span,
        "",
        "unit",
        &[
            normalize_match_pattern(&arm.pattern, arm.span, depth + 1),
            normalize_block("Block.Match", &arm.body, arm.span, depth + 1),
        ],
    )
}

fn normalize_match_pattern(pattern: &IrMatchPattern, span: Span, depth: usize) -> String {
    let (kind, value, ty, children) = match pattern {
        IrMatchPattern::Wildcard(ty) => ("Pattern.Wildcard", "", ty.source_name(), Vec::new()),
        IrMatchPattern::Binding { name, ty } => (
            "Pattern.Binding",
            name.as_str(),
            ty.source_name(),
            Vec::new(),
        ),
        IrMatchPattern::Variant {
            ty,
            variant,
            storage,
            payloads,
        } => (
            match storage {
                IrEnumStorage::Inline => "Pattern.Variant.Inline",
                IrEnumStorage::Owned => "Pattern.Variant.Owned",
            },
            variant.as_str(),
            ty.source_name(),
            payloads
                .iter()
                .map(|payload| normalize_match_pattern(payload, span, depth + 1))
                .collect(),
        ),
    };
    normalize_line(depth, "P", kind, span, value, &ty, &children)
}

fn normalize_capture(capture: &IrClosureCaptureValue, depth: usize) -> String {
    normalize_line(
        depth,
        "C",
        "Closure.Capture",
        capture.expr.span,
        &capture.name,
        &capture.expr.ty.source_name(),
        &[normalize_expression(&capture.expr, depth + 1)],
    )
}

fn normalize_argument(argument: &IrArg, depth: usize) -> String {
    let kind = match argument.mode {
        ArgMode::Owned => "Arg.Owned",
        ArgMode::Con => "Arg.Con",
        ArgMode::Mut => "Arg.Mut",
    };
    normalize_line(
        depth,
        "A",
        kind,
        argument.span,
        "",
        &argument.expr.ty.source_name(),
        &[normalize_expression(&argument.expr, depth + 1)],
    )
}

fn normalize_line(
    depth: usize,
    category: &str,
    kind: &str,
    span: Span,
    value: &str,
    ty: &str,
    children: &[String],
) -> String {
    let mut lines = vec![format!(
        "I|{depth}|{category}|{kind}|{}|{}|{}|{}|{ty}|{}",
        span.source.index(),
        span.start,
        span.end,
        encode_bytes(value),
        children.len()
    )];
    lines.extend(children.iter().cloned());
    lines.join("\n")
}

fn encode_bytes(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use crate::{
        check, ir::lower, parse_with_source_diagnostics, source::SourceMap, token::SourceId,
    };

    use super::normalize_ir;

    #[test]
    fn normalizes_typed_ir_with_the_self_hosting_protocol() {
        let source = "func main() { mut value := 1; value = value + 2 }\n";
        let mut sources = SourceMap::new();
        sources.add_file("fixture.mlg", source);
        let program =
            parse_with_source_diagnostics(source, SourceId::new(0)).expect("source parses");
        let checked = check(&program).expect("source checks");
        let ir = lower(&checked).expect("source lowers");
        let normalized = normalize_ir(&ir);

        assert!(normalized.starts_with("IR|1\nFUNCTION|main|unit|0|2\n"));
        assert!(normalized.contains("|S|Stmt.Let.Mutable|"));
        assert!(normalized.contains("|E|Expr.Binary.Add|"));
        assert!(normalized.ends_with("CLOSURES|0"));
    }
}
