use std::{collections::HashSet, fmt};

mod expressions;
mod names;
mod patterns;
mod platform_runtime;
mod runtime;
mod standard_runtime;
mod statements;
mod types;
mod utils;
mod validate;

use names::{
    c_field, c_ident, c_param_decl, callable_thunk_name, closure_call_name, closure_drop_name,
    closure_env_type_name, drop_fn_name, TypeCName,
};
use runtime::{emit_allocation_runtime, emit_string_runtime};
use standard_runtime::{emit_standard_runtime, program_uses_intrinsic};
use types::{collect_defined_types, emit_drop_helpers, emit_type_definitions};
use utils::{param_env, param_env_from_params, push_indented_lines, runtime_error_call};
use validate::validate_program;

use crate::{
    ast::Program,
    ir::{lower, IrClosure, IrEnum, IrFunction, IrProgram, IrStmt, IrStmtKind, IrTestRunner},
    semantic::{check, FunctionParamType, FunctionType, Type},
    standard::StandardIntrinsic,
};

pub fn generate_c(program: &Program) -> Result<String, CompileError> {
    let checked = check(program).map_err(|error| CompileError::new(error.to_string()))?;
    let ir = lower(&checked).map_err(|error| CompileError::new(error.to_string()))?;
    generate_c_from_ir(&ir)
}

pub fn generate_c_from_ir(program: &IrProgram) -> Result<String, CompileError> {
    CGenerator::new(program).generate()
}

pub(crate) fn generate_c_test_runner_from_ir(
    runner: &IrTestRunner,
) -> Result<String, CompileError> {
    CGenerator::new(&runner.program).generate_test_runner(&runner.test_functions)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
}

impl CompileError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompileError {}

struct CGenerator<'a> {
    program: &'a IrProgram,
}

struct CExpr {
    prelude: Vec<String>,
    code: String,
    postlude: Vec<String>,
}

struct AppendSourceExpr {
    prelude: Vec<String>,
    code: String,
    clear_source: Option<String>,
    postlude: Vec<String>,
}

impl CExpr {
    fn simple(code: String) -> Self {
        Self {
            prelude: Vec::new(),
            code,
            postlude: Vec::new(),
        }
    }
}

impl<'a> CGenerator<'a> {
    fn new(program: &'a IrProgram) -> Self {
        Self { program }
    }

    fn generate(self) -> Result<String, CompileError> {
        self.generate_with_test_runner(None)
    }

    fn generate_test_runner(self, test_functions: &[String]) -> Result<String, CompileError> {
        validate_test_runner(self.program, test_functions)?;
        self.generate_with_test_runner(Some(test_functions))
    }

