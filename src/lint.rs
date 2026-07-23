use crate::{
    ast::{Block, Expr, ExprKind, ForInit, Function, MatchPattern, Program, StmtKind, TestDecl},
    Span,
};

pub const TYPE_CASE_RULE: &str = "MLG-NAME-001";
pub const VARIANT_CASE_RULE: &str = "MLG-NAME-002";
pub const TYPE_PARAM_CASE_RULE: &str = "MLG-NAME-003";
pub const FUNCTION_CASE_RULE: &str = "MLG-NAME-004";
pub const BINDING_CASE_RULE: &str = "MLG-NAME-005";
pub const FIELD_CASE_RULE: &str = "MLG-NAME-006";
pub const TEST_CASE_RULE: &str = "MLG-NAME-007";
pub const PACKAGE_CASE_RULE: &str = "MLG-NAME-008";
pub const PROJECT_CASE_RULE: &str = "MLG-NAME-009";

pub const NAME_RULE_IDS: &[&str] = &[
    TYPE_CASE_RULE,
    VARIANT_CASE_RULE,
    TYPE_PARAM_CASE_RULE,
    FUNCTION_CASE_RULE,
    BINDING_CASE_RULE,
    FIELD_CASE_RULE,
    TEST_CASE_RULE,
    PACKAGE_CASE_RULE,
    PROJECT_CASE_RULE,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFinding {
    pub rule_id: &'static str,
    pub name: String,
    pub span: Span,
}

impl LintFinding {
    pub fn message(&self) -> String {
        naming_message(self.rule_id, &self.name)
            .expect("LintFinding rules must be declared in NAME_RULE_IDS")
    }
}

pub fn naming_message(rule_id: &str, name: &str) -> Option<String> {
    let (role, spelling) = match rule_id {
        TYPE_CASE_RULE => ("type name", "PascalCase"),
        VARIANT_CASE_RULE => ("enum variant", "PascalCase"),
        TYPE_PARAM_CASE_RULE => ("type parameter", "PascalCase"),
        FUNCTION_CASE_RULE => ("function or method", "lowerCamelCase"),
        BINDING_CASE_RULE => ("binding", "lowerCamelCase"),
        FIELD_CASE_RULE => ("field", "lowerCamelCase"),
        TEST_CASE_RULE => ("test name", "PascalCase"),
        PACKAGE_CASE_RULE => ("package name", "lower_snake_case"),
        PROJECT_CASE_RULE => ("project name", "lower_snake_case"),
        _ => return None,
    };
    Some(format!("{role} `{name}` should use {spelling}"))
}

pub fn lint_program_names(program: &Program) -> Vec<LintFinding> {
    let mut findings = Vec::new();

    for source_unit in &program.source_units {
        if let Some(package) = &source_unit.package {
            check_name(
                &mut findings,
                PACKAGE_CASE_RULE,
                &package.name,
                package.span,
                is_lower_snake_case,
            );
        }
    }
    for declaration in &program.structs {
        check_name(
            &mut findings,
            TYPE_CASE_RULE,
            &declaration.name,
            declaration.span,
            is_pascal_case,
        );
        for parameter in &declaration.type_params {
            check_name(
                &mut findings,
                TYPE_PARAM_CASE_RULE,
                &parameter.name,
                parameter.span,
                is_pascal_case,
            );
        }
        for field in &declaration.fields {
            check_name(
                &mut findings,
                FIELD_CASE_RULE,
                &field.name,
                field.span,
                is_lower_camel_case,
            );
        }
    }
    for declaration in &program.enums {
        check_name(
            &mut findings,
            TYPE_CASE_RULE,
            &declaration.name,
            declaration.span,
            is_pascal_case,
        );
        for parameter in &declaration.type_params {
            check_name(
                &mut findings,
                TYPE_PARAM_CASE_RULE,
                &parameter.name,
                parameter.span,
                is_pascal_case,
            );
        }
        for variant in &declaration.variants {
            check_name(
                &mut findings,
                VARIANT_CASE_RULE,
                &variant.name,
                variant.span,
                is_pascal_case,
            );
        }
    }
    for function in &program.functions {
        lint_function(&mut findings, function);
    }
    for test in &program.tests {
        lint_test(&mut findings, test);
    }

    findings.sort_by_key(|finding| {
        (
            finding.span.source.index(),
            finding.span.start,
            finding.span.end,
            finding.rule_id,
        )
    });
    findings
}

pub fn is_known_name_rule(rule_id: &str) -> bool {
    canonical_name_rule(rule_id).is_some()
}

pub fn canonical_name_rule(rule_id: &str) -> Option<&'static str> {
    NAME_RULE_IDS
        .iter()
        .copied()
        .find(|candidate| *candidate == rule_id)
}

