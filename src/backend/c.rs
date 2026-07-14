use std::fmt;

mod expressions;
mod names;
mod statements;
mod types;
mod utils;

use names::{c_ident, c_param_decl, callable_thunk_name, TypeCName};
use types::{collect_defined_types, emit_drop_helpers, emit_type_definitions};
use utils::{param_env, push_indented_lines, runtime_error_call};

use crate::{
    ast::Program,
    ir::{lower, IrFunction, IrProgram},
    semantic::{check, FunctionParamType, FunctionType, Type},
};

pub fn generate_c(program: &Program) -> Result<String, CompileError> {
    let checked = check(program).map_err(|error| CompileError::new(error.to_string()))?;
    let ir = lower(&checked).map_err(|error| CompileError::new(error.to_string()))?;
    generate_c_from_ir(&ir)
}

pub fn generate_c_from_ir(program: &IrProgram) -> Result<String, CompileError> {
    CGenerator::new(program).generate()
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
}

struct AppendSourceExpr {
    prelude: Vec<String>,
    code: String,
    clear_source: Option<String>,
}

impl CExpr {
    fn simple(code: String) -> Self {
        Self {
            prelude: Vec::new(),
            code,
        }
    }
}

impl<'a> CGenerator<'a> {
    fn new(program: &'a IrProgram) -> Self {
        Self { program }
    }

    fn generate(self) -> Result<String, CompileError> {
        let mut output = String::new();
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
        output.push_str("static void MLG_UNUSED mallang_runtime_error(const char *message) {\n");
        output.push_str("    fprintf(stderr, \"mallang runtime error: %s\\n\", message);\n");
        output.push_str("    exit(1);\n");
        output.push_str("}\n\n");
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

        for function in &self.program.functions {
            output.push_str(&self.emit_function(function)?);
            output.push('\n');
        }

        Ok(output)
    }

    fn prototype(&self, function: &IrFunction) -> Result<String, CompileError> {
        let params = if function.name == "main" || function.params.is_empty() {
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

    fn struct_def(&self, name: &str) -> Result<&crate::ir::IrStruct, CompileError> {
        self.program
            .structs
            .iter()
            .find(|struct_def| struct_def.name == name)
            .ok_or_else(|| {
                CompileError::new(format!("IR invariant violation: unknown struct `{name}`"))
            })
    }
}

#[cfg(test)]
mod tests;
