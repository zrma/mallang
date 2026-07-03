use super::*;
use crate::{
    ast::{ArgMode, ParamMode},
    check,
    ir::{
        lower, IrArg, IrExpr, IrExprKind, IrForInit, IrFunction, IrMatchBlockArm, IrMatchPattern,
        IrProgram, IrStmt, IrStmtKind, IrStruct, IrStructField,
    },
    parse,
    semantic::Type,
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
fn generates_single_runtime_error_helper() {
    let program = parse(
        r#"
func main() {
values := [1]int{1}
print(values[0])
}
"#,
    )
    .unwrap();
    let checked = check(&program).unwrap();
    let ir = lower(&checked).unwrap();
    let c = generate_c_from_ir(&ir).unwrap();

    assert!(c.contains("static void mallang_runtime_error(const char *message)"));
    assert_eq!(c.matches("fprintf(stderr").count(), 1);
    assert!(c.contains("mallang_runtime_error(\"array index out of bounds\")"));
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

    assert!(c.contains(
        "void mlg_consume(mlg_Slice_int mlg_values) {\n    mlg_drop_Slice_int(&(mlg_values));\n}"
    ));
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

    assert!(c.contains("mlg_drop_Slice_int(&((mlg_values).mlg_data[mallang_check_index(0, 2)]));"));
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
fn rejects_invalid_ir_print_arity() {
    let span = crate::token::Span { start: 0, end: 0 };
    let program = IrProgram {
        structs: Vec::new(),
        functions: vec![IrFunction {
            name: "main".to_string(),
            params: Vec::new(),
            return_type: Type::Unit,
            body: vec![IrStmt {
                kind: IrStmtKind::Expr {
                    expr: IrExpr {
                        kind: IrExprKind::Call {
                            callee: "print".to_string(),
                            args: Vec::new(),
                        },
                        ty: Type::Unit,
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
        .contains("IR invariant violation: print arity"));
}

#[test]
fn rejects_invalid_ir_range_source_type() {
    let span = crate::token::Span { start: 0, end: 0 };
    let program = IrProgram {
        structs: Vec::new(),
        functions: vec![IrFunction {
            name: "main".to_string(),
            params: Vec::new(),
            return_type: Type::Unit,
            body: vec![IrStmt {
                kind: IrStmtKind::RangeFor {
                    index_name: "index".to_string(),
                    value_name: "value".to_string(),
                    source: IrExpr {
                        kind: IrExprKind::Var("not_rangeable".to_string()),
                        ty: Type::Int,
                        span,
                    },
                    element_ty: Type::Int,
                    body: Vec::new(),
                },
                span,
            }],
        }],
    };

    let error = generate_c_from_ir(&program).unwrap_err();

    assert!(error
        .message
        .contains("IR invariant violation: range source must be an array or slice"));
}

#[test]
fn rejects_invalid_ir_option_match_arm() {
    let span = crate::token::Span { start: 0, end: 0 };
    let option_int = Type::Option(Box::new(Type::Int));
    let program = IrProgram {
        structs: Vec::new(),
        functions: vec![IrFunction {
            name: "main".to_string(),
            params: Vec::new(),
            return_type: Type::Unit,
            body: vec![IrStmt {
                kind: IrStmtKind::Match {
                    scrutinee: IrExpr {
                        kind: IrExprKind::AdtConstructor {
                            constructor: crate::ir::IrAdtConstructor::None,
                            payload: None,
                        },
                        ty: option_int,
                        span,
                    },
                    arms: vec![IrMatchBlockArm {
                        pattern: IrMatchPattern::Ok("value".to_string()),
                        body: Vec::new(),
                        span,
                    }],
                },
                span,
            }],
        }],
    };

    let error = generate_c_from_ir(&program).unwrap_err();

    assert!(error
        .message
        .contains("IR invariant violation: invalid Option match arm"));
}

#[test]
fn rejects_invalid_ir_borrow_argument_expression() {
    let span = crate::token::Span { start: 0, end: 0 };
    let program = IrProgram {
        structs: Vec::new(),
        functions: vec![
            IrFunction {
                name: "inspect".to_string(),
                params: vec![crate::ir::IrParam {
                    name: "value".to_string(),
                    mode: ParamMode::Con,
                    ty: Type::Int,
                }],
                return_type: Type::Unit,
                body: Vec::new(),
            },
            IrFunction {
                name: "main".to_string(),
                params: Vec::new(),
                return_type: Type::Unit,
                body: vec![IrStmt {
                    kind: IrStmtKind::Expr {
                        expr: IrExpr {
                            kind: IrExprKind::Call {
                                callee: "inspect".to_string(),
                                args: vec![IrArg {
                                    mode: ArgMode::Con,
                                    expr: IrExpr {
                                        kind: IrExprKind::Binary {
                                            op: crate::ast::BinaryOp::Add,
                                            left: Box::new(IrExpr {
                                                kind: IrExprKind::Int(1),
                                                ty: Type::Int,
                                                span,
                                            }),
                                            right: Box::new(IrExpr {
                                                kind: IrExprKind::Int(2),
                                                ty: Type::Int,
                                                span,
                                            }),
                                        },
                                        ty: Type::Int,
                                        span,
                                    },
                                    span,
                                }],
                            },
                            ty: Type::Unit,
                            span,
                        },
                    },
                    span,
                }],
            },
        ],
    };

    let error = generate_c_from_ir(&program).unwrap_err();

    assert!(error
        .message
        .contains("IR invariant violation: invalid borrow argument expression"));
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
    assert!(c.contains("mallang runtime error: %s"));
    assert!(c.contains("mallang_runtime_error(\"division by zero\")"));
    assert!(c.contains("mallang_runtime_error(\"integer overflow\")"));
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
    assert!(c.contains("mallang_runtime_error(\"integer overflow\")"));
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
    assert!(c.contains("mallang_runtime_error(\"array index out of bounds\")"));
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
    assert!(c.contains("mallang_runtime_error(\"slice index out of bounds\")"));
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
    assert!(c.contains("mallang_runtime_error(\"slice allocation failed\")"));
    assert!(c.contains(".mlg_data[mallang_slice_append_tmp_"));
    assert!(c.contains(" = 3;"));
    assert!(c.contains("mlg_values = mallang_slice_append_tmp_"));
    assert!(c.contains("mlg_drop_Slice_int(&(mlg_values));"));
}

#[test]
fn generates_c_for_slice_field_append_reassignment() {
    let program = parse(
        r#"
type Bag struct {
values []int
}

func main() {
mut bag := Bag{values: []int{1, 2}}
bag.values = append(bag.values, 3)
print(bag.values[2])
}
"#,
    )
    .unwrap();
    let checked = check(&program).unwrap();
    let ir = lower(&checked).unwrap();
    let c = generate_c_from_ir(&ir).unwrap();

    assert!(c.contains("mlg_Slice_int mallang_slice_append_tmp_"));
    assert!(c.contains("(mlg_bag).mlg_values = mallang_slice_append_tmp_"));
    assert!(!c.contains("mlg_drop_Slice_int(&((mlg_bag).mlg_values));"));
    assert!(c.contains("mlg_drop_Struct_Bag(&(mlg_bag));"));
}

#[test]
fn generates_c_for_slice_field_append_take_source() {
    let program = parse(
        r#"
type Bag struct {
values []int
}

func main() {
mut bag := Bag{values: []int{1}}
grown := append(bag.values, 2)
print(len(grown))
print(len(bag.values))
}
"#,
    )
    .unwrap();
    let checked = check(&program).unwrap();
    let ir = lower(&checked).unwrap();
    let c = generate_c_from_ir(&ir).unwrap();

    assert!(c.contains("mlg_Slice_int mallang_slice_append_tmp_"));
    assert!(c.contains("mallang_slice_append_tmp_"));
    assert!(c.contains(" = (mlg_bag).mlg_values;"));
    assert!(c.contains(
        "(mlg_bag).mlg_values = (mlg_Slice_int){ .mlg_data = NULL, .mlg_len = 0, .mlg_cap = 0 };"
    ));
    assert!(c.contains("mlg_Slice_int mlg_grown = mallang_slice_append_tmp_"));
    assert!(c.contains("mlg_drop_Slice_int(&(mlg_grown));"));
    assert!(c.contains("mlg_drop_Struct_Bag(&(mlg_bag));"));
}

#[test]
fn generates_c_for_owned_slice_field_take_expression() {
    let program = parse(
        r#"
type Bag struct {
values []int
}

func main() {
bag := Bag{values: []int{1, 2}}
taken := bag.values
print(len(bag.values))
consume(bag.values)
}

func consume(values []int) {
print(len(values))
}
"#,
    )
    .unwrap();
    let checked = check(&program).unwrap();
    let ir = lower(&checked).unwrap();
    let c = generate_c_from_ir(&ir).unwrap();

    assert!(c.contains("mlg_Slice_int mallang_slice_take_tmp_"));
    assert!(c.contains(" = (mlg_bag).mlg_values;"));
    assert!(c.contains(
        "(mlg_bag).mlg_values = (mlg_Slice_int){ .mlg_data = NULL, .mlg_len = 0, .mlg_cap = 0 };"
    ));
    assert!(c.contains("mlg_Slice_int mlg_taken = mallang_slice_take_tmp_"));
    assert!(c.contains("mlg_consume(mallang_slice_take_tmp_"));
    assert!(c.contains("mlg_drop_Slice_int(&(mlg_taken));"));
    assert!(c.contains("mlg_drop_Struct_Bag(&(mlg_bag));"));
}

#[test]
fn generates_c_for_indexed_slice_field_append_reassignment() {
    let program = parse(
        r#"
type Bag struct {
values []int
}

type Store struct {
bags []Bag
}

func main() {
mut store := Store{bags: []Bag{Bag{values: []int{1}}, Bag{values: []int{2}}}}
i := 1
store.bags[i].values = append(store.bags[i].values, 3)
print(store.bags[i].values[1])
}
"#,
    )
    .unwrap();
    let checked = check(&program).unwrap();
    let ir = lower(&checked).unwrap();
    let c = generate_c_from_ir(&ir).unwrap();

    assert!(c.contains("mlg_Slice_int mallang_slice_append_tmp_"));
    assert!(c.contains(".mlg_values = mallang_slice_append_tmp_"));
    assert!(c.contains("mallang_runtime_error(\"slice index out of bounds\")"));
    assert!(!c.contains("mlg_drop_Slice_int(&(((mlg_store).mlg_bags"));
    assert!(c.contains("mlg_drop_Struct_Store(&(mlg_store));"));
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
    assert!(c.contains("mallang_runtime_error(\"slice index out of bounds\")"));
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
fn generates_c_for_local_rooted_slice_element_assignment() {
    let program = parse(
        r#"
type Bag struct {
values []int
}

type Store struct {
bags []Bag
}

func main() {
mut bag := Bag{values: []int{1, 2}}
bag.values[1] = 5

mut store := Store{bags: []Bag{Bag{values: []int{3}}, Bag{values: []int{4}}}}
store.bags[0] = Bag{values: []int{7, 8}}
}
"#,
    )
    .unwrap();
    let checked = check(&program).unwrap();
    let ir = lower(&checked).unwrap();
    let c = generate_c_from_ir(&ir).unwrap();

    assert!(c.contains(">= ((mlg_bag).mlg_values).mlg_len"));
    assert!(c.contains("((mlg_bag).mlg_values).mlg_data[mallang_index_assign_value_"));
    assert!(c.contains("] = 5;"));
    assert!(c.contains("mlg_struct_Bag mlg_mallang_cleanup_assign_rhs_"));
    assert!(
        c.contains("mlg_drop_Struct_Bag(&(((mlg_store).mlg_bags).mlg_data[mallang_index_value_")
    );
    assert!(c.contains("((mlg_store).mlg_bags).mlg_data[mallang_index_assign_value_"));
    assert!(c.contains("] = mlg_mallang_cleanup_assign_rhs_"));
    assert!(c.contains("mlg_drop_Struct_Store(&(mlg_store));"));
    assert!(c.contains("mlg_drop_Struct_Bag(&(mlg_bag));"));
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
    assert!(c.contains("mlg_drop_Struct_User(&((mlg_users).mlg_data[mallang_check_index(1, 2)]));"));
    assert!(c.contains("(mlg_users).mlg_data[mallang_index_assign_value_"));
    assert!(c.contains("] = mlg_mallang_cleanup_assign_rhs_"));
    assert!(c.contains("mlg_show(&(((mlg_users).mlg_data[mallang_check_index(1, 2)]).mlg_name));"));
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
    assert!(c.contains("mlg_Counter_inc(&((mlg_counters).mlg_data[mallang_check_index(1, 2)]));"));
    assert!(
        c.contains("mlg_show(&(((mlg_counters).mlg_data[mallang_check_index(1, 2)]).mlg_value));")
    );
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
    assert!(c.contains("mallang_runtime_error(\"slice index out of bounds\")"));
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
    assert!(c.contains("mlg_show(&(((mlg_users).mlg_data[mallang_check_index(0, 2)]).mlg_name));"));
    assert!(
        c.contains("mlg_rename(&(((mlg_users).mlg_data[mallang_check_index(1, 2)]).mlg_name));")
    );
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
    assert!(c.contains("mallang_runtime_error(\"slice index out of bounds\")"));
    assert!(c.contains("mlg_show(&(((mlg_users).mlg_data[mallang_index_value_"));
    assert!(c.contains("mlg_rename(&(((mlg_users).mlg_data[mallang_index_value_"));
    assert!(c.contains("]).mlg_name));"));
    assert!(c.contains("(*mlg_name) = \"park\";"));
    assert!(c.contains("mlg_drop_Slice_Struct_User(&(mlg_users));"));
}