pub fn is_lower_snake_case(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut previous_underscore = false;
    for byte in bytes {
        if byte == b'_' {
            if previous_underscore {
                return false;
            }
            previous_underscore = true;
        } else if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_underscore = false;
        } else {
            return false;
        }
    }
    !previous_underscore
}

pub fn is_pascal_case(name: &str) -> bool {
    is_camel_case(name, true)
}

pub fn is_lower_camel_case(name: &str) -> bool {
    is_camel_case(name, false)
}

fn is_camel_case(name: &str, upper_first: bool) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if (upper_first && !first.is_ascii_uppercase()) || (!upper_first && !first.is_ascii_lowercase())
    {
        return false;
    }
    let mut previous_upper = first.is_ascii_uppercase();
    for byte in bytes {
        if !byte.is_ascii_alphanumeric() {
            return false;
        }
        let upper = byte.is_ascii_uppercase();
        if previous_upper && upper {
            return false;
        }
        previous_upper = upper;
    }
    true
}

fn check_name(
    findings: &mut Vec<LintFinding>,
    rule_id: &'static str,
    name: &str,
    span: Span,
    predicate: fn(&str) -> bool,
) {
    if name != "_" && !predicate(name) {
        findings.push(LintFinding {
            rule_id,
            name: name.to_string(),
            span,
        });
    }
}

fn lint_function(findings: &mut Vec<LintFinding>, function: &Function) {
    check_name(
        findings,
        FUNCTION_CASE_RULE,
        &function.name,
        function.span,
        is_lower_camel_case,
    );
    for parameter in &function.type_params {
        check_name(
            findings,
            TYPE_PARAM_CASE_RULE,
            &parameter.name,
            parameter.span,
            is_pascal_case,
        );
    }
    if let Some(receiver) = &function.receiver {
        check_name(
            findings,
            BINDING_CASE_RULE,
            &receiver.name,
            receiver.span,
            is_lower_camel_case,
        );
    }
    for parameter in &function.params {
        check_name(
            findings,
            BINDING_CASE_RULE,
            &parameter.name,
            parameter.span,
            is_lower_camel_case,
        );
    }
    lint_block(findings, &function.body);
}

fn lint_test(findings: &mut Vec<LintFinding>, test: &TestDecl) {
    check_name(
        findings,
        TEST_CASE_RULE,
        &test.name,
        test.span,
        is_pascal_case,
    );
    lint_block(findings, &test.body);
}

