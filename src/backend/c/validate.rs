use std::collections::HashSet;

use crate::{
    ir::{IrClosure, IrFunction, IrProgram},
    semantic::Type,
};

use super::CompileError;

pub(super) fn validate_program(program: &IrProgram) -> Result<(), CompileError> {
    validate_types(program)?;
    validate_callables(program)?;
    Ok(())
}

fn validate_types(program: &IrProgram) -> Result<(), CompileError> {
    let mut names = HashSet::new();
    for declaration in &program.structs {
        if !names.insert(declaration.name.as_str()) {
            return invariant(format!("duplicate type declaration `{}`", declaration.name));
        }
        reject_duplicate_names(
            declaration.fields.iter().map(|field| field.name.as_str()),
            "struct field",
            &declaration.name,
        )?;
    }
    for declaration in &program.enums {
        if !names.insert(declaration.name.as_str()) {
            return invariant(format!("duplicate type declaration `{}`", declaration.name));
        }
        reject_duplicate_names(
            declaration
                .variants
                .iter()
                .map(|variant| variant.name.as_str()),
            "enum variant",
            &declaration.name,
        )?;
    }
    Ok(())
}

fn validate_callables(program: &IrProgram) -> Result<(), CompileError> {
    let mut function_names = HashSet::new();
    for function in &program.functions {
        if !function_names.insert(function.name.as_str()) {
            return invariant(format!("duplicate function `{}`", function.name));
        }
        validate_function(function)?;
    }

    if let Some(main) = program
        .functions
        .iter()
        .find(|function| function.name == "main")
    {
        if !main.params.is_empty() || main.return_type != Type::Unit {
            return invariant("`main` must have no parameters and return `unit`");
        }
    }

    let mut closure_names = HashSet::new();
    for closure in &program.closures {
        if !closure_names.insert(closure.name.as_str()) {
            return invariant(format!("duplicate closure `{}`", closure.name));
        }
        validate_closure(closure)?;
    }
    Ok(())
}

fn validate_function(function: &IrFunction) -> Result<(), CompileError> {
    reject_duplicate_names(
        function.params.iter().map(|param| param.name.as_str()),
        "parameter",
        &function.name,
    )
}

fn validate_closure(closure: &IrClosure) -> Result<(), CompileError> {
    reject_duplicate_names(
        closure.captures.iter().map(|capture| capture.name.as_str()),
        "closure capture",
        &closure.name,
    )?;
    reject_duplicate_names(
        closure.params.iter().map(|param| param.name.as_str()),
        "parameter",
        &closure.name,
    )
}

fn reject_duplicate_names<'a>(
    names: impl Iterator<Item = &'a str>,
    kind: &str,
    owner: &str,
) -> Result<(), CompileError> {
    let mut seen = HashSet::new();
    for name in names {
        if !seen.insert(name) {
            return invariant(format!("duplicate {kind} `{name}` in `{owner}`"));
        }
    }
    Ok(())
}

fn invariant<T>(message: impl Into<String>) -> Result<T, CompileError> {
    Err(CompileError::new(format!(
        "IR invariant violation: {}",
        message.into()
    )))
}