    fn generate_with_test_runner(
        self,
        test_functions: Option<&[String]>,
    ) -> Result<String, CompileError> {
        validate_program(self.program)?;
        let mut output = String::new();
        output.push_str("#include <errno.h>\n");
        output.push_str("#include <stdbool.h>\n");
        output.push_str("#include <stdint.h>\n");
        output.push_str("#include <stdio.h>\n");
        output.push_str("#include <stdlib.h>\n");
        output.push_str("#include <string.h>\n\n");
        output.push_str("#if defined(__GNUC__) || defined(__clang__)\n");
        output.push_str("#define MLG_UNUSED __attribute__((unused))\n");
        output.push_str("#else\n");
        output.push_str("#define MLG_UNUSED\n");
        output.push_str("#endif\n\n");
        output.push_str(
            "static _Noreturn void MLG_UNUSED mallang_runtime_error(const char *message) {\n",
        );
        output.push_str("    fprintf(stderr, \"mallang runtime error: %s\\n\", message);\n");
        output.push_str("    exit(1);\n");
        output.push_str("}\n\n");
        if program_uses_test_assertion(self.program) {
            output.push_str(
                "static _Noreturn void MLG_UNUSED mallang_test_assertion_failed(size_t source_id, size_t offset) {\n",
            );
            output.push_str(
                "    fprintf(stderr, \"__mlg_test_assert:%zu:%zu\\n\", source_id, offset);\n",
            );
            output.push_str("    exit(1);\n");
            output.push_str("}\n\n");
        }
        output.push_str(&emit_allocation_runtime());
        output.push_str(
            "static int64_t MLG_UNUSED mallang_check_index(int64_t index, int64_t len) {\n",
        );
        output.push_str("    if (index < 0 || index >= len) {\n");
        push_indented_lines(
            &mut output,
            &runtime_error_call("array index out of bounds"),
            2,
        );
        output.push_str("    }\n");
        output.push_str("    return index;\n");
        output.push_str("}\n\n");

        let defined_types = collect_defined_types(self.program);
        output.push_str(&emit_string_runtime(&defined_types));
        let type_definitions = emit_type_definitions(self.program, &defined_types)?;
        output.push_str(&type_definitions);
        if !type_definitions.is_empty() {
            output.push('\n');
        }

        let drop_helpers = emit_drop_helpers(self.program, &defined_types)?;
        output.push_str(&drop_helpers);
        if !drop_helpers.is_empty() {
            output.push('\n');
        }

        let standard_runtime = emit_standard_runtime(self.program)?;
        output.push_str(&standard_runtime);
        if !standard_runtime.is_empty() {
            output.push('\n');
        }

        let closure_environment_types = self.emit_closure_environment_types();
        output.push_str(&closure_environment_types);
        if !closure_environment_types.is_empty() {
            output.push('\n');
        }

        let closure_drop_helpers = self.emit_closure_drop_helpers();
        output.push_str(&closure_drop_helpers);
        if !closure_drop_helpers.is_empty() {
            output.push('\n');
        }

        for function in &self.program.functions {
            output.push_str(&self.prototype(function)?);
            output.push_str(";\n");
        }
        output.push('\n');

        for function in self
            .program
            .functions
            .iter()
            .filter(|function| function.name != "main")
        {
            output.push_str(&self.emit_callable_thunk(function));
            output.push('\n');
        }

        for closure in &self.program.closures {
            output.push_str(&self.emit_closure_function(closure)?);
            output.push('\n');
        }

        for function in &self.program.functions {
            output.push_str(&self.emit_function(function)?);
            output.push('\n');
        }

        if let Some(test_functions) = test_functions {
            output.push_str(&self.emit_test_runner(test_functions));
            output.push('\n');
        }

        Ok(output)
    }