fn lint_block(findings: &mut Vec<LintFinding>, block: &Block) {
    for statement in &block.statements {
        match &statement.kind {
            StmtKind::Let { name, expr, .. } => {
                check_name(
                    findings,
                    BINDING_CASE_RULE,
                    name,
                    statement.span,
                    is_lower_camel_case,
                );
                lint_expr(findings, expr);
            }
            StmtKind::Assign { expr, .. }
            | StmtKind::Return { expr }
            | StmtKind::Assert { condition: expr }
            | StmtKind::Expr { expr } => lint_expr(findings, expr),
            StmtKind::FieldAssign { base, expr, .. } => {
                lint_expr(findings, base);
                lint_expr(findings, expr);
            }
            StmtKind::IndexAssign { base, index, expr } => {
                lint_expr(findings, base);
                lint_expr(findings, index);
                lint_expr(findings, expr);
            }
            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                lint_expr(findings, condition);
                lint_block(findings, then_block);
                if let Some(else_block) = else_block {
                    lint_block(findings, else_block);
                }
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
            } => {
                if let Some(ForInit::Let { name, expr, .. }) = init {
                    check_name(
                        findings,
                        BINDING_CASE_RULE,
                        name,
                        statement.span,
                        is_lower_camel_case,
                    );
                    lint_expr(findings, expr);
                }
                if let Some(condition) = condition {
                    lint_expr(findings, condition);
                }
                if let Some(crate::ast::ForPost::Assign { target, expr }) = post {
                    lint_expr(findings, target);
                    lint_expr(findings, expr);
                }
                lint_block(findings, body);
            }
            StmtKind::RangeFor {
                index_name,
                value_name,
                source,
                body,
            } => {
                check_name(
                    findings,
                    BINDING_CASE_RULE,
                    index_name,
                    statement.span,
                    is_lower_camel_case,
                );
                check_name(
                    findings,
                    BINDING_CASE_RULE,
                    value_name,
                    statement.span,
                    is_lower_camel_case,
                );
                lint_expr(findings, source);
                lint_block(findings, body);
            }
            StmtKind::Match { scrutinee, arms } => {
                lint_expr(findings, scrutinee);
                for arm in arms {
                    lint_pattern(findings, &arm.pattern, arm.span);
                    lint_block(findings, &arm.block);
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
        }
    }
}

fn lint_expr(findings: &mut Vec<LintFinding>, expr: &Expr) {
    match &expr.kind {
        ExprKind::FunctionLiteral(function) => {
            for parameter in &function.params {
                check_name(
                    findings,
                    BINDING_CASE_RULE,
                    &parameter.name,
                    parameter.span,
                    is_lower_camel_case,
                );
            }
            lint_block(findings, &function.body);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            lint_expr(findings, condition);
            lint_expr(findings, then_branch);
            lint_expr(findings, else_branch);
        }
        ExprKind::Match { scrutinee, arms } => {
            lint_expr(findings, scrutinee);
            for arm in arms {
                lint_pattern(findings, &arm.pattern, arm.span);
                lint_expr(findings, &arm.expr);
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                lint_expr(findings, &field.expr);
            }
        }
        ExprKind::ArrayLiteral { elements, .. } => {
            for element in elements {
                lint_expr(findings, element);
            }
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::TypeApply { base, .. }
        | ExprKind::Unary { expr: base, .. } => lint_expr(findings, base),
        ExprKind::Index { base, index } => {
            lint_expr(findings, base);
            lint_expr(findings, index);
        }
        ExprKind::EnumConstructor { args, .. } => {
            if let Some(args) = args {
                for argument in args {
                    lint_expr(findings, &argument.expr);
                }
            }
        }
        ExprKind::Call { callee, args } => {
            lint_expr(findings, callee);
            for argument in args {
                lint_expr(findings, &argument.expr);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            lint_expr(findings, left);
            lint_expr(findings, right);
        }
        ExprKind::Int(_)
        | ExprKind::String(_)
        | ExprKind::Bool(_)
        | ExprKind::Nil
        | ExprKind::Var(_) => {}
    }
}

fn lint_pattern(findings: &mut Vec<LintFinding>, pattern: &MatchPattern, span: Span) {
    match pattern {
        MatchPattern::Some(name)
        | MatchPattern::Ok(name)
        | MatchPattern::Err(name)
        | MatchPattern::Binding(name) => {
            check_name(findings, BINDING_CASE_RULE, name, span, is_lower_camel_case)
        }
        MatchPattern::Variant { payloads, .. } => {
            for payload in payloads {
                lint_pattern(findings, payload, span);
            }
        }
        MatchPattern::NestedBuiltin { payload, .. } => lint_pattern(findings, payload, span),
        MatchPattern::None | MatchPattern::Wildcard => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn classifies_role_based_names_and_acronyms() {
        let program = parse(
            r#"package BadPackage

type http_client[t_item] struct {
    User_name string
}

type ResultCode enum {
    ok_value
    HttpOK
}

func ParseUTF8(con User_name string) {
    mut Bad_local := User_name
}

test parses_name() {}
"#,
        )
        .unwrap();

        let findings = lint_program_names(&program);
        let observed = findings
            .iter()
            .map(|finding| (finding.rule_id, finding.name.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            observed,
            vec![
                (PACKAGE_CASE_RULE, "BadPackage"),
                (TYPE_CASE_RULE, "http_client"),
                (TYPE_PARAM_CASE_RULE, "t_item"),
                (FIELD_CASE_RULE, "User_name"),
                (VARIANT_CASE_RULE, "ok_value"),
                (VARIANT_CASE_RULE, "HttpOK"),
                (FUNCTION_CASE_RULE, "ParseUTF8"),
                (BINDING_CASE_RULE, "User_name"),
                (BINDING_CASE_RULE, "Bad_local"),
                (TEST_CASE_RULE, "parses_name"),
            ]
        );
    }

    #[test]
    fn accepts_target_spellings_and_blank_bindings() {
        assert!(is_pascal_case("HttpClient"));
        assert!(is_lower_camel_case("parseUtf8"));
        assert!(is_lower_camel_case("apiUrl"));
        assert!(is_lower_snake_case("bootstrap_compiler"));
        assert!(!is_pascal_case("HTTPClient"));
        assert!(!is_lower_camel_case("parseUTF8"));
        assert!(!is_lower_snake_case("bootstrap-compiler"));
    }
}