    fn emit_test_runner(&self, test_functions: &[String]) -> String {
        let mut output = String::from(
            r#"int main(int argc, char **argv) {
    if (argc != 2 || argv == NULL || argv[1] == NULL || argv[1][0] == '\0') {
        mallang_runtime_error("invalid test runner invocation");
    }
    for (const char *cursor = argv[1]; *cursor != '\0'; cursor++) {
        if (*cursor < '0' || *cursor > '9') {
            mallang_runtime_error("invalid test runner invocation");
        }
    }
    errno = 0;
    char *end = NULL;
    unsigned long long test_case = strtoull(argv[1], &end, 10);
    if (errno == ERANGE || end == argv[1] || *end != '\0') {
        mallang_runtime_error("invalid test runner invocation");
    }
"#,
        );
        if program_uses_intrinsic(self.program, StandardIntrinsic::OsArgs) {
            output.push_str("    mallang_process_init(argc - 1, argv);\n");
        }
        output.push_str("    switch (test_case) {\n");
        for (runner_case, function_name) in test_functions.iter().enumerate() {
            output.push_str(&format!(
                "        case {runner_case}: {}(); return 0;\n",
                c_ident(function_name)
            ));
        }
        output.push_str(
            r#"        default: mallang_runtime_error("unknown test runner case");
    }
}
"#,
        );
        output
    }

    fn prototype(&self, function: &IrFunction) -> Result<String, CompileError> {
        let params = if function.name == "main"
            && program_uses_intrinsic(self.program, StandardIntrinsic::OsArgs)
        {
            "int argc, char **argv".to_string()
        } else if function.name == "main" || function.params.is_empty() {
            "void".to_string()
        } else {
            function
                .params
                .iter()
                .map(c_param_decl)
                .collect::<Vec<_>>()
                .join(", ")
        };

        let return_type = if function.name == "main" {
            "int".to_string()
        } else {
            function.return_type.c_name()
        };

        Ok(format!(
            "{} {}({})",
            return_type,
            c_ident(&function.name),
            params
        ))
    }

    fn emit_function(&self, function: &IrFunction) -> Result<String, CompileError> {
        let mut output = String::new();
        output.push_str(&self.prototype(function)?);
        output.push_str(" {\n");
        let env = param_env(function);

        if function.name == "main"
            && program_uses_intrinsic(self.program, StandardIntrinsic::OsArgs)
        {
            output.push_str("    mallang_process_init(argc, argv);\n");
        }

        for param in &function.params {
            output.push_str(&format!("    (void){};\n", c_ident(&param.name)));
        }

        for stmt in &function.body {
            let line = self.emit_stmt_with_env(stmt, &env)?;
            push_indented_lines(&mut output, &line, 1);
        }

        if function.name == "main" {
            output.push_str("    return 0;\n");
        }

        output.push_str("}\n");
        Ok(output)
    }

    fn emit_callable_thunk(&self, function: &IrFunction) -> String {
        let mut params = vec!["void *mlg_env".to_string()];
        params.extend(function.params.iter().enumerate().map(|(index, param)| {
            format!("{} mlg_arg_{index}", param.ty.c_param_type(param.mode))
        }));
        let args = (0..function.params.len())
            .map(|index| format!("mlg_arg_{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let mut output = format!(
            "static {} MLG_UNUSED {}({}) {{\n    (void)mlg_env;\n",
            function.return_type.c_name(),
            callable_thunk_name(&function.name),
            params.join(", ")
        );
        if function.return_type == crate::semantic::Type::Unit {
            output.push_str(&format!("    {}({args});\n", c_ident(&function.name)));
        } else {
            output.push_str(&format!(
                "    return {}({args});\n",
                c_ident(&function.name)
            ));
        }
        output.push_str("}\n");
        output
    }

    fn emit_closure_environment_types(&self) -> String {
        let mut output = String::new();
        for closure in self
            .program
            .closures
            .iter()
            .filter(|closure| !closure.captures.is_empty())
        {
            output.push_str("typedef struct {\n");
            for capture in &closure.captures {
                output.push_str(&format!(
                    "    {} {};\n",
                    capture.ty.c_name(),
                    c_field(&capture.name)
                ));
            }
            output.push_str(&format!("}} {};\n\n", closure_env_type_name(&closure.name)));
        }
        output
    }

    fn emit_closure_drop_helpers(&self) -> String {
        let mut output = String::new();
        for closure in self
            .program
            .closures
            .iter()
            .filter(|closure| !closure.captures.is_empty())
        {
            let env_type = closure_env_type_name(&closure.name);
            output.push_str(&format!(
                "static void MLG_UNUSED {}(void *mlg_raw_env) {{\n",
                closure_drop_name(&closure.name)
            ));
            output.push_str("    if (mlg_raw_env == NULL) {\n        return;\n    }\n");
            output.push_str(&format!(
                "    {env_type} *mlg_env = ({env_type} *)mlg_raw_env;\n"
            ));
            for capture in &closure.captures {
                if capture.ty.needs_cleanup() {
                    output.push_str(&format!(
                        "    {}(&(mlg_env->{}));\n",
                        drop_fn_name(&capture.ty),
                        c_field(&capture.name)
                    ));
                }
            }
            output.push_str("    mallang_dealloc(mlg_env);\n}\n\n");
        }
        output
    }

    fn emit_closure_function(&self, closure: &IrClosure) -> Result<String, CompileError> {
        let mut params = vec!["void *mlg_raw_env".to_string()];
        params.extend(closure.params.iter().map(c_param_decl));
        let mut output = format!(
            "static {} MLG_UNUSED {}({}) {{\n",
            closure.return_type.c_name(),
            closure_call_name(&closure.name),
            params.join(", ")
        );
        let mut env = param_env_from_params(&closure.params);
        if closure.captures.is_empty() {
            output.push_str("    (void)mlg_raw_env;\n");
        } else {
            let env_type = closure_env_type_name(&closure.name);
            output.push_str(&format!(
                "    {env_type} *mlg_env = ({env_type} *)mlg_raw_env;\n"
            ));
            for capture in &closure.captures {
                env.insert(
                    capture.name.clone(),
                    format!("(mlg_env->{})", c_field(&capture.name)),
                );
            }
        }
        for param in &closure.params {
            output.push_str(&format!("    (void){};\n", c_ident(&param.name)));
        }
        for stmt in &closure.body {
            let line = self.emit_stmt_with_env(stmt, &env)?;
            push_indented_lines(&mut output, &line, 1);
        }
        output.push_str("}\n");
        Ok(output)
    }

    fn function_def(&self, name: &str) -> Result<&IrFunction, CompileError> {
        self.program
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or_else(|| {
                CompileError::new(format!(
                    "IR invariant violation: unknown function value `{name}`"
                ))
            })
    }

    fn callable_type(function: &IrFunction) -> Type {
        Type::Function(FunctionType {
            mutable: false,
            params: function
                .params
                .iter()
                .map(|param| FunctionParamType {
                    mode: param.mode,
                    ty: param.ty.clone(),
                })
                .collect(),
            return_type: Box::new(function.return_type.clone()),
        })
    }

    fn closure_callable_type(closure: &IrClosure) -> Type {
        Type::Function(FunctionType {
            mutable: closure.mutable,
            params: closure
                .params
                .iter()
                .map(|param| FunctionParamType {
                    mode: param.mode,
                    ty: param.ty.clone(),
                })
                .collect(),
            return_type: Box::new(closure.return_type.clone()),
        })
    }

    fn closure_def(&self, name: &str) -> Result<&IrClosure, CompileError> {
        self.program
            .closures
            .iter()
            .find(|closure| closure.name == name)
            .ok_or_else(|| {
                CompileError::new(format!(
                    "IR invariant violation: unknown closure value `{name}`"
                ))
            })
    }

    fn struct_def(&self, name: &str) -> Result<&crate::ir::IrStruct, CompileError> {
        self.program
            .structs
            .iter()
            .find(|struct_def| struct_def.name == name)
            .ok_or_else(|| {
                CompileError::new(format!("IR invariant violation: unknown struct `{name}`"))
            })
    }

    fn enum_def(&self, name: &str) -> Result<&IrEnum, CompileError> {
        self.program
            .enums
            .iter()
            .find(|enum_def| enum_def.name == name)
            .ok_or_else(|| {
                CompileError::new(format!("IR invariant violation: unknown enum `{name}`"))
            })
    }
}

fn validate_test_runner(
    program: &IrProgram,
    test_functions: &[String],
) -> Result<(), CompileError> {
    if program
        .functions
        .iter()
        .any(|function| function.name == "main")
    {
        return Err(CompileError::new(
            "IR invariant violation: test runner must not contain application `main`",
        ));
    }
    if test_functions.is_empty() {
        return Err(CompileError::new(
            "IR invariant violation: test runner must contain at least one test",
        ));
    }

    let mut seen = HashSet::new();
    for function_name in test_functions {
        if !seen.insert(function_name.as_str()) {
            return Err(CompileError::new(format!(
                "IR invariant violation: duplicate test runner function `{function_name}`"
            )));
        }
        let function = program
            .functions
            .iter()
            .find(|function| function.name == *function_name)
            .ok_or_else(|| {
                CompileError::new(format!(
                    "IR invariant violation: unknown test runner function `{function_name}`"
                ))
            })?;
        if !function.params.is_empty() || function.return_type != Type::Unit {
            return Err(CompileError::new(format!(
                "IR invariant violation: test runner function `{function_name}` must have no parameters and return `unit`"
            )));
        }
    }
    Ok(())
}

fn program_uses_test_assertion(program: &IrProgram) -> bool {
    program
        .functions
        .iter()
        .any(|function| statements_use_test_assertion(&function.body))
        || program
            .closures
            .iter()
            .any(|closure| statements_use_test_assertion(&closure.body))
}

fn statements_use_test_assertion(statements: &[IrStmt]) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        IrStmtKind::Assert { .. } => true,
        IrStmtKind::If {
            then_body,
            else_body,
            ..
        } => statements_use_test_assertion(then_body) || statements_use_test_assertion(else_body),
        IrStmtKind::For { body, cleanup, .. } | IrStmtKind::RangeFor { body, cleanup, .. } => {
            statements_use_test_assertion(body) || statements_use_test_assertion(cleanup)
        }
        IrStmtKind::Match { arms, .. } => arms
            .iter()
            .any(|arm| statements_use_test_assertion(&arm.body)),
        IrStmtKind::Let { .. }
        | IrStmtKind::Assign { .. }
        | IrStmtKind::FieldAssign { .. }
        | IrStmtKind::IndexAssign { .. }
        | IrStmtKind::Overwrite { .. }
        | IrStmtKind::Return { .. }
        | IrStmtKind::Drop { .. }
        | IrStmtKind::Break
        | IrStmtKind::Continue
        | IrStmtKind::Expr { .. } => false,
    })
}

#[cfg(test)]
mod tests;
