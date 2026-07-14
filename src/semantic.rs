use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};

use crate::{
    ast::{
        Arg, ArgMode, BinaryOp, Block, Expr, ExprKind, ForInit, ForPost, Function, FunctionLiteral,
        MatchArm, MatchBlockArm, MatchPattern, ParamMode, Program, Stmt, StmtKind, TypeRef,
        UnaryOp, Visibility,
    },
    package::PackageGraph,
    specialize::{specialize, specialize_for_validation},
    token::Span,
};

pub fn check(program: &Program) -> Result<CheckedProgram, SemanticError> {
    if needs_specialization(program) {
        validate_generic_bodies(program, None)?;
        let concrete =
            specialize(program).map_err(|error| SemanticError::new(error.message, error.span))?;
        return Checker::new(&concrete).check();
    }
    Checker::new(program).check()
}

pub fn check_project(
    program: &Program,
    package_graph: &PackageGraph,
) -> Result<CheckedProgram, SemanticError> {
    if needs_specialization(program) {
        validate_generic_bodies(program, Some(package_graph))?;
        let concrete =
            specialize(program).map_err(|error| SemanticError::new(error.message, error.span))?;
        return Checker::new_project(&concrete, package_graph).check();
    }
    Checker::new_project(program, package_graph).check()
}

fn validate_generic_bodies(
    program: &Program,
    package_graph: Option<&PackageGraph>,
) -> Result<(), SemanticError> {
    let symbolic = specialize_for_validation(program)
        .map_err(|error| SemanticError::new(error.message, error.span))?;
    let result = if let Some(package_graph) = package_graph {
        Checker::new_project(&symbolic.program, package_graph).check()
    } else {
        Checker::new(&symbolic.program).check()
    };
    result
        .map(|_| ())
        .map_err(|error| SemanticError::new(symbolic.display_message(&error.message), error.span))
}

fn needs_specialization(program: &Program) -> bool {
    program
        .structs
        .iter()
        .any(|declaration| !declaration.type_params.is_empty())
        || program
            .functions
            .iter()
            .any(|function| !function.type_params.is_empty())
}

#[derive(Debug, Clone)]
pub struct CheckedProgram {
    pub program: Arc<Program>,
    pub signatures: HashMap<String, FunctionSig>,
    pub methods: HashMap<MethodKey, MethodSig>,
    pub structs: HashMap<String, StructSig>,
    pub closures: Vec<CheckedClosure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedClosure {
    pub span: Span,
    pub literal: FunctionLiteral,
    pub function_type: FunctionType,
    pub captures: Vec<ClosureCapture>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureCapture {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSig {
    pub return_type: Type,
    pub params: Vec<ParamSig>,
}

impl FunctionSig {
    pub fn function_type(&self, mutable: bool) -> FunctionType {
        FunctionType {
            mutable,
            params: self
                .params
                .iter()
                .map(|param| FunctionParamType {
                    mode: param.mode,
                    ty: param.ty.clone(),
                })
                .collect(),
            return_type: Box::new(self.return_type.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodSig {
    pub receiver: ParamSig,
    pub function: FunctionSig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodKey {
    pub receiver: Type,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSig {
    pub name: String,
    pub mode: ParamMode,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructSig {
    pub fields: Vec<FieldSig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSig {
    pub name: String,
    pub ty: Type,
    pub ty_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub mutable: bool,
    pub params: Vec<FunctionParamType>,
    pub return_type: Box<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParamType {
    pub mode: ParamMode,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Bool,
    String,
    Unit,
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Array { len: usize, element: Box<Type> },
    Slice(Box<Type>),
    Struct(String),
    Function(FunctionType),
}

impl Type {
    pub fn source_name(&self) -> String {
        match self {
            Self::Int => "int".to_string(),
            Self::Bool => "bool".to_string(),
            Self::String => "string".to_string(),
            Self::Unit => "unit".to_string(),
            Self::Option(inner) => format!("Option[{}]", inner.source_name()),
            Self::Result(ok, err) => format!("Result[{}, {}]", ok.source_name(), err.source_name()),
            Self::Array { len, element } => format!("[{}]{}", len, element.source_name()),
            Self::Slice(element) => format!("[]{}", element.source_name()),
            Self::Struct(name) => name.clone(),
            Self::Function(function) => {
                let params = function
                    .params
                    .iter()
                    .map(|param| {
                        let mode = match param.mode {
                            ParamMode::Owned => "",
                            ParamMode::Con => "con ",
                            ParamMode::Mut => "mut ",
                        };
                        format!("{mode}{}", param.ty.source_name())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let mutable = if function.mutable { " mut" } else { "" };
                format!(
                    "func{mutable}({params}) {}",
                    function.return_type.source_name()
                )
            }
        }
    }

    pub fn is_copy(&self) -> bool {
        match self {
            Self::Int | Self::Bool | Self::Unit => true,
            Self::String => false,
            Self::Option(inner) => inner.is_copy(),
            Self::Result(ok, err) => ok.is_copy() && err.is_copy(),
            Self::Array { .. } | Self::Slice(_) => false,
            Self::Struct(_) => false,
            Self::Function(_) => false,
        }
    }

    pub fn needs_cleanup(&self) -> bool {
        match self {
            Self::Slice(_) => true,
            Self::Option(inner) => inner.needs_cleanup(),
            Self::Result(ok, err) => ok.needs_cleanup() || err.needs_cleanup(),
            Self::Array { element, .. } => element.needs_cleanup(),
            Self::Struct(_) => true,
            Self::Function(_) => true,
            Self::Int | Self::Bool | Self::String | Self::Unit => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub message: String,
    pub span: Span,
}

impl SemanticError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SemanticError {}

struct Checker<'a> {
    program: &'a Program,
    package_graph: Option<&'a PackageGraph>,
    signatures: HashMap<String, FunctionSig>,
    methods: HashMap<MethodKey, MethodSig>,
    method_access: HashMap<MethodKey, MethodAccess>,
    structs: HashMap<String, StructSig>,
    closures: RefCell<Vec<CheckedClosure>>,
}

struct MethodAccess {
    package_path: String,
    visibility: Visibility,
}

impl<'a> Checker<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            program,
            package_graph: None,
            signatures: HashMap::new(),
            methods: HashMap::new(),
            method_access: HashMap::new(),
            structs: HashMap::new(),
            closures: RefCell::new(Vec::new()),
        }
    }

    fn new_project(program: &'a Program, package_graph: &'a PackageGraph) -> Self {
        Self {
            package_graph: Some(package_graph),
            ..Self::new(program)
        }
    }

    fn check(mut self) -> Result<CheckedProgram, SemanticError> {
        self.reject_unlowered_generic_declarations()?;
        self.collect_structs()?;
        self.collect_signatures()?;
        for function in &self.program.functions {
            self.check_function(function)?;
        }

        Ok(CheckedProgram {
            program: Arc::new(self.program.clone()),
            signatures: self.signatures,
            methods: self.methods,
            structs: self.structs,
            closures: self.closures.into_inner(),
        })
    }

    fn reject_unlowered_generic_declarations(&self) -> Result<(), SemanticError> {
        if let Some(declaration) = self.program.enums.first() {
            return Err(SemanticError::new(
                "user-defined enum declarations require v0.4 semantic lowering",
                declaration.span,
            ));
        }
        if let Some(declaration) = self
            .program
            .structs
            .iter()
            .find(|declaration| !declaration.type_params.is_empty())
        {
            return Err(SemanticError::new(
                "generic struct declarations require v0.4 specialization",
                declaration.span,
            ));
        }
        if let Some(function) = self
            .program
            .functions
            .iter()
            .find(|function| !function.type_params.is_empty())
        {
            return Err(SemanticError::new(
                "generic function declarations require v0.4 specialization",
                function.span,
            ));
        }
        Ok(())
    }

    fn collect_structs(&mut self) -> Result<(), SemanticError> {
        for struct_decl in &self.program.structs {
            if is_builtin_type_name(&struct_decl.name) {
                return Err(SemanticError::new(
                    format!("`{}` is a built-in type name", struct_decl.name),
                    struct_decl.span,
                ));
            }
            reject_builtin_value_name(&struct_decl.name, struct_decl.span)?;
            if self.structs.contains_key(struct_decl.name.as_str()) {
                return Err(SemanticError::new(
                    format!("duplicate struct `{}`", struct_decl.name),
                    struct_decl.span,
                ));
            }
            self.structs
                .insert(struct_decl.name.clone(), StructSig { fields: Vec::new() });
        }

        for struct_decl in &self.program.structs {
            let mut seen_fields = HashMap::new();
            let mut fields = Vec::new();
            for field in &struct_decl.fields {
                if seen_fields
                    .insert(field.name.as_str(), field.span)
                    .is_some()
                {
                    return Err(SemanticError::new(
                        format!("duplicate field `{}` in `{}`", field.name, struct_decl.name),
                        field.span,
                    ));
                }
                fields.push(FieldSig {
                    name: field.name.clone(),
                    ty: self.type_from_ref(&field.ty)?,
                    ty_span: field.ty.span,
                });
            }

            self.structs
                .insert(struct_decl.name.clone(), StructSig { fields });
        }

        self.reject_recursive_structs()?;

        Ok(())
    }

    fn reject_recursive_structs(&self) -> Result<(), SemanticError> {
        for struct_decl in &self.program.structs {
            let mut visiting = vec![struct_decl.name.clone()];
            let struct_sig = self.struct_sig(&struct_decl.name, struct_decl.span)?;
            for field_sig in &struct_sig.fields {
                self.reject_recursive_type(&field_sig.ty, field_sig.ty_span, &mut visiting)?;
            }
        }

        Ok(())
    }

    fn reject_recursive_type(
        &self,
        ty: &Type,
        span: Span,
        visiting: &mut Vec<String>,
    ) -> Result<(), SemanticError> {
        match ty {
            Type::Struct(name) => {
                if visiting.contains(name) {
                    return Err(SemanticError::new(
                        format!(
                            "recursive type definition involving `{name}` is not supported in v0"
                        ),
                        span,
                    ));
                }

                visiting.push(name.clone());
                let struct_sig = self.struct_sig(name, span)?;
                for field_sig in &struct_sig.fields {
                    self.reject_recursive_type(&field_sig.ty, field_sig.ty_span, visiting)?;
                }
                visiting.pop();
                Ok(())
            }
            Type::Option(inner) | Type::Array { element: inner, .. } | Type::Slice(inner) => {
                self.reject_recursive_type(inner, span, visiting)
            }
            Type::Result(ok, err) => {
                self.reject_recursive_type(ok, span, visiting)?;
                self.reject_recursive_type(err, span, visiting)
            }
            Type::Int | Type::Bool | Type::String | Type::Unit | Type::Function(_) => Ok(()),
        }
    }

    fn collect_signatures(&mut self) -> Result<(), SemanticError> {
        for function in &self.program.functions {
            if function.receiver.is_none() {
                if is_builtin_type_name(&function.name) {
                    return Err(SemanticError::new(
                        format!("`{}` is a built-in type name", function.name),
                        function.span,
                    ));
                }
                reject_builtin_value_name(&function.name, function.span)?;
            }

            if function.name == "main" {
                if function.receiver.is_some() {
                    return Err(SemanticError::new(
                        "`main` must not declare a method receiver",
                        function.span,
                    ));
                }
                if !function.params.is_empty() {
                    return Err(SemanticError::new(
                        "`main` must not take parameters",
                        function.params[0].span,
                    ));
                }
                if let Some(return_type) = &function.return_type {
                    return Err(SemanticError::new(
                        "`main` must not declare a return type in v0",
                        return_type.span,
                    ));
                }
            }

            let return_type = self.type_from_optional_ref(function.return_type.as_ref())?;
            let mut params = Vec::new();
            for param in &function.params {
                params.push(self.param_sig(param)?);
            }

            let function_sig = FunctionSig {
                return_type,
                params,
            };
            if let Some(receiver) = &function.receiver {
                let receiver = self.param_sig(receiver)?;
                if !matches!(receiver.ty, Type::Struct(_)) {
                    return Err(SemanticError::new(
                        "method receiver must be a struct type in v0",
                        function.receiver.as_ref().unwrap().span,
                    ));
                }
                let key = MethodKey {
                    receiver: receiver.ty.clone(),
                    name: function.name.clone(),
                };
                if let Some(package_graph) = self.package_graph {
                    let package = package_graph
                        .package_for_source(function.span.source)
                        .ok_or_else(|| {
                            SemanticError::new(
                                "method source is not part of the package graph",
                                function.span,
                            )
                        })?;
                    self.method_access.insert(
                        key.clone(),
                        MethodAccess {
                            package_path: package.path.clone(),
                            visibility: function.visibility,
                        },
                    );
                }
                if self
                    .methods
                    .insert(
                        key,
                        MethodSig {
                            receiver,
                            function: function_sig,
                        },
                    )
                    .is_some()
                {
                    return Err(SemanticError::new(
                        format!("duplicate method `{}`", function.name),
                        function.span,
                    ));
                }
            } else {
                if self.structs.contains_key(function.name.as_str()) {
                    return Err(SemanticError::new(
                        format!(
                            "top-level function `{}` conflicts with struct `{}`",
                            function.name, function.name
                        ),
                        function.span,
                    ));
                }
                if self.signatures.contains_key(function.name.as_str()) {
                    return Err(SemanticError::new(
                        format!("duplicate function `{}`", function.name),
                        function.span,
                    ));
                }
                self.signatures.insert(function.name.clone(), function_sig);
            }
        }

        if !self.signatures.contains_key("main") {
            return Err(SemanticError::new(
                "program must declare `func main()`",
                self.program.span,
            ));
        }

        Ok(())
    }

    fn check_function(&self, function: &Function) -> Result<(), SemanticError> {
        let (receiver, sig) = self.callable_sig(function)?;
        let mut locals = HashMap::new();
        if let Some(receiver) = receiver {
            locals.insert(
                receiver.name.clone(),
                Local {
                    ty: receiver.ty.clone(),
                    mutable: matches!(receiver.mode, ParamMode::Mut),
                    borrowed: !matches!(receiver.mode, ParamMode::Owned),
                    moved: false,
                    range_source: false,
                    scope_depth: 0,
                },
            );
        }
        for param in &sig.params {
            if locals
                .insert(
                    param.name.clone(),
                    Local {
                        ty: param.ty.clone(),
                        mutable: matches!(param.mode, ParamMode::Mut),
                        borrowed: !matches!(param.mode, ParamMode::Owned),
                        moved: false,
                        range_source: false,
                        scope_depth: 0,
                    },
                )
                .is_some()
            {
                return Err(SemanticError::new(
                    format!("duplicate parameter `{}`", param.name),
                    function.span,
                ));
            }
        }

        let returned =
            self.check_block_statements(&function.body, &mut locals, &sig.return_type, 0, 0)?;

        if sig.return_type != Type::Unit && !returned {
            return Err(SemanticError::new(
                format!(
                    "function `{}` must return `{}`",
                    function.name,
                    sig.return_type.source_name()
                ),
                function.span,
            ));
        }

        Ok(())
    }

    fn param_sig(&self, param: &crate::ast::Param) -> Result<ParamSig, SemanticError> {
        reject_builtin_value_name(&param.name, param.span)?;

        Ok(ParamSig {
            name: param.name.clone(),
            mode: param.mode,
            ty: self.type_from_ref(&param.ty)?,
        })
    }

    fn callable_sig(
        &self,
        function: &Function,
    ) -> Result<(Option<&ParamSig>, &FunctionSig), SemanticError> {
        if let Some(receiver) = &function.receiver {
            let receiver_ty = self.type_from_ref(&receiver.ty)?;
            let key = MethodKey {
                receiver: receiver_ty,
                name: function.name.clone(),
            };
            let method = self.method_sig(&key, function.span)?;
            Ok((Some(&method.receiver), &method.function))
        } else {
            Ok((None, self.function_sig(&function.name, function.span)?))
        }
    }

    fn check_stmt(
        &self,
        stmt: &Stmt,
        locals: &mut HashMap<String, Local>,
        return_type: &Type,
        loop_depth: usize,
        scope_depth: usize,
    ) -> Result<bool, SemanticError> {
        match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                expr,
            } => {
                self.check_let_binding(*mutable, name, expr, locals, stmt.span, scope_depth)?;
                Ok(false)
            }
            StmtKind::Assign { name, expr } => {
                self.check_assign_binding(name, expr, locals, stmt.span)?;
                Ok(false)
            }
            StmtKind::FieldAssign { base, field, expr } => {
                self.check_field_assign(base, field, expr, locals)?;
                Ok(false)
            }
            StmtKind::IndexAssign { base, index, expr } => {
                self.check_index_assign(base, index, expr, locals, stmt.span)?;
                Ok(false)
            }
            StmtKind::Return { expr } => {
                let value_ty = self.check_expr_with_expected(
                    expr,
                    locals,
                    ValueUse::Owned,
                    Some(return_type),
                )?;
                if &value_ty != return_type {
                    return Err(SemanticError::new(
                        format!(
                            "return type mismatch: expected `{}`, got `{}`",
                            return_type.source_name(),
                            value_ty.source_name()
                        ),
                        stmt.span,
                    ));
                }
                Ok(true)
            }
            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition_ty = self.check_expr(condition, locals, ValueUse::Owned)?;
                if condition_ty != Type::Bool {
                    return Err(SemanticError::new(
                        "if condition must have type `bool`",
                        condition.span,
                    ));
                }

                let mut then_locals = locals.clone();
                let then_returns = self.check_block_statements(
                    then_block,
                    &mut then_locals,
                    return_type,
                    loop_depth,
                    scope_depth + 1,
                )?;
                let mut else_locals = locals.clone();
                let else_returns = if let Some(else_block) = else_block {
                    self.check_block_statements(
                        else_block,
                        &mut else_locals,
                        return_type,
                        loop_depth,
                        scope_depth + 1,
                    )?
                } else {
                    false
                };

                merge_branch_moves(locals, &then_locals, &else_locals);
                Ok(then_returns && else_returns)
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
            } => {
                let mut loop_locals = locals.clone();
                let loop_scope_depth = scope_depth + 1;
                if let Some(init) = init {
                    self.check_for_init(init, &mut loop_locals, stmt.span, loop_scope_depth)?;
                }

                if let Some(condition) = condition {
                    let condition_start_locals = loop_locals.clone();
                    let condition_ty =
                        self.check_expr(condition, &mut loop_locals, ValueUse::Owned)?;
                    if condition_ty != Type::Bool {
                        return Err(SemanticError::new(
                            "for condition must have type `bool`",
                            condition.span,
                        ));
                    }
                    reject_loop_persistent_moves(
                        &condition_start_locals,
                        &loop_locals,
                        loop_scope_depth,
                        condition.span,
                    )?;
                }

                let mut body_locals = loop_locals.clone();
                self.check_block_statements(
                    body,
                    &mut body_locals,
                    return_type,
                    loop_depth + 1,
                    loop_scope_depth + 1,
                )?;
                reject_loop_persistent_moves(
                    &loop_locals,
                    &body_locals,
                    loop_scope_depth,
                    body.span,
                )?;
                let mut post_locals = loop_locals.clone();
                merge_loop_body_moves(&mut post_locals, &body_locals);
                if let Some(post) = post {
                    let post_start_locals = post_locals.clone();
                    self.check_for_post(post, &mut post_locals, stmt.span)?;
                    reject_loop_persistent_moves(
                        &post_start_locals,
                        &post_locals,
                        loop_scope_depth,
                        stmt.span,
                    )?;
                }
                merge_loop_body_moves(locals, &loop_locals);
                merge_loop_body_moves(locals, &body_locals);
                merge_loop_body_moves(locals, &post_locals);
                Ok(false)
            }
            StmtKind::RangeFor {
                index_name,
                value_name,
                source,
                body,
            } => {
                self.check_range_for(
                    RangeForParts {
                        index_name,
                        value_name,
                        source,
                        body,
                        span: stmt.span,
                    },
                    locals,
                    return_type,
                    loop_depth,
                    scope_depth,
                )?;
                Ok(false)
            }
            StmtKind::Break => {
                if loop_depth == 0 {
                    return Err(SemanticError::new(
                        "`break` can only be used inside a loop",
                        stmt.span,
                    ));
                }
                Ok(false)
            }
            StmtKind::Continue => {
                if loop_depth == 0 {
                    return Err(SemanticError::new(
                        "`continue` can only be used inside a loop",
                        stmt.span,
                    ));
                }
                Ok(false)
            }
            StmtKind::Match { scrutinee, arms } => self.check_match_stmt(
                MatchStmtParts {
                    scrutinee,
                    arms,
                    span: stmt.span,
                },
                locals,
                return_type,
                loop_depth,
                scope_depth,
            ),
            StmtKind::Expr { expr } => {
                self.check_stmt_expr(expr, locals)?;
                Ok(false)
            }
        }
    }

    fn check_stmt_expr(
        &self,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        if let ExprKind::Call { callee, args } = &expr.kind {
            if matches!(&callee.kind, ExprKind::Var(name) if name == "print") {
                return self.check_print_builtin(args, locals, expr.span);
            }
        }

        let ty = self.check_expr(expr, locals, ValueUse::Owned)?;
        if ty.needs_cleanup() {
            return Err(SemanticError::new(
                format!(
                    "expression statements cannot discard cleanup value of type `{}` in v0",
                    ty.source_name()
                ),
                expr.span,
            ));
        }
        Ok(ty)
    }

    fn check_let_binding(
        &self,
        mutable: bool,
        name: &str,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        span: Span,
        scope_depth: usize,
    ) -> Result<(), SemanticError> {
        reject_builtin_value_name(name, span)?;

        let recursive_binding = (!locals.contains_key(name)).then_some(name);
        let ty = if let ExprKind::FunctionLiteral(function) = &expr.kind {
            self.check_function_literal(function, locals, expr.span, recursive_binding)?
        } else {
            self.check_expr(expr, locals, ValueUse::Owned)?
        };
        if locals
            .get(name)
            .is_some_and(|local| local.scope_depth == scope_depth)
        {
            return Err(SemanticError::new(
                format!("binding `{name}` already exists in this block"),
                span,
            ));
        }

        locals.insert(
            name.to_string(),
            Local {
                ty,
                mutable,
                borrowed: false,
                moved: false,
                range_source: false,
                scope_depth,
            },
        );
        Ok(())
    }

    fn check_assign_binding(
        &self,
        name: &str,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<(), SemanticError> {
        let (local_ty, local_mutable) = {
            let Some(local) = locals.get(name) else {
                return Err(SemanticError::new(
                    format!("unknown variable `{name}`"),
                    span,
                ));
            };
            (local.ty.clone(), local.mutable)
        };
        if !local_mutable {
            return Err(SemanticError::new(
                format!("cannot assign to immutable binding `{name}`"),
                span,
            ));
        }
        if locals.get(name).is_some_and(|local| local.range_source) {
            return Err(SemanticError::new(
                format!("cannot assign to active range source `{name}` in v0"),
                span,
            ));
        }
        let value_ty =
            self.check_expr_with_expected(expr, locals, ValueUse::Owned, Some(&local_ty))?;
        if value_ty != local_ty {
            return Err(SemanticError::new(
                format!(
                    "assignment type mismatch for `{name}`: expected `{}`, got `{}`",
                    local_ty.source_name(),
                    value_ty.source_name()
                ),
                span,
            ));
        }
        if let Some(local) = locals.get_mut(name) {
            local.moved = false;
        }
        Ok(())
    }

    fn check_for_init(
        &self,
        init: &ForInit,
        locals: &mut HashMap<String, Local>,
        span: Span,
        scope_depth: usize,
    ) -> Result<(), SemanticError> {
        match init {
            ForInit::Let {
                mutable,
                name,
                expr,
            } => self.check_let_binding(*mutable, name, expr, locals, span, scope_depth),
        }
    }

    fn check_for_post(
        &self,
        post: &ForPost,
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<(), SemanticError> {
        match post {
            ForPost::Assign { target, expr } => match &target.kind {
                ExprKind::Var(name) => self.check_assign_binding(name, expr, locals, span),
                ExprKind::FieldAccess { base, field } => {
                    self.check_field_assign(base, field, expr, locals)
                }
                ExprKind::Index { base, index } => {
                    self.check_index_assign(base, index, expr, locals, span)
                }
                _ => Err(SemanticError::new(
                    "for post target must be a variable, field access, or index expression",
                    target.span,
                )),
            },
        }
    }

    fn check_block_statements(
        &self,
        block: &Block,
        locals: &mut HashMap<String, Local>,
        return_type: &Type,
        loop_depth: usize,
        scope_depth: usize,
    ) -> Result<bool, SemanticError> {
        let mut returns = false;
        for stmt in &block.statements {
            returns |= self.check_stmt(stmt, locals, return_type, loop_depth, scope_depth)?;
        }
        Ok(returns)
    }

    fn check_expr(
        &self,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
    ) -> Result<Type, SemanticError> {
        self.check_expr_with_expected(expr, locals, value_use, None)
    }

    fn check_expr_with_expected(
        &self,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        expected: Option<&Type>,
    ) -> Result<Type, SemanticError> {
        match &expr.kind {
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::String(_) => Ok(Type::String),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::Nil => Err(SemanticError::new(
                "`nil` is reserved; use Option[T] when optional values are implemented",
                expr.span,
            )),
            ExprKind::Var(name) if name == "None" => {
                self.check_none_constructor(expected, expr.span)
            }
            ExprKind::Var(name) => self.check_var(name, locals, value_use, expr.span),
            ExprKind::FunctionLiteral(function) => {
                self.check_function_literal(function, locals, expr.span, None)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.check_if_expr(
                IfExprParts {
                    condition,
                    then_branch,
                    else_branch,
                    span: expr.span,
                },
                locals,
                value_use,
                expected,
            ),
            ExprKind::Match { scrutinee, arms } => {
                self.check_match_expr(scrutinee, arms, locals, value_use, expected, expr.span)
            }
            ExprKind::StructLiteral {
                type_name,
                type_args,
                fields,
            } => {
                if !type_args.is_empty() {
                    return Err(SemanticError::new(
                        "generic struct literals require v0.4 specialization",
                        expr.span,
                    ));
                }
                self.check_struct_literal(type_name, fields, locals, expected, expr.span)
            }
            ExprKind::ArrayLiteral { ty, elements } => {
                self.check_array_literal(ty, elements, locals, expected, expr.span)
            }
            ExprKind::FieldAccess { base, field } => {
                self.check_field_access(base, field, locals, value_use, expr.span)
            }
            ExprKind::Index { base, index } => {
                self.check_index_access(base, index, locals, value_use, expr.span)
            }
            ExprKind::TypeApply { .. } => Err(SemanticError::new(
                "generic value application requires v0.4 specialization",
                expr.span,
            )),
            ExprKind::Call { callee, args } => {
                self.check_call(callee, args, locals, expected, expr.span)
            }
            ExprKind::Unary { op, expr } => {
                let ty = self.check_expr(expr, locals, ValueUse::Owned)?;
                match (*op, &ty) {
                    (UnaryOp::Negate, Type::Int) => {
                        if let Some(value) = const_int_expr(expr) {
                            value
                                .checked_neg()
                                .ok_or_else(|| SemanticError::new("integer overflow", expr.span))?;
                        }
                        Ok(Type::Int)
                    }
                    (UnaryOp::Not, Type::Bool) => Ok(Type::Bool),
                    (UnaryOp::Negate, _) => Err(SemanticError::new(
                        "`-` expects an `int` operand",
                        expr.span,
                    )),
                    (UnaryOp::Not, _) => Err(SemanticError::new(
                        "`!` expects a `bool` operand",
                        expr.span,
                    )),
                }
            }
            ExprKind::Binary { op, left, right } => self.check_binary(*op, left, right, locals),
        }
    }

    fn check_function_literal(
        &self,
        function: &FunctionLiteral,
        outer_locals: &mut HashMap<String, Local>,
        span: Span,
        recursive_binding: Option<&str>,
    ) -> Result<Type, SemanticError> {
        let capture_uses = collect_closure_captures(
            function,
            outer_locals,
            &self.methods,
            &self.structs,
            recursive_binding,
        )?;
        let mut captures = Vec::new();
        let mut closure_locals = HashMap::new();
        for capture_use in capture_uses {
            let name = capture_use.name;
            let local = outer_locals
                .get(&name)
                .expect("capture collection only records existing outer locals");
            if local.moved {
                return Err(SemanticError::new(
                    format!("cannot capture moved value `{name}`"),
                    span,
                ));
            }
            if local.range_source {
                return Err(SemanticError::new(
                    format!("cannot capture active range source `{name}`"),
                    span,
                ));
            }
            if local.borrowed && !local.ty.is_copy() {
                return Err(SemanticError::new(
                    format!("cannot capture borrowed non-Copy value `{name}`"),
                    span,
                ));
            }
            if function.mutable && capture_use.mutable && !local.mutable {
                return Err(SemanticError::new(
                    format!("mutable closure capture `{name}` requires a mutable source binding"),
                    span,
                ));
            }

            captures.push(ClosureCapture {
                name: name.clone(),
                ty: local.ty.clone(),
                mutable: capture_use.mutable,
            });
            closure_locals.insert(
                name,
                Local {
                    ty: local.ty.clone(),
                    mutable: function.mutable && capture_use.mutable,
                    borrowed: true,
                    moved: false,
                    range_source: false,
                    scope_depth: 0,
                },
            );
        }

        let mut params = Vec::new();
        for param in &function.params {
            let param = self.param_sig(param)?;
            if closure_locals
                .insert(
                    param.name.clone(),
                    Local {
                        ty: param.ty.clone(),
                        mutable: matches!(param.mode, ParamMode::Mut),
                        borrowed: !matches!(param.mode, ParamMode::Owned),
                        moved: false,
                        range_source: false,
                        scope_depth: 0,
                    },
                )
                .is_some()
            {
                return Err(SemanticError::new(
                    format!("duplicate closure parameter `{}`", param.name),
                    span,
                ));
            }
            params.push(param);
        }
        let return_type = self.type_from_optional_ref(function.return_type.as_ref())?;
        let returned =
            self.check_block_statements(&function.body, &mut closure_locals, &return_type, 0, 0)?;
        if return_type != Type::Unit && !returned {
            return Err(SemanticError::new(
                format!(
                    "function literal must return `{}`",
                    return_type.source_name()
                ),
                span,
            ));
        }

        for capture in &captures {
            if !capture.ty.is_copy() {
                outer_locals
                    .get_mut(&capture.name)
                    .expect("validated capture must remain in outer locals")
                    .moved = true;
            }
        }

        let function_type = FunctionType {
            mutable: function.mutable,
            params: params
                .iter()
                .map(|param| FunctionParamType {
                    mode: param.mode,
                    ty: param.ty.clone(),
                })
                .collect(),
            return_type: Box::new(return_type),
        };
        self.closures.borrow_mut().push(CheckedClosure {
            span,
            literal: function.clone(),
            function_type: function_type.clone(),
            captures,
        });

        Ok(Type::Function(function_type))
    }

    fn check_if_expr(
        &self,
        parts: IfExprParts<'_>,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        expected: Option<&Type>,
    ) -> Result<Type, SemanticError> {
        let condition_ty = self.check_expr(parts.condition, locals, ValueUse::Owned)?;
        if condition_ty != Type::Bool {
            return Err(SemanticError::new(
                "if condition must have type `bool`",
                parts.condition.span,
            ));
        }

        let branch_check = self.check_if_branches(
            parts.then_branch,
            parts.else_branch,
            locals,
            value_use,
            expected,
        )?;

        if branch_check.then_ty != branch_check.else_ty {
            return Err(SemanticError::new(
                format!(
                    "if branches must have the same type: got `{}` and `{}`",
                    branch_check.then_ty.source_name(),
                    branch_check.else_ty.source_name()
                ),
                parts.span,
            ));
        }
        if branch_check.then_ty == Type::Unit {
            return Err(SemanticError::new(
                "if expression branches must produce a value in v0",
                parts.span,
            ));
        }

        merge_branch_moves(locals, &branch_check.then_locals, &branch_check.else_locals);
        Ok(branch_check.then_ty)
    }

    fn check_match_expr(
        &self,
        scrutinee: &Expr,
        arms: &[MatchArm],
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        if arms.is_empty() {
            return Err(SemanticError::new("match requires at least one arm", span));
        }

        let scrutinee_ty = self.check_expr(scrutinee, locals, ValueUse::Owned)?;
        let prepared_arms = self.prepare_match_arms(&scrutinee_ty, arms, span)?;
        let arm_checks = self.check_match_arms(&prepared_arms, locals, value_use, expected)?;
        let first_ty = arm_checks[0].ty.clone();

        for arm_check in &arm_checks[1..] {
            if arm_check.ty != first_ty {
                return Err(SemanticError::new(
                    format!(
                        "match arms must have the same type: got `{}` and `{}`",
                        first_ty.source_name(),
                        arm_check.ty.source_name()
                    ),
                    span,
                ));
            }
        }
        if first_ty == Type::Unit {
            return Err(SemanticError::new(
                "match expression arms must produce a value in v0",
                span,
            ));
        }

        merge_many_branch_moves(locals, arm_checks.iter().map(|check| &check.locals));
        Ok(first_ty)
    }

    fn check_match_stmt(
        &self,
        parts: MatchStmtParts<'_>,
        locals: &mut HashMap<String, Local>,
        return_type: &Type,
        loop_depth: usize,
        scope_depth: usize,
    ) -> Result<bool, SemanticError> {
        if parts.arms.is_empty() {
            return Err(SemanticError::new(
                "match requires at least one arm",
                parts.span,
            ));
        }

        let scrutinee_ty = self.check_expr(parts.scrutinee, locals, ValueUse::Owned)?;
        let prepared_arms = self.prepare_match_block_arms(&scrutinee_ty, parts.arms, parts.span)?;
        let mut checks = Vec::new();
        for arm in &prepared_arms {
            checks.push(self.check_prepared_match_block_arm(
                arm,
                locals,
                return_type,
                loop_depth,
                scope_depth + 1,
            )?);
        }

        let all_return = checks.iter().all(|check| check.returns);
        merge_many_branch_moves(locals, checks.iter().map(|check| &check.locals));
        Ok(all_return)
    }

    fn check_struct_literal(
        &self,
        type_name: &str,
        fields: &[crate::ast::FieldInit],
        locals: &mut HashMap<String, Local>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        if let Some(expected) = expected {
            let expected_ty = Type::Struct(type_name.to_string());
            if expected != &expected_ty {
                return Err(SemanticError::new(
                    format!(
                        "struct literal type mismatch: expected `{}`, got `{}`",
                        expected.source_name(),
                        expected_ty.source_name()
                    ),
                    span,
                ));
            }
        }

        let struct_sig = self.struct_sig(type_name, span)?;
        let mut seen = HashMap::new();
        for field in fields {
            if seen.insert(field.name.as_str(), field.span).is_some() {
                return Err(SemanticError::new(
                    format!("duplicate field `{}` in `{type_name}` literal", field.name),
                    field.span,
                ));
            }

            let Some(field_sig) = struct_sig
                .fields
                .iter()
                .find(|candidate| candidate.name == field.name)
            else {
                return Err(SemanticError::new(
                    format!("unknown field `{}` in `{type_name}` literal", field.name),
                    field.span,
                ));
            };

            let value_ty = self.check_expr_with_expected(
                &field.expr,
                locals,
                ValueUse::Owned,
                Some(&field_sig.ty),
            )?;
            if value_ty != field_sig.ty {
                return Err(SemanticError::new(
                    format!(
                        "field `{}` type mismatch: expected `{}`, got `{}`",
                        field.name,
                        field_sig.ty.source_name(),
                        value_ty.source_name()
                    ),
                    field.span,
                ));
            }
        }

        for field_sig in &struct_sig.fields {
            if !seen.contains_key(field_sig.name.as_str()) {
                return Err(SemanticError::new(
                    format!(
                        "missing field `{}` in `{type_name}` literal",
                        field_sig.name
                    ),
                    span,
                ));
            }
        }

        Ok(Type::Struct(type_name.to_string()))
    }

    fn check_array_literal(
        &self,
        ty_ref: &TypeRef,
        elements: &[Expr],
        locals: &mut HashMap<String, Local>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let array_ty = self.type_from_ref(ty_ref)?;
        if let Some(expected) = expected {
            if expected != &array_ty {
                return Err(SemanticError::new(
                    format!(
                        "array literal type mismatch: expected `{}`, got `{}`",
                        expected.source_name(),
                        array_ty.source_name()
                    ),
                    span,
                ));
            }
        }

        let element = match &array_ty {
            Type::Array { len, element } => {
                if elements.len() != *len {
                    return Err(SemanticError::new(
                        format!(
                            "array literal length mismatch: expected {len} elements, got {}",
                            elements.len()
                        ),
                        span,
                    ));
                }
                element.as_ref()
            }
            Type::Slice(element) => element.as_ref(),
            _ => {
                return Err(SemanticError::new(
                    "array literal requires a fixed-size array or slice type",
                    ty_ref.span,
                ));
            }
        };

        for (index, element_expr) in elements.iter().enumerate() {
            let value_ty = self.check_expr_with_expected(
                element_expr,
                locals,
                ValueUse::Owned,
                Some(element),
            )?;
            if value_ty != *element {
                return Err(SemanticError::new(
                    format!(
                        "array literal element {index} type mismatch: expected `{}`, got `{}`",
                        element.source_name(),
                        value_ty.source_name()
                    ),
                    element_expr.span,
                ));
            }
        }

        Ok(array_ty)
    }

    fn check_range_for(
        &self,
        parts: RangeForParts<'_>,
        locals: &mut HashMap<String, Local>,
        return_type: &Type,
        loop_depth: usize,
        scope_depth: usize,
    ) -> Result<(), SemanticError> {
        if !is_blank_identifier(parts.index_name) {
            reject_builtin_value_name(parts.index_name, parts.span)?;
        }
        if !is_blank_identifier(parts.value_name) {
            reject_builtin_value_name(parts.value_name, parts.span)?;
        }

        if parts.index_name == parts.value_name && !is_blank_identifier(parts.index_name) {
            return Err(SemanticError::new(
                "range index and value bindings must use different names",
                parts.span,
            ));
        }

        let source_ty = self.check_expr(parts.source, locals, ValueUse::Borrow)?;
        let element = match &source_ty {
            Type::Array { element, .. } => element,
            Type::Slice(element) => {
                if !is_direct_borrow_expr(parts.source) {
                    return Err(SemanticError::new(
                        "range over slices requires a local-rooted slice source in v0",
                        parts.source.span,
                    ));
                }
                element
            }
            _ => {
                return Err(SemanticError::new(
                    format!(
                        "range source must be a fixed-size array or slice, got `{}`",
                        source_ty.source_name()
                    ),
                    parts.source.span,
                ));
            }
        };
        if !is_blank_identifier(parts.value_name) && !element.is_copy() {
            return Err(SemanticError::new(
                format!(
                    "range value binding requires a Copy element type in v0, got `{}`",
                    element.source_name()
                ),
                parts.source.span,
            ));
        }

        let mut body_locals = locals.clone();
        let range_scope_depth = scope_depth + 1;
        let body_scope_depth = range_scope_depth + 1;
        if !is_blank_identifier(parts.index_name) {
            body_locals.insert(
                parts.index_name.to_string(),
                Local {
                    ty: Type::Int,
                    mutable: false,
                    borrowed: false,
                    moved: false,
                    range_source: false,
                    scope_depth: range_scope_depth,
                },
            );
        }
        if !is_blank_identifier(parts.value_name) {
            body_locals.insert(
                parts.value_name.to_string(),
                Local {
                    ty: element.as_ref().clone(),
                    mutable: false,
                    borrowed: false,
                    moved: false,
                    range_source: false,
                    scope_depth: range_scope_depth,
                },
            );
        }
        if let ExprKind::Var(name) = &parts.source.kind {
            if let Some(local) = body_locals.get_mut(name) {
                local.range_source = true;
            }
        }

        self.check_block_statements(
            parts.body,
            &mut body_locals,
            return_type,
            loop_depth + 1,
            body_scope_depth,
        )?;
        reject_loop_persistent_moves(locals, &body_locals, range_scope_depth, parts.body.span)?;
        merge_loop_body_moves(locals, &body_locals);
        Ok(())
    }

    fn check_field_access(
        &self,
        base: &Expr,
        field: &str,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let base_ty = self.check_expr(base, locals, ValueUse::Borrow)?;
        let Type::Struct(type_name) = base_ty else {
            return Err(SemanticError::new(
                format!(
                    "field access requires a struct value, got `{}`",
                    base_ty.source_name()
                ),
                base.span,
            ));
        };
        let struct_sig = self.struct_sig(&type_name, span)?;
        let Some(field_sig) = struct_sig
            .fields
            .iter()
            .find(|candidate| candidate.name == field)
        else {
            return Err(SemanticError::new(
                format!("unknown field `{field}` on `{type_name}`"),
                span,
            ));
        };

        if matches!(value_use, ValueUse::Owned)
            && !field_sig.ty.is_copy()
            && !(matches!(field_sig.ty, Type::Slice(_)) && is_direct_borrow_expr(base))
        {
            self.mark_field_base_moved(base)?;
        }

        Ok(field_sig.ty.clone())
    }

    fn check_index_access(
        &self,
        base: &Expr,
        index: &Expr,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let base_ty = self.check_expr(base, locals, ValueUse::Borrow)?;
        let index_ty = self.check_expr(index, locals, ValueUse::Owned)?;
        match base_ty {
            Type::Array { len, element } => {
                self.validate_index_type_and_bounds(index, &index_ty, len)?;

                if matches!(value_use, ValueUse::Owned) && !element.is_copy() {
                    return Err(SemanticError::new(
                        format!(
                            "array indexing requires a Copy element type in v0, got `{}`",
                            element.source_name()
                        ),
                        span,
                    ));
                }

                Ok(*element)
            }
            Type::Slice(element) => {
                if !is_direct_borrow_expr(base) {
                    return Err(SemanticError::new(
                        "slice indexing requires a local-rooted slice source in v0",
                        base.span,
                    ));
                }
                self.validate_index_type_and_non_negative_literal(index, &index_ty, "slice")?;

                if matches!(value_use, ValueUse::Owned) && !element.is_copy() {
                    return Err(SemanticError::new(
                        format!(
                            "slice indexing requires a Copy element type in v0, got `{}`",
                            element.source_name()
                        ),
                        span,
                    ));
                }

                Ok(*element)
            }
            _ => Err(SemanticError::new(
                format!(
                    "indexing requires a fixed-size array or slice, got `{}`",
                    base_ty.source_name()
                ),
                base.span,
            )),
        }
    }

    fn check_index_expr(
        &self,
        index: &Expr,
        locals: &mut HashMap<String, Local>,
        len: usize,
    ) -> Result<(), SemanticError> {
        let index_ty = self.check_expr(index, locals, ValueUse::Owned)?;
        self.validate_index_type_and_bounds(index, &index_ty, len)
    }

    fn validate_index_type_and_bounds(
        &self,
        index: &Expr,
        index_ty: &Type,
        len: usize,
    ) -> Result<(), SemanticError> {
        self.validate_index_type_and_non_negative_literal(index, index_ty, "array")?;

        if let Some(index_value) = const_int_expr(index) {
            let out_of_bounds = match usize::try_from(index_value) {
                Ok(index_value) => index_value >= len,
                Err(_) => true,
            };
            if out_of_bounds {
                return Err(SemanticError::new(
                    format!("array index {index_value} is out of bounds for length {len}"),
                    index.span,
                ));
            }
        }

        Ok(())
    }

    fn validate_index_type_and_non_negative_literal(
        &self,
        index: &Expr,
        index_ty: &Type,
        kind: &str,
    ) -> Result<(), SemanticError> {
        if index_ty != &Type::Int {
            return Err(SemanticError::new(
                format!(
                    "{kind} index must have type `int`, got `{}`",
                    index_ty.source_name()
                ),
                index.span,
            ));
        }

        if let Some(index_value) = const_int_expr(index) {
            if index_value < 0 {
                return Err(SemanticError::new(
                    format!("{kind} index {index_value} must be non-negative"),
                    index.span,
                ));
            }
        }

        Ok(())
    }

    fn check_field_assign(
        &self,
        base: &Expr,
        field: &str,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<(), SemanticError> {
        let mut place = direct_borrow_place(
            base,
            "field assignment target must be a direct local variable, field, or indexed element in v0",
        )?;
        place.fields.push(field.to_string());
        let base_mutable = {
            let Some(local) = locals.get(&place.root) else {
                return Err(SemanticError::new(
                    format!("unknown variable `{}`", place.root),
                    base.span,
                ));
            };
            if local.moved {
                return Err(SemanticError::new(
                    format!("assignment to field of moved value `{}`", place.root),
                    base.span,
                ));
            }
            local.mutable
        };
        if !base_mutable {
            return Err(SemanticError::new(
                format!("cannot assign field of immutable binding `{}`", place.root),
                base.span,
            ));
        }

        let base_ty = self.resolve_assignment_place_type(base, locals)?;
        let field_path = [field.to_string()];
        let field_ty =
            self.resolve_field_path_type(&base_ty, &field_path, base.span, "field assignment")?;
        if self.is_same_field_append_assignment(base, field, expr) {
            self.check_same_field_append_assignment(expr, &field_ty, locals)?;
            return Ok(());
        }
        let value_ty =
            self.check_expr_with_expected(expr, locals, ValueUse::Owned, Some(&field_ty))?;
        if value_ty != field_ty {
            return Err(SemanticError::new(
                format!(
                    "field `{field}` assignment type mismatch: expected `{}`, got `{}`",
                    field_ty.source_name(),
                    value_ty.source_name()
                ),
                expr.span,
            ));
        }

        Ok(())
    }

    fn is_same_field_append_assignment(&self, base: &Expr, field: &str, expr: &Expr) -> bool {
        let ExprKind::Call { callee, args } = &expr.kind else {
            return false;
        };
        if !matches!(&callee.kind, ExprKind::Var(name) if name == "append") || args.len() != 2 {
            return false;
        }
        is_same_field_target_expr(base, field, &args[0].expr)
    }

    fn check_same_field_append_assignment(
        &self,
        expr: &Expr,
        field_ty: &Type,
        locals: &mut HashMap<String, Local>,
    ) -> Result<(), SemanticError> {
        let ExprKind::Call { args, .. } = &expr.kind else {
            return Err(SemanticError::new(
                "internal semantic error: expected append call",
                expr.span,
            ));
        };
        if args.len() != 2 {
            return Err(SemanticError::new(
                "`append` expects exactly two arguments",
                expr.span,
            ));
        }
        if args[0].mode != ArgMode::Owned || args[1].mode != ArgMode::Owned {
            return Err(SemanticError::new(
                "`append` arguments do not take `con` or `mut` mode markers",
                expr.span,
            ));
        }
        let Type::Slice(element_ty) = field_ty else {
            return Err(SemanticError::new(
                format!(
                    "`append` first argument must be a slice, got `{}`",
                    field_ty.source_name()
                ),
                args[0].span,
            ));
        };
        let item_ty = self.check_expr_with_expected(
            &args[1].expr,
            locals,
            ValueUse::Owned,
            Some(element_ty),
        )?;
        if item_ty != **element_ty {
            return Err(SemanticError::new(
                format!(
                    "`append` item type mismatch: expected `{}`, got `{}`",
                    element_ty.source_name(),
                    item_ty.source_name()
                ),
                args[1].span,
            ));
        }

        Ok(())
    }

    fn resolve_assignment_place_type(
        &self,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        match &expr.kind {
            ExprKind::Var(name) => {
                let Some(local) = locals.get(name) else {
                    return Err(SemanticError::new(
                        format!("unknown variable `{name}`"),
                        expr.span,
                    ));
                };
                if local.moved {
                    return Err(SemanticError::new(
                        format!("assignment to moved value `{name}`"),
                        expr.span,
                    ));
                }
                Ok(local.ty.clone())
            }
            ExprKind::FieldAccess { base, field } => {
                let base_ty = self.resolve_assignment_place_type(base, locals)?;
                self.resolve_field_path_type(
                    &base_ty,
                    std::slice::from_ref(field),
                    expr.span,
                    "field assignment",
                )
            }
            ExprKind::Index { base, index } => {
                let base_ty = self.resolve_assignment_place_type(base, locals)?;
                match base_ty {
                    Type::Array { len, element } => {
                        self.check_index_expr(index, locals, len)?;
                        Ok(*element)
                    }
                    Type::Slice(element) => {
                        let index_ty = self.check_expr(index, locals, ValueUse::Owned)?;
                        self.validate_index_type_and_non_negative_literal(
                            index, &index_ty, "slice",
                        )?;
                        Ok(*element)
                    }
                    _ => Err(SemanticError::new(
                        format!(
                            "indexed field assignment target must be a fixed-size array or slice, got `{}`",
                            base_ty.source_name()
                        ),
                        base.span,
                    )),
                }
            }
            _ => Err(SemanticError::new(
                "field assignment target must be a direct local variable, field, or indexed element in v0",
                expr.span,
            )),
        }
    }

    fn check_index_assign(
        &self,
        base: &Expr,
        index: &Expr,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<(), SemanticError> {
        let place = direct_borrow_place(
            base,
            "indexed assignment target must be a local-rooted mutable array or slice in v0",
        )?;
        let (mutable, root_has_fields) = {
            let Some(local) = locals.get(&place.root) else {
                return Err(SemanticError::new(
                    format!("unknown variable `{}`", place.root),
                    base.span,
                ));
            };
            if local.moved {
                return Err(SemanticError::new(
                    format!("use of moved value `{}`", place.root),
                    base.span,
                ));
            }
            (local.mutable, !place.fields.is_empty())
        };
        if !mutable {
            let message = if root_has_fields {
                format!(
                    "cannot assign through immutable indexed place of binding `{}`",
                    place.root
                )
            } else {
                format!(
                    "cannot assign through immutable indexed binding `{}`",
                    place.root
                )
            };
            return Err(SemanticError::new(message, base.span));
        }

        let base_ty = self.resolve_assignment_place_type(base, locals)?;
        let element = match base_ty {
            Type::Array { len, element } => {
                self.check_index_expr(index, locals, len)?;
                element
            }
            Type::Slice(element) => {
                let index_ty = self.check_expr(index, locals, ValueUse::Owned)?;
                self.validate_index_type_and_non_negative_literal(index, &index_ty, "slice")?;
                element
            }
            _ => {
                return Err(SemanticError::new(
                    format!(
                        "indexed assignment target must be a fixed-size array or slice, got `{}`",
                        base_ty.source_name()
                    ),
                    base.span,
                ));
            }
        };

        let value_ty =
            self.check_expr_with_expected(expr, locals, ValueUse::Owned, Some(&element))?;
        if value_ty != *element {
            return Err(SemanticError::new(
                format!(
                    "indexed assignment type mismatch: expected `{}`, got `{}`",
                    element.source_name(),
                    value_ty.source_name()
                ),
                span,
            ));
        }

        Ok(())
    }

    fn prepare_match_arms<'b>(
        &self,
        scrutinee_ty: &Type,
        arms: &'b [MatchArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchArm<'b>>, SemanticError> {
        match scrutinee_ty {
            Type::Option(inner) => self.prepare_option_match_arms(inner, arms, span),
            Type::Result(ok, err) => self.prepare_result_match_arms(ok, err, arms, span),
            _ => Err(SemanticError::new(
                format!(
                    "match scrutinee must be `Option` or `Result`, got `{}`",
                    scrutinee_ty.source_name()
                ),
                span,
            )),
        }
    }

    fn prepare_match_block_arms<'b>(
        &self,
        scrutinee_ty: &Type,
        arms: &'b [MatchBlockArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchBlockArm<'b>>, SemanticError> {
        match scrutinee_ty {
            Type::Option(inner) => self.prepare_option_match_block_arms(inner, arms, span),
            Type::Result(ok, err) => self.prepare_result_match_block_arms(ok, err, arms, span),
            _ => Err(SemanticError::new(
                format!(
                    "match scrutinee must be `Option` or `Result`, got `{}`",
                    scrutinee_ty.source_name()
                ),
                span,
            )),
        }
    }

    fn prepare_option_match_arms<'b>(
        &self,
        inner: &Type,
        arms: &'b [MatchArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchArm<'b>>, SemanticError> {
        let mut prepared = Vec::new();
        let mut seen_some = false;
        let mut seen_none = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Some(binding) => {
                    reject_builtin_value_name(binding, arm.span)?;
                    if seen_some {
                        return Err(SemanticError::new(
                            "Option match must contain exactly one `Some` arm",
                            arm.span,
                        ));
                    }
                    seen_some = true;
                    prepared.push(PreparedMatchArm {
                        expr: &arm.expr,
                        binding: Some((binding.as_str(), inner.clone())),
                    });
                }
                MatchPattern::None => {
                    if seen_none {
                        return Err(SemanticError::new(
                            "Option match must contain exactly one `None` arm",
                            arm.span,
                        ));
                    }
                    seen_none = true;
                    prepared.push(PreparedMatchArm {
                        expr: &arm.expr,
                        binding: None,
                    });
                }
                MatchPattern::Ok(_) | MatchPattern::Err(_) => {
                    return Err(SemanticError::new(
                        "Option match patterns must be `Some(name)` and `None`",
                        arm.span,
                    ));
                }
                _ => {
                    return Err(SemanticError::new(
                        "user-defined and nested patterns require v0.4 semantic lowering",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_some || !seen_none {
            return Err(SemanticError::new(
                "Option match must include `Some(name)` and `None` arms",
                span,
            ));
        }
        Ok(prepared)
    }

    fn prepare_result_match_arms<'b>(
        &self,
        ok: &Type,
        err: &Type,
        arms: &'b [MatchArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchArm<'b>>, SemanticError> {
        let mut prepared = Vec::new();
        let mut seen_ok = false;
        let mut seen_err = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Ok(binding) => {
                    reject_builtin_value_name(binding, arm.span)?;
                    if seen_ok {
                        return Err(SemanticError::new(
                            "Result match must contain exactly one `Ok` arm",
                            arm.span,
                        ));
                    }
                    seen_ok = true;
                    prepared.push(PreparedMatchArm {
                        expr: &arm.expr,
                        binding: Some((binding.as_str(), ok.clone())),
                    });
                }
                MatchPattern::Err(binding) => {
                    reject_builtin_value_name(binding, arm.span)?;
                    if seen_err {
                        return Err(SemanticError::new(
                            "Result match must contain exactly one `Err` arm",
                            arm.span,
                        ));
                    }
                    seen_err = true;
                    prepared.push(PreparedMatchArm {
                        expr: &arm.expr,
                        binding: Some((binding.as_str(), err.clone())),
                    });
                }
                MatchPattern::Some(_) | MatchPattern::None => {
                    return Err(SemanticError::new(
                        "Result match patterns must be `Ok(name)` and `Err(name)`",
                        arm.span,
                    ));
                }
                _ => {
                    return Err(SemanticError::new(
                        "user-defined and nested patterns require v0.4 semantic lowering",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_ok || !seen_err {
            return Err(SemanticError::new(
                "Result match must include `Ok(name)` and `Err(name)` arms",
                span,
            ));
        }
        Ok(prepared)
    }

    fn prepare_option_match_block_arms<'b>(
        &self,
        inner: &Type,
        arms: &'b [MatchBlockArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchBlockArm<'b>>, SemanticError> {
        let mut prepared = Vec::new();
        let mut seen_some = false;
        let mut seen_none = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Some(binding) => {
                    reject_builtin_value_name(binding, arm.span)?;
                    if seen_some {
                        return Err(SemanticError::new(
                            "Option match must contain exactly one `Some` arm",
                            arm.span,
                        ));
                    }
                    seen_some = true;
                    prepared.push(PreparedMatchBlockArm {
                        block: &arm.block,
                        binding: Some((binding.as_str(), inner.clone())),
                    });
                }
                MatchPattern::None => {
                    if seen_none {
                        return Err(SemanticError::new(
                            "Option match must contain exactly one `None` arm",
                            arm.span,
                        ));
                    }
                    seen_none = true;
                    prepared.push(PreparedMatchBlockArm {
                        block: &arm.block,
                        binding: None,
                    });
                }
                MatchPattern::Ok(_) | MatchPattern::Err(_) => {
                    return Err(SemanticError::new(
                        "Option match patterns must be `Some(name)` and `None`",
                        arm.span,
                    ));
                }
                _ => {
                    return Err(SemanticError::new(
                        "user-defined and nested patterns require v0.4 semantic lowering",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_some || !seen_none {
            return Err(SemanticError::new(
                "Option match must include `Some(name)` and `None` arms",
                span,
            ));
        }
        Ok(prepared)
    }

    fn prepare_result_match_block_arms<'b>(
        &self,
        ok: &Type,
        err: &Type,
        arms: &'b [MatchBlockArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchBlockArm<'b>>, SemanticError> {
        let mut prepared = Vec::new();
        let mut seen_ok = false;
        let mut seen_err = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Ok(binding) => {
                    reject_builtin_value_name(binding, arm.span)?;
                    if seen_ok {
                        return Err(SemanticError::new(
                            "Result match must contain exactly one `Ok` arm",
                            arm.span,
                        ));
                    }
                    seen_ok = true;
                    prepared.push(PreparedMatchBlockArm {
                        block: &arm.block,
                        binding: Some((binding.as_str(), ok.clone())),
                    });
                }
                MatchPattern::Err(binding) => {
                    reject_builtin_value_name(binding, arm.span)?;
                    if seen_err {
                        return Err(SemanticError::new(
                            "Result match must contain exactly one `Err` arm",
                            arm.span,
                        ));
                    }
                    seen_err = true;
                    prepared.push(PreparedMatchBlockArm {
                        block: &arm.block,
                        binding: Some((binding.as_str(), err.clone())),
                    });
                }
                MatchPattern::Some(_) | MatchPattern::None => {
                    return Err(SemanticError::new(
                        "Result match patterns must be `Ok(name)` and `Err(name)`",
                        arm.span,
                    ));
                }
                _ => {
                    return Err(SemanticError::new(
                        "user-defined and nested patterns require v0.4 semantic lowering",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_ok || !seen_err {
            return Err(SemanticError::new(
                "Result match must include `Ok(name)` and `Err(name)` arms",
                span,
            ));
        }
        Ok(prepared)
    }

    fn check_match_arms(
        &self,
        arms: &[PreparedMatchArm<'_>],
        locals: &HashMap<String, Local>,
        value_use: ValueUse,
        expected: Option<&Type>,
    ) -> Result<Vec<MatchArmCheck>, SemanticError> {
        if let Some(expected) = expected {
            return arms
                .iter()
                .map(|arm| self.check_prepared_match_arm(arm, locals, value_use, Some(expected)))
                .collect();
        }

        let mut first_error = None;
        for arm in arms {
            match self.check_prepared_match_arm(arm, locals, value_use, None) {
                Ok(_) => {
                    let expected_ty = self
                        .check_prepared_match_arm(arm, locals, value_use, None)?
                        .ty;
                    let mut checks = Vec::new();
                    for retry_arm in arms {
                        checks.push(self.check_prepared_match_arm(
                            retry_arm,
                            locals,
                            value_use,
                            Some(&expected_ty),
                        )?);
                    }
                    return Ok(checks);
                }
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }

        Err(first_error.expect("match arms are non-empty"))
    }

    fn check_prepared_match_arm(
        &self,
        arm: &PreparedMatchArm<'_>,
        locals: &HashMap<String, Local>,
        value_use: ValueUse,
        expected: Option<&Type>,
    ) -> Result<MatchArmCheck, SemanticError> {
        let mut arm_locals = locals.clone();
        let arm_scope_depth = nested_scope_depth(locals);
        if let Some((name, ty)) = &arm.binding {
            arm_locals.insert(
                (*name).to_string(),
                Local {
                    ty: ty.clone(),
                    mutable: false,
                    borrowed: false,
                    moved: false,
                    range_source: false,
                    scope_depth: arm_scope_depth,
                },
            );
        }
        let ty = self.check_expr_with_expected(arm.expr, &mut arm_locals, value_use, expected)?;
        Ok(MatchArmCheck {
            ty,
            locals: arm_locals,
        })
    }

    fn check_prepared_match_block_arm(
        &self,
        arm: &PreparedMatchBlockArm<'_>,
        locals: &HashMap<String, Local>,
        return_type: &Type,
        loop_depth: usize,
        scope_depth: usize,
    ) -> Result<MatchBlockArmCheck, SemanticError> {
        let mut arm_locals = locals.clone();
        if let Some((name, ty)) = &arm.binding {
            arm_locals.insert(
                (*name).to_string(),
                Local {
                    ty: ty.clone(),
                    mutable: false,
                    borrowed: false,
                    moved: false,
                    range_source: false,
                    scope_depth,
                },
            );
        }
        let returns = self.check_block_statements(
            arm.block,
            &mut arm_locals,
            return_type,
            loop_depth,
            scope_depth,
        )?;
        Ok(MatchBlockArmCheck {
            returns,
            locals: arm_locals,
        })
    }

    fn check_if_branches(
        &self,
        then_branch: &Expr,
        else_branch: &Expr,
        locals: &HashMap<String, Local>,
        value_use: ValueUse,
        expected: Option<&Type>,
    ) -> Result<IfBranchCheck, SemanticError> {
        if let Some(expected) = expected {
            let mut then_locals = locals.clone();
            let then_ty = self.check_expr_with_expected(
                then_branch,
                &mut then_locals,
                value_use,
                Some(expected),
            )?;
            let mut else_locals = locals.clone();
            let else_ty = self.check_expr_with_expected(
                else_branch,
                &mut else_locals,
                value_use,
                Some(expected),
            )?;
            return Ok(IfBranchCheck {
                then_ty,
                then_locals,
                else_ty,
                else_locals,
            });
        }

        let mut then_locals = locals.clone();
        match self.check_expr(then_branch, &mut then_locals, value_use) {
            Ok(then_ty) => {
                let mut else_locals = locals.clone();
                let else_ty = self.check_expr_with_expected(
                    else_branch,
                    &mut else_locals,
                    value_use,
                    Some(&then_ty),
                )?;
                Ok(IfBranchCheck {
                    then_ty,
                    then_locals,
                    else_ty,
                    else_locals,
                })
            }
            Err(then_error) => {
                let mut else_locals = locals.clone();
                let else_ty = self.check_expr(else_branch, &mut else_locals, value_use)?;
                let mut then_locals = locals.clone();
                let then_ty = self
                    .check_expr_with_expected(
                        then_branch,
                        &mut then_locals,
                        value_use,
                        Some(&else_ty),
                    )
                    .map_err(|_| then_error)?;
                Ok(IfBranchCheck {
                    then_ty,
                    then_locals,
                    else_ty,
                    else_locals,
                })
            }
        }
    }

    fn check_var(
        &self,
        name: &str,
        locals: &mut HashMap<String, Local>,
        value_use: ValueUse,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let Some(local) = locals.get_mut(name) else {
            if name == "main" {
                return Err(SemanticError::new(
                    "`main` cannot be used as a function value",
                    span,
                ));
            }
            if let Some(signature) = self.signatures.get(name) {
                return Ok(Type::Function(signature.function_type(false)));
            }
            return Err(SemanticError::new(
                format!("unknown variable `{name}`"),
                span,
            ));
        };
        if local.moved {
            return Err(SemanticError::new(
                format!("use of moved value `{name}`"),
                span,
            ));
        }

        let ty = local.ty.clone();
        if matches!(value_use, ValueUse::Owned) && !ty.is_copy() {
            if local.borrowed {
                return Err(SemanticError::new(
                    format!("cannot move borrowed value `{name}`"),
                    span,
                ));
            }
            local.moved = true;
        }

        Ok(ty)
    }

    fn mark_field_base_moved(&self, base: &Expr) -> Result<(), SemanticError> {
        match &base.kind {
            ExprKind::Var(name) => Err(SemanticError::new(
                format!("moving non-copy field out of `{name}` is not supported without destructuring"),
                base.span,
            )),
            ExprKind::Index { .. } => Err(SemanticError::new(
                "moving non-copy field out of indexed element is not supported without destructuring",
                base.span,
            )),
            ExprKind::FieldAccess { base, .. } => self.mark_field_base_moved(base),
            _ => Ok(()),
        }
    }

    fn check_call(
        &self,
        callee: &Expr,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        if let ExprKind::FieldAccess { base, field } = &callee.kind {
            return self.check_method_call(base, field, args, locals, span);
        }

        let ExprKind::Var(name) = &callee.kind else {
            return Err(SemanticError::new(
                "only direct function and method calls are supported in v0",
                callee.span,
            ));
        };

        match name.as_str() {
            "Some" => return self.check_some_constructor(args, locals, expected, span),
            "Ok" => return self.check_ok_constructor(args, locals, expected, span),
            "Err" => return self.check_err_constructor(args, locals, expected, span),
            _ => {}
        }

        if name == "len" {
            return self.check_len_builtin(args, locals, span);
        }
        if name == "append" {
            return self.check_append_builtin(args, locals, span);
        }

        if name == "print" {
            return Err(SemanticError::new(
                "`print` is only supported as a statement",
                span,
            ));
        }

        if locals.contains_key(name) {
            return self.check_function_value_call(name, args, locals, span);
        }

        let sig = self.function_sig(name, callee.span)?;
        self.check_call_args(name, args, &sig.params, locals, Vec::new(), span)?;
        Ok(sig.return_type.clone())
    }

    fn check_function_value_call(
        &self,
        name: &str,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let (function, local_mutable, moved) = {
            let local = locals
                .get(name)
                .expect("function value call starts from an existing local");
            let Type::Function(function) = &local.ty else {
                return Err(SemanticError::new(
                    format!("`{name}` is not callable"),
                    span,
                ));
            };
            (function.clone(), local.mutable, local.moved)
        };
        if moved {
            return Err(SemanticError::new(
                format!("use of moved value `{name}`"),
                span,
            ));
        }
        if function.mutable && !local_mutable {
            return Err(SemanticError::new(
                format!("mutable function value `{name}` requires mutable access"),
                span,
            ));
        }

        let params = function
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| ParamSig {
                name: format!("argument {}", index + 1),
                mode: param.mode,
                ty: param.ty.clone(),
            })
            .collect::<Vec<_>>();
        let borrow_kind = if function.mutable {
            BorrowKind::Exclusive
        } else {
            BorrowKind::Shared
        };
        self.check_call_args(
            name,
            args,
            &params,
            locals,
            vec![(BorrowPlace::root(name.to_string()), borrow_kind)],
            span,
        )?;
        Ok(*function.return_type)
    }

    fn check_print_builtin(
        &self,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        if args.len() != 1 {
            return Err(SemanticError::new(
                "`print` expects exactly one argument",
                span,
            ));
        }
        if args[0].mode != ArgMode::Owned {
            return Err(SemanticError::new(
                "`print` arguments do not take `con` or `mut` mode markers",
                args[0].span,
            ));
        }

        let arg_ty = self.check_expr(&args[0].expr, locals, ValueUse::Borrow)?;
        if !self.is_printable_type(&arg_ty) {
            return Err(SemanticError::new(
                format!(
                    "cannot print value of type `{}` in v0",
                    arg_ty.source_name()
                ),
                args[0].span,
            ));
        }
        Ok(Type::Unit)
    }

    fn check_len_builtin(
        &self,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        if args.len() != 1 {
            return Err(SemanticError::new(
                "`len` expects exactly one argument",
                span,
            ));
        }
        if args[0].mode != ArgMode::Owned {
            return Err(SemanticError::new(
                "`len` arguments do not take `con` or `mut` mode markers",
                args[0].span,
            ));
        }

        let arg_ty = self.check_expr(&args[0].expr, locals, ValueUse::Borrow)?;
        match &arg_ty {
            Type::Array { .. } => {}
            Type::Slice(_) => {
                if !is_direct_borrow_expr(&args[0].expr) {
                    return Err(SemanticError::new(
                        "`len` on slices requires a local-rooted slice source in v0",
                        args[0].span,
                    ));
                }
            }
            _ => {
                return Err(SemanticError::new(
                    format!(
                        "`len` expects a fixed-size array or slice, got `{}`",
                        arg_ty.source_name()
                    ),
                    args[0].span,
                ));
            }
        }

        Ok(Type::Int)
    }

    fn check_append_builtin(
        &self,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        if args.len() != 2 {
            return Err(SemanticError::new(
                "`append` expects exactly two arguments",
                span,
            ));
        }
        if args[0].mode != ArgMode::Owned || args[1].mode != ArgMode::Owned {
            return Err(SemanticError::new(
                "`append` arguments do not take `con` or `mut` mode markers",
                span,
            ));
        }

        let slice_ty = if is_field_place_expr(&args[0].expr) {
            self.check_expr(&args[0].expr, locals, ValueUse::Borrow)?
        } else {
            self.check_expr(&args[0].expr, locals, ValueUse::Owned)?
        };
        let Type::Slice(element_ty) = &slice_ty else {
            return Err(SemanticError::new(
                format!(
                    "`append` first argument must be a slice, got `{}`",
                    slice_ty.source_name()
                ),
                args[0].span,
            ));
        };
        let item_ty = self.check_expr_with_expected(
            &args[1].expr,
            locals,
            ValueUse::Owned,
            Some(element_ty),
        )?;
        if item_ty != **element_ty {
            return Err(SemanticError::new(
                format!(
                    "`append` item type mismatch: expected `{}`, got `{}`",
                    element_ty.source_name(),
                    item_ty.source_name()
                ),
                args[1].span,
            ));
        }

        Ok(slice_ty)
    }

    fn check_method_call(
        &self,
        base: &Expr,
        method_name: &str,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let base_ty = self.check_receiver_probe_type(base, locals)?;
        let Type::Struct(_) = base_ty else {
            return Err(SemanticError::new(
                format!(
                    "method call requires a struct receiver, got `{}`",
                    base_ty.source_name()
                ),
                base.span,
            ));
        };
        let key = MethodKey {
            receiver: base_ty.clone(),
            name: method_name.to_string(),
        };
        self.check_method_visibility(&key, span)?;
        let sig = self.method_sig(&key, span)?;
        let mut call_borrows = Vec::new();
        match sig.receiver.mode {
            ParamMode::Owned => {
                let receiver_ty = self.check_expr_with_expected(
                    base,
                    locals,
                    ValueUse::Owned,
                    Some(&sig.receiver.ty),
                )?;
                if receiver_ty != sig.receiver.ty {
                    return Err(SemanticError::new(
                        format!(
                            "receiver type mismatch for `{method_name}`: expected `{}`, got `{}`",
                            sig.receiver.ty.source_name(),
                            receiver_ty.source_name()
                        ),
                        base.span,
                    ));
                }
            }
            ParamMode::Con => {
                register_receiver_borrow(base, &mut call_borrows, BorrowKind::Shared)?;
                let receiver_ty = self.check_receiver_borrow(base, locals, false)?;
                if receiver_ty != sig.receiver.ty {
                    return Err(SemanticError::new(
                        format!(
                            "receiver type mismatch for `{method_name}`: expected `{}`, got `{}`",
                            sig.receiver.ty.source_name(),
                            receiver_ty.source_name()
                        ),
                        base.span,
                    ));
                }
            }
            ParamMode::Mut => {
                register_receiver_borrow(base, &mut call_borrows, BorrowKind::Exclusive)?;
                let receiver_ty = self.check_receiver_borrow(base, locals, true)?;
                if receiver_ty != sig.receiver.ty {
                    return Err(SemanticError::new(
                        format!(
                            "receiver type mismatch for `{method_name}`: expected `{}`, got `{}`",
                            sig.receiver.ty.source_name(),
                            receiver_ty.source_name()
                        ),
                        base.span,
                    ));
                }
            }
        }

        self.check_call_args(
            method_name,
            args,
            &sig.function.params,
            locals,
            call_borrows,
            span,
        )?;
        Ok(sig.function.return_type.clone())
    }

    fn check_method_visibility(&self, key: &MethodKey, span: Span) -> Result<(), SemanticError> {
        let Some(package_graph) = self.package_graph else {
            return Ok(());
        };
        let Some(access) = self.method_access.get(key) else {
            return Ok(());
        };
        if access.visibility == Visibility::Public {
            return Ok(());
        }
        let caller = package_graph
            .package_for_source(span.source)
            .ok_or_else(|| {
                SemanticError::new("method call source is not part of the package graph", span)
            })?;
        if caller.path != access.package_path {
            return Err(SemanticError::new(
                format!(
                    "method `{}` is private to package `{}`",
                    key.name, access.package_path
                ),
                span,
            ));
        }
        Ok(())
    }

    fn check_receiver_probe_type(
        &self,
        receiver: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        if is_direct_borrow_expr(receiver) {
            return self.resolve_borrow_expr_type(receiver, locals);
        }
        self.check_expr(receiver, locals, ValueUse::Borrow)
    }

    fn check_call_args(
        &self,
        name: &str,
        args: &[Arg],
        params: &[ParamSig],
        locals: &mut HashMap<String, Local>,
        mut call_borrows: Vec<(BorrowPlace, BorrowKind)>,
        span: Span,
    ) -> Result<(), SemanticError> {
        if args.len() != params.len() {
            return Err(SemanticError::new(
                format!(
                    "function `{name}` expects {} arguments, got {}",
                    params.len(),
                    args.len()
                ),
                span,
            ));
        }

        for (arg, param) in args.iter().zip(params.iter()) {
            let arg_ty = match (param.mode, arg.mode) {
                (ParamMode::Owned, ArgMode::Owned) => self.check_expr_with_expected(
                    &arg.expr,
                    locals,
                    ValueUse::Owned,
                    Some(&param.ty),
                )?,
                (ParamMode::Con, ArgMode::Con) => {
                    self.register_call_borrow(arg, &mut call_borrows, BorrowKind::Shared)?;
                    self.check_borrow_arg(arg, locals, false)?
                }
                (ParamMode::Mut, ArgMode::Mut) => {
                    self.register_call_borrow(arg, &mut call_borrows, BorrowKind::Exclusive)?;
                    self.check_borrow_arg(arg, locals, true)?
                }
                (ParamMode::Owned, _) => {
                    return Err(SemanticError::new(
                        format!("parameter `{}` expects an owned argument", param.name),
                        arg.span,
                    ));
                }
                (ParamMode::Con, _) => {
                    return Err(SemanticError::new(
                        format!("parameter `{}` expects `con` argument", param.name),
                        arg.span,
                    ));
                }
                (ParamMode::Mut, _) => {
                    return Err(SemanticError::new(
                        format!("parameter `{}` expects `mut` argument", param.name),
                        arg.span,
                    ));
                }
            };
            if arg_ty != param.ty {
                return Err(SemanticError::new(
                    format!(
                        "argument type mismatch for `{name}`: expected `{}`, got `{}`",
                        param.ty.source_name(),
                        arg_ty.source_name()
                    ),
                    arg.span,
                ));
            }
        }

        Ok(())
    }

    fn check_some_constructor(
        &self,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let arg = expect_constructor_arg("Some", args, span)?;
        let expected_payload = match expected {
            Some(Type::Option(inner)) => Some(inner.as_ref()),
            _ => None,
        };
        let payload_ty =
            self.check_expr_with_expected(&arg.expr, locals, ValueUse::Owned, expected_payload)?;
        Ok(Type::Option(Box::new(payload_ty)))
    }

    fn check_none_constructor(
        &self,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let Some(expected) = expected else {
            return Err(SemanticError::new(
                "`None` requires expected `Option[T]` context",
                span,
            ));
        };
        if !matches!(expected, Type::Option(_)) {
            return Err(SemanticError::new(
                "`None` requires expected `Option[T]` context",
                span,
            ));
        }
        Ok(expected.clone())
    }

    fn check_ok_constructor(
        &self,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let arg = expect_constructor_arg("Ok", args, span)?;
        let Some(Type::Result(expected_ok, expected_err)) = expected else {
            return Err(SemanticError::new(
                "`Ok` requires expected `Result[T, E]` context",
                span,
            ));
        };
        let ok_ty = self.check_expr_with_expected(
            &arg.expr,
            locals,
            ValueUse::Owned,
            Some(expected_ok.as_ref()),
        )?;
        Ok(Type::Result(
            Box::new(ok_ty),
            Box::new(expected_err.as_ref().clone()),
        ))
    }

    fn check_err_constructor(
        &self,
        args: &[Arg],
        locals: &mut HashMap<String, Local>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let arg = expect_constructor_arg("Err", args, span)?;
        let Some(Type::Result(expected_ok, expected_err)) = expected else {
            return Err(SemanticError::new(
                "`Err` requires expected `Result[T, E]` context",
                span,
            ));
        };
        let err_ty = self.check_expr_with_expected(
            &arg.expr,
            locals,
            ValueUse::Owned,
            Some(expected_err.as_ref()),
        )?;
        Ok(Type::Result(
            Box::new(expected_ok.as_ref().clone()),
            Box::new(err_ty),
        ))
    }

    fn register_call_borrow(
        &self,
        arg: &Arg,
        call_borrows: &mut Vec<(BorrowPlace, BorrowKind)>,
        kind: BorrowKind,
    ) -> Result<(), SemanticError> {
        let place = borrow_arg_place(arg)?;
        register_borrow_place(place, kind, arg.span, call_borrows)
    }

    fn check_borrow_arg(
        &self,
        arg: &Arg,
        locals: &mut HashMap<String, Local>,
        mutable: bool,
    ) -> Result<Type, SemanticError> {
        let BorrowPlace { root: name, fields } = borrow_arg_place(arg)?;
        let Some(local) = locals.get(&name) else {
            return Err(SemanticError::new(
                format!("unknown variable `{name}`"),
                arg.expr.span,
            ));
        };
        if local.moved {
            return Err(SemanticError::new(
                format!("borrow of moved value `{name}`"),
                arg.expr.span,
            ));
        }
        if mutable && !local.mutable {
            let message = if fields.is_empty() {
                format!("cannot mutably borrow immutable binding `{name}`")
            } else if fields.iter().any(|field| field == INDEX_BORROW_SEGMENT) {
                format!("cannot mutably borrow place of immutable binding `{name}`")
            } else {
                format!("cannot mutably borrow field of immutable binding `{name}`")
            };
            return Err(SemanticError::new(message, arg.span));
        }

        self.resolve_borrow_expr_type(&arg.expr, locals)
    }

    fn resolve_borrow_expr_type(
        &self,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        match &expr.kind {
            ExprKind::Var(name) => {
                let Some(local) = locals.get(name) else {
                    return Err(SemanticError::new(
                        format!("unknown variable `{name}`"),
                        expr.span,
                    ));
                };
                if local.moved {
                    return Err(SemanticError::new(
                        format!("borrow of moved value `{name}`"),
                        expr.span,
                    ));
                }
                Ok(local.ty.clone())
            }
            ExprKind::FieldAccess { base, field } => {
                let base_ty = self.resolve_borrow_expr_type(base, locals)?;
                self.resolve_field_path_type(&base_ty, std::slice::from_ref(field), expr.span, "field borrow")
            }
            ExprKind::Index { base, index } => {
                let base_ty = self.resolve_borrow_expr_type(base, locals)?;
                match base_ty {
                    Type::Array { len, element } => {
                        self.check_index_expr(index, locals, len)?;
                        Ok(*element)
                    }
                    Type::Slice(element) => {
                        if !is_direct_borrow_expr(base) {
                            return Err(SemanticError::new(
                                "slice element borrow requires a local-rooted slice source in v0",
                                base.span,
                            ));
                        }
                        let index_ty = self.check_expr(index, locals, ValueUse::Owned)?;
                        self.validate_index_type_and_non_negative_literal(
                            index, &index_ty, "slice",
                        )?;
                        Ok(*element)
                    }
                    _ => Err(SemanticError::new(
                        format!(
                            "element borrow target must be a fixed-size array or slice, got `{}`",
                            base_ty.source_name()
                        ),
                        base.span,
                    )),
                }
            }
            _ => Err(SemanticError::new(
                "borrow arguments must be direct local variables, direct local fields, or direct local array/slice elements in v0",
                expr.span,
            )),
        }
    }

    fn check_receiver_borrow(
        &self,
        receiver: &Expr,
        locals: &mut HashMap<String, Local>,
        mutable: bool,
    ) -> Result<Type, SemanticError> {
        let place = direct_borrow_place(
            receiver,
            "method receivers with `con` or `mut` must be direct local variables, direct local fields, or direct local array/slice elements in v0",
        )?;
        let name = place.root.clone();
        let fields = place.fields.clone();
        {
            let Some(local) = locals.get(&name) else {
                return Err(SemanticError::new(
                    format!("unknown variable `{name}`"),
                    receiver.span,
                ));
            };
            if local.moved {
                return Err(SemanticError::new(
                    format!("borrow of moved value `{name}`"),
                    receiver.span,
                ));
            }
            if mutable && !local.mutable {
                let message = if fields.is_empty() {
                    format!("cannot mutably borrow immutable binding `{name}`")
                } else if fields.iter().any(|field| field == INDEX_BORROW_SEGMENT) {
                    format!("cannot mutably borrow place of immutable binding `{name}`")
                } else {
                    format!("cannot mutably borrow field of immutable binding `{name}`")
                };
                return Err(SemanticError::new(message, receiver.span));
            }
        }

        self.resolve_borrow_expr_type(receiver, locals)
    }

    fn check_binary(
        &self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<Type, SemanticError> {
        let value_use = match op {
            BinaryOp::Equal | BinaryOp::NotEqual => ValueUse::Borrow,
            _ => ValueUse::Owned,
        };
        let left_ty = self.check_expr(left, locals, value_use)?;
        let right_ty = self.check_expr(right, locals, value_use)?;
        if left_ty != right_ty {
            return Err(SemanticError::new(
                "binary operands must have the same type",
                left.span.join(right.span),
            ));
        }

        match op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder => {
                if left_ty == Type::Int {
                    check_const_int_arithmetic(op, left, right)?;
                    Ok(Type::Int)
                } else {
                    Err(SemanticError::new(
                        "arithmetic operators currently require `int` operands",
                        left.span.join(right.span),
                    ))
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                if matches!(left_ty, Type::Int | Type::Bool | Type::String) {
                    Ok(Type::Bool)
                } else {
                    Err(SemanticError::new(
                        "equality currently supports `int`, `bool`, and `string` operands",
                        left.span.join(right.span),
                    ))
                }
            }
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                if left_ty == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(SemanticError::new(
                        "logical operators require `bool` operands",
                        left.span.join(right.span),
                    ))
                }
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                if left_ty == Type::Int {
                    Ok(Type::Bool)
                } else {
                    Err(SemanticError::new(
                        "ordering comparisons currently require `int` operands",
                        left.span.join(right.span),
                    ))
                }
            }
        }
    }

    fn function_sig(&self, name: &str, span: Span) -> Result<&FunctionSig, SemanticError> {
        self.signatures
            .get(name)
            .ok_or_else(|| SemanticError::new(format!("unknown function `{name}`"), span))
    }

    fn method_sig(&self, key: &MethodKey, span: Span) -> Result<&MethodSig, SemanticError> {
        self.methods.get(key).ok_or_else(|| {
            SemanticError::new(
                format!(
                    "unknown method `{}` on `{}`",
                    key.name,
                    key.receiver.source_name()
                ),
                span,
            )
        })
    }

    fn struct_sig(&self, name: &str, span: Span) -> Result<&StructSig, SemanticError> {
        self.structs
            .get(name)
            .ok_or_else(|| SemanticError::new(format!("unknown struct `{name}`"), span))
    }

    fn resolve_field_path_type(
        &self,
        root_ty: &Type,
        fields: &[String],
        span: Span,
        context: &str,
    ) -> Result<Type, SemanticError> {
        let mut current_ty = root_ty.clone();
        for field in fields {
            let Type::Struct(type_name) = &current_ty else {
                return Err(SemanticError::new(
                    format!(
                        "{context} requires a struct value, got `{}`",
                        current_ty.source_name()
                    ),
                    span,
                ));
            };
            let struct_sig = self.struct_sig(type_name, span)?;
            let Some(field_sig) = struct_sig
                .fields
                .iter()
                .find(|candidate| candidate.name == *field)
            else {
                return Err(SemanticError::new(
                    format!("unknown field `{field}` on `{type_name}`"),
                    span,
                ));
            };
            current_ty = field_sig.ty.clone();
        }

        Ok(current_ty)
    }

    fn is_printable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Int | Type::Bool | Type::String => true,
            Type::Option(inner) => self.is_printable_type(inner),
            Type::Result(ok, err) => self.is_printable_type(ok) && self.is_printable_type(err),
            Type::Struct(name) => self.structs.get(name.as_str()).is_some_and(|struct_sig| {
                struct_sig
                    .fields
                    .iter()
                    .all(|field| self.is_printable_type(&field.ty))
            }),
            Type::Unit | Type::Array { .. } | Type::Slice(_) | Type::Function(_) => false,
        }
    }

    fn type_from_optional_ref(&self, ty: Option<&TypeRef>) -> Result<Type, SemanticError> {
        ty.map_or(Ok(Type::Unit), |ty| self.type_from_ref(ty))
    }

    fn type_from_ref(&self, ty: &TypeRef) -> Result<Type, SemanticError> {
        if let Some(function) = &ty.function {
            if ty.name != "func" || !ty.args.is_empty() || ty.array_len.is_some() || ty.slice {
                return Err(SemanticError::new(
                    "malformed function type reference",
                    ty.span,
                ));
            }
            let params = function
                .params
                .iter()
                .map(|param| {
                    Ok(FunctionParamType {
                        mode: param.mode,
                        ty: self.type_from_ref(&param.ty)?,
                    })
                })
                .collect::<Result<Vec<_>, SemanticError>>()?;
            return Ok(Type::Function(FunctionType {
                mutable: function.mutable,
                params,
                return_type: Box::new(self.type_from_ref(&function.return_type)?),
            }));
        }
        if ty.slice {
            if ty.name != "Slice" || ty.args.len() != 1 || ty.array_len.is_some() {
                return Err(SemanticError::new(
                    "malformed slice type reference",
                    ty.span,
                ));
            }
            return Ok(Type::Slice(Box::new(self.type_from_ref(&ty.args[0])?)));
        }

        if let Some(len) = ty.array_len {
            if ty.name != "Array" || ty.args.len() != 1 {
                return Err(SemanticError::new(
                    "malformed fixed-size array type reference",
                    ty.span,
                ));
            }
            return Ok(Type::Array {
                len,
                element: Box::new(self.type_from_ref(&ty.args[0])?),
            });
        }

        match ty.name.as_str() {
            "int" if ty.args.is_empty() => Ok(Type::Int),
            "bool" if ty.args.is_empty() => Ok(Type::Bool),
            "string" if ty.args.is_empty() => Ok(Type::String),
            "unit" if ty.args.is_empty() => Ok(Type::Unit),
            "int" | "bool" | "string" | "unit" => Err(SemanticError::new(
                format!("primitive type `{}` does not take type arguments", ty.name),
                ty.span,
            )),
            "Option" => {
                if ty.args.len() != 1 {
                    return Err(SemanticError::new(
                        "`Option` expects exactly 1 type argument",
                        ty.span,
                    ));
                }
                Ok(Type::Option(Box::new(self.type_from_ref(&ty.args[0])?)))
            }
            "Result" => {
                if ty.args.len() != 2 {
                    return Err(SemanticError::new(
                        "`Result` expects exactly 2 type arguments",
                        ty.span,
                    ));
                }
                Ok(Type::Result(
                    Box::new(self.type_from_ref(&ty.args[0])?),
                    Box::new(self.type_from_ref(&ty.args[1])?),
                ))
            }
            name if ty.args.is_empty() && self.structs.contains_key(name) => {
                Ok(Type::Struct(name.to_string()))
            }
            name if self.structs.contains_key(name) => Err(SemanticError::new(
                format!("struct type `{}` does not take type arguments", ty.name),
                ty.span,
            )),
            _ => Err(SemanticError::new(
                format!("unknown type `{}`", ty.name),
                ty.span,
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct Local {
    ty: Type,
    mutable: bool,
    borrowed: bool,
    moved: bool,
    range_source: bool,
    scope_depth: usize,
}

struct IfExprParts<'a> {
    condition: &'a Expr,
    then_branch: &'a Expr,
    else_branch: &'a Expr,
    span: Span,
}

struct RangeForParts<'a> {
    index_name: &'a str,
    value_name: &'a str,
    source: &'a Expr,
    body: &'a Block,
    span: Span,
}

struct MatchStmtParts<'a> {
    scrutinee: &'a Expr,
    arms: &'a [MatchBlockArm],
    span: Span,
}

struct IfBranchCheck {
    then_ty: Type,
    then_locals: HashMap<String, Local>,
    else_ty: Type,
    else_locals: HashMap<String, Local>,
}

struct PreparedMatchArm<'a> {
    expr: &'a Expr,
    binding: Option<(&'a str, Type)>,
}

struct MatchArmCheck {
    ty: Type,
    locals: HashMap<String, Local>,
}

struct PreparedMatchBlockArm<'a> {
    block: &'a Block,
    binding: Option<(&'a str, Type)>,
}

struct MatchBlockArmCheck {
    returns: bool,
    locals: HashMap<String, Local>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueUse {
    Owned,
    Borrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BorrowKind {
    Shared,
    Exclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BorrowPlace {
    root: String,
    fields: Vec<String>,
}

impl BorrowPlace {
    fn root(root: String) -> Self {
        Self {
            root,
            fields: Vec::new(),
        }
    }

    fn display(&self) -> String {
        if self.fields.is_empty() {
            return self.root.clone();
        }

        let mut display = self.root.clone();
        for field in &self.fields {
            if field == INDEX_BORROW_SEGMENT {
                display.push_str("[?]");
            } else {
                display.push('.');
                display.push_str(field);
            }
        }
        display
    }

    fn overlaps(&self, other: &Self) -> bool {
        if self.root != other.root {
            return false;
        }
        let common_len = self.fields.len().min(other.fields.len());
        self.fields[..common_len] == other.fields[..common_len]
    }
}

const INDEX_BORROW_SEGMENT: &str = "[]";

fn borrow_arg_place(arg: &Arg) -> Result<BorrowPlace, SemanticError> {
    direct_borrow_place(
        &arg.expr,
        "borrow arguments must be direct local variables, direct local fields, or direct local array/slice elements in v0",
    )
}

fn direct_borrow_place(expr: &Expr, message: &'static str) -> Result<BorrowPlace, SemanticError> {
    match &expr.kind {
        ExprKind::Var(name) => Ok(BorrowPlace::root(name.clone())),
        ExprKind::FieldAccess { base, field } => {
            let mut place = direct_borrow_place(base, message)?;
            place.fields.push(field.clone());
            Ok(place)
        }
        ExprKind::Index { base, .. } => {
            let mut place = direct_borrow_place(base, message)?;
            place.fields.push(INDEX_BORROW_SEGMENT.to_string());
            Ok(place)
        }
        _ => Err(SemanticError::new(message, expr.span)),
    }
}

fn is_direct_borrow_expr(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Var(_) => true,
        ExprKind::FieldAccess { base, .. } | ExprKind::Index { base, .. } => {
            is_direct_borrow_expr(base)
        }
        _ => false,
    }
}

fn is_field_place_expr(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::FieldAccess { .. }) && is_direct_borrow_expr(expr)
}

fn is_same_field_target_expr(base: &Expr, field: &str, expr: &Expr) -> bool {
    let ExprKind::FieldAccess {
        base: expr_base,
        field: expr_field,
    } = &expr.kind
    else {
        return false;
    };
    expr_field == field && is_same_direct_field_path(base, expr_base)
}

fn is_same_direct_field_path(left: &Expr, right: &Expr) -> bool {
    match (&left.kind, &right.kind) {
        (ExprKind::Var(left), ExprKind::Var(right)) => left == right,
        (
            ExprKind::FieldAccess {
                base: left_base,
                field: left_field,
            },
            ExprKind::FieldAccess {
                base: right_base,
                field: right_field,
            },
        ) => left_field == right_field && is_same_direct_field_path(left_base, right_base),
        (
            ExprKind::Index {
                base: left_base,
                index: left_index,
            },
            ExprKind::Index {
                base: right_base,
                index: right_index,
            },
        ) => {
            is_same_direct_field_path(left_base, right_base)
                && is_stable_place_index_expr(left_index)
                && is_stable_place_index_expr(right_index)
                && is_same_expr_ignoring_span(left_index, right_index)
        }
        _ => false,
    }
}

fn is_stable_place_index_expr(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Int(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Nil => true,
        ExprKind::Var(_) => true,
        ExprKind::Unary { expr, .. } => is_stable_place_index_expr(expr),
        ExprKind::Binary { left, right, .. } => {
            is_stable_place_index_expr(left) && is_stable_place_index_expr(right)
        }
        ExprKind::FieldAccess { base, .. } => is_stable_place_index_expr(base),
        ExprKind::Index { base, index } => {
            is_stable_place_index_expr(base) && is_stable_place_index_expr(index)
        }
        ExprKind::TypeApply { .. } => false,
        ExprKind::If { .. }
        | ExprKind::Match { .. }
        | ExprKind::StructLiteral { .. }
        | ExprKind::ArrayLiteral { .. }
        | ExprKind::FunctionLiteral(_)
        | ExprKind::Call { .. } => false,
    }
}

fn is_same_expr_ignoring_span(left: &Expr, right: &Expr) -> bool {
    match (&left.kind, &right.kind) {
        (ExprKind::Int(left), ExprKind::Int(right)) => left == right,
        (ExprKind::String(left), ExprKind::String(right)) => left == right,
        (ExprKind::Bool(left), ExprKind::Bool(right)) => left == right,
        (ExprKind::Nil, ExprKind::Nil) => true,
        (ExprKind::Var(left), ExprKind::Var(right)) => left == right,
        (
            ExprKind::Unary {
                op: left_op,
                expr: left_expr,
            },
            ExprKind::Unary {
                op: right_op,
                expr: right_expr,
            },
        ) => left_op == right_op && is_same_expr_ignoring_span(left_expr, right_expr),
        (
            ExprKind::Binary {
                op: left_op,
                left: left_left,
                right: left_right,
            },
            ExprKind::Binary {
                op: right_op,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_op == right_op
                && is_same_expr_ignoring_span(left_left, right_left)
                && is_same_expr_ignoring_span(left_right, right_right)
        }
        (
            ExprKind::FieldAccess {
                base: left_base,
                field: left_field,
            },
            ExprKind::FieldAccess {
                base: right_base,
                field: right_field,
            },
        ) => left_field == right_field && is_same_expr_ignoring_span(left_base, right_base),
        (
            ExprKind::Index {
                base: left_base,
                index: left_index,
            },
            ExprKind::Index {
                base: right_base,
                index: right_index,
            },
        ) => {
            is_same_expr_ignoring_span(left_base, right_base)
                && is_same_expr_ignoring_span(left_index, right_index)
        }
        _ => false,
    }
}

fn is_blank_identifier(name: &str) -> bool {
    name == "_"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClosureCaptureUse {
    name: String,
    mutable: bool,
}

fn collect_closure_captures(
    function: &FunctionLiteral,
    outer_locals: &HashMap<String, Local>,
    methods: &HashMap<MethodKey, MethodSig>,
    structs: &HashMap<String, StructSig>,
    recursive_binding: Option<&str>,
) -> Result<Vec<ClosureCaptureUse>, SemanticError> {
    let mut collector = ClosureCaptureCollector {
        outer_locals,
        methods,
        structs,
        recursive_binding,
        recursive_reference: None,
        captures: Vec::new(),
        capture_indices: HashMap::new(),
    };
    let mut bound = function
        .params
        .iter()
        .map(|param| param.name.clone())
        .collect::<HashSet<_>>();
    collector.visit_block(&function.body, &mut bound)?;
    if let Some(span) = collector.recursive_reference {
        let name = recursive_binding.expect("recursive reference has an initializing binding");
        return Err(SemanticError::new(
            format!("recursive closure `{name}` is not supported in v0.3"),
            span,
        ));
    }
    Ok(collector.captures)
}

struct ClosureCaptureCollector<'a> {
    outer_locals: &'a HashMap<String, Local>,
    methods: &'a HashMap<MethodKey, MethodSig>,
    structs: &'a HashMap<String, StructSig>,
    recursive_binding: Option<&'a str>,
    recursive_reference: Option<Span>,
    captures: Vec<ClosureCaptureUse>,
    capture_indices: HashMap<String, usize>,
}

impl ClosureCaptureCollector<'_> {
    fn visit_block(
        &mut self,
        block: &Block,
        bound: &mut HashSet<String>,
    ) -> Result<(), SemanticError> {
        for stmt in &block.statements {
            self.visit_stmt(stmt, bound)?;
        }
        Ok(())
    }

    fn visit_stmt(
        &mut self,
        stmt: &Stmt,
        bound: &mut HashSet<String>,
    ) -> Result<(), SemanticError> {
        match &stmt.kind {
            StmtKind::Let { name, expr, .. } => {
                self.visit_expr(expr, bound)?;
                bound.insert(name.clone());
            }
            StmtKind::Assign { name, expr } => {
                self.visit_name(name, bound, true, stmt.span);
                self.visit_expr(expr, bound)?;
            }
            StmtKind::FieldAssign { base, expr, .. } => {
                self.visit_mutated_place(base, bound)?;
                self.visit_expr(expr, bound)?;
            }
            StmtKind::IndexAssign { base, index, expr } => {
                self.visit_mutated_place(base, bound)?;
                self.visit_expr(index, bound)?;
                self.visit_expr(expr, bound)?;
            }
            StmtKind::Return { expr } | StmtKind::Expr { expr } => {
                self.visit_expr(expr, bound)?;
            }
            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.visit_expr(condition, bound)?;
                let mut then_bound = bound.clone();
                self.visit_block(then_block, &mut then_bound)?;
                if let Some(else_block) = else_block {
                    let mut else_bound = bound.clone();
                    self.visit_block(else_block, &mut else_bound)?;
                }
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
            } => {
                let mut loop_bound = bound.clone();
                if let Some(ForInit::Let { name, expr, .. }) = init {
                    self.visit_expr(expr, &loop_bound)?;
                    loop_bound.insert(name.clone());
                }
                if let Some(condition) = condition {
                    self.visit_expr(condition, &loop_bound)?;
                }
                let mut body_bound = loop_bound.clone();
                self.visit_block(body, &mut body_bound)?;
                if let Some(ForPost::Assign { target, expr }) = post {
                    self.visit_mutated_place(target, &loop_bound)?;
                    self.visit_expr(expr, &loop_bound)?;
                }
            }
            StmtKind::RangeFor {
                index_name,
                value_name,
                source,
                body,
            } => {
                self.visit_expr(source, bound)?;
                let mut body_bound = bound.clone();
                if !is_blank_identifier(index_name) {
                    body_bound.insert(index_name.clone());
                }
                if !is_blank_identifier(value_name) {
                    body_bound.insert(value_name.clone());
                }
                self.visit_block(body, &mut body_bound)?;
            }
            StmtKind::Match { scrutinee, arms } => {
                self.visit_expr(scrutinee, bound)?;
                for arm in arms {
                    let mut arm_bound = bound.clone();
                    if let Some(binding) = match_pattern_binding(&arm.pattern) {
                        arm_bound.insert(binding.to_string());
                    }
                    self.visit_block(&arm.block, &mut arm_bound)?;
                }
            }
            StmtKind::Break | StmtKind::Continue => {}
        }
        Ok(())
    }

    fn visit_expr(&mut self, expr: &Expr, bound: &HashSet<String>) -> Result<(), SemanticError> {
        match &expr.kind {
            ExprKind::Var(name) => self.visit_name(name, bound, false, expr.span),
            ExprKind::FunctionLiteral(function) => {
                let mut nested_bound = bound.clone();
                nested_bound.extend(function.params.iter().map(|param| param.name.clone()));
                self.visit_block(&function.body, &mut nested_bound)?;
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.visit_expr(condition, bound)?;
                self.visit_expr(then_branch, bound)?;
                self.visit_expr(else_branch, bound)?;
            }
            ExprKind::Match { scrutinee, arms } => {
                self.visit_expr(scrutinee, bound)?;
                for arm in arms {
                    let mut arm_bound = bound.clone();
                    if let Some(binding) = match_pattern_binding(&arm.pattern) {
                        arm_bound.insert(binding.to_string());
                    }
                    self.visit_expr(&arm.expr, &arm_bound)?;
                }
            }
            ExprKind::StructLiteral { fields, .. } => {
                for field in fields {
                    self.visit_expr(&field.expr, bound)?;
                }
            }
            ExprKind::ArrayLiteral { elements, .. } => {
                for element in elements {
                    self.visit_expr(element, bound)?;
                }
            }
            ExprKind::FieldAccess { base, .. } => self.visit_expr(base, bound)?,
            ExprKind::Index { base, index } => {
                self.visit_expr(base, bound)?;
                self.visit_expr(index, bound)?;
            }
            ExprKind::TypeApply { base, .. } => self.visit_expr(base, bound)?,
            ExprKind::Call { callee, args } => {
                self.visit_call_callee(callee, bound)?;
                for arg in args {
                    if matches!(arg.mode, ArgMode::Mut) {
                        self.visit_mutated_place(&arg.expr, bound)?;
                    } else {
                        self.visit_expr(&arg.expr, bound)?;
                    }
                }
            }
            ExprKind::Unary { expr, .. } => self.visit_expr(expr, bound)?,
            ExprKind::Binary { left, right, .. } => {
                self.visit_expr(left, bound)?;
                self.visit_expr(right, bound)?;
            }
            ExprKind::Int(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Nil => {}
        }
        Ok(())
    }

    fn visit_call_callee(
        &mut self,
        callee: &Expr,
        bound: &HashSet<String>,
    ) -> Result<(), SemanticError> {
        match &callee.kind {
            ExprKind::Var(name) => {
                let mutable = !bound.contains(name)
                    && matches!(
                        self.outer_locals.get(name).map(|local| &local.ty),
                        Some(Type::Function(function)) if function.mutable
                    );
                self.visit_name(name, bound, mutable, callee.span);
            }
            ExprKind::FieldAccess { base, field } => {
                let mutable_receiver = self
                    .outer_place_type(base, bound)
                    .and_then(|receiver| {
                        self.methods.get(&MethodKey {
                            receiver,
                            name: field.clone(),
                        })
                    })
                    .is_some_and(|method| matches!(method.receiver.mode, ParamMode::Mut));
                if mutable_receiver {
                    self.visit_mutated_place(base, bound)?;
                } else {
                    self.visit_expr(base, bound)?;
                }
            }
            _ => self.visit_expr(callee, bound)?,
        }
        Ok(())
    }

    fn visit_mutated_place(
        &mut self,
        expr: &Expr,
        bound: &HashSet<String>,
    ) -> Result<(), SemanticError> {
        match &expr.kind {
            ExprKind::Var(name) => self.visit_name(name, bound, true, expr.span),
            ExprKind::FieldAccess { base, .. } => self.visit_mutated_place(base, bound)?,
            ExprKind::Index { base, index } => {
                self.visit_mutated_place(base, bound)?;
                self.visit_expr(index, bound)?;
            }
            _ => self.visit_expr(expr, bound)?,
        }
        Ok(())
    }

    fn outer_place_type(&self, expr: &Expr, bound: &HashSet<String>) -> Option<Type> {
        match &expr.kind {
            ExprKind::Var(name) if !bound.contains(name) => {
                self.outer_locals.get(name).map(|local| local.ty.clone())
            }
            ExprKind::FieldAccess { base, field } => {
                let Type::Struct(name) = self.outer_place_type(base, bound)? else {
                    return None;
                };
                self.structs
                    .get(name.as_str())?
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == *field)
                    .map(|candidate| candidate.ty.clone())
            }
            ExprKind::Index { base, .. } => match self.outer_place_type(base, bound)? {
                Type::Array { element, .. } | Type::Slice(element) => Some(*element),
                _ => None,
            },
            _ => None,
        }
    }

    fn visit_name(&mut self, name: &str, bound: &HashSet<String>, mutable: bool, span: Span) {
        if bound.contains(name) {
            return;
        }
        if !self.outer_locals.contains_key(name) {
            if self.recursive_binding == Some(name) && self.recursive_reference.is_none() {
                self.recursive_reference = Some(span);
            }
            return;
        }
        if let Some(index) = self.capture_indices.get(name).copied() {
            self.captures[index].mutable |= mutable;
            return;
        }
        self.capture_indices
            .insert(name.to_string(), self.captures.len());
        self.captures.push(ClosureCaptureUse {
            name: name.to_string(),
            mutable,
        });
    }
}

fn match_pattern_binding(pattern: &MatchPattern) -> Option<&str> {
    match pattern {
        MatchPattern::Some(name) | MatchPattern::Ok(name) | MatchPattern::Err(name) => Some(name),
        MatchPattern::Binding(name) => Some(name),
        MatchPattern::Variant { payload, .. } => payload.as_deref().and_then(match_pattern_binding),
        MatchPattern::NestedBuiltin { payload, .. } => match_pattern_binding(payload),
        MatchPattern::None | MatchPattern::Wildcard => None,
    }
}

fn const_int_expr(expr: &Expr) -> Option<i64> {
    match &expr.kind {
        ExprKind::Int(value) => Some(*value),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => const_int_expr(expr)?.checked_neg(),
        ExprKind::Binary { op, left, right } => {
            let left = const_int_expr(left)?;
            let right = const_int_expr(right)?;
            match op {
                BinaryOp::Add => left.checked_add(right),
                BinaryOp::Subtract => left.checked_sub(right),
                BinaryOp::Multiply => left.checked_mul(right),
                BinaryOp::Divide => left.checked_div(right),
                BinaryOp::Remainder => left.checked_rem(right),
                _ => None,
            }
        }
        _ => None,
    }
}

fn check_const_int_arithmetic(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
) -> Result<(), SemanticError> {
    let Some(left_value) = const_int_expr(left) else {
        return Ok(());
    };
    let Some(right_value) = const_int_expr(right) else {
        return Ok(());
    };

    let result = match op {
        BinaryOp::Add => left_value.checked_add(right_value),
        BinaryOp::Subtract => left_value.checked_sub(right_value),
        BinaryOp::Multiply => left_value.checked_mul(right_value),
        BinaryOp::Divide => {
            if right_value == 0 {
                return Err(SemanticError::new("division by zero", right.span));
            }
            left_value.checked_div(right_value)
        }
        BinaryOp::Remainder => {
            if right_value == 0 {
                return Err(SemanticError::new("division by zero", right.span));
            }
            left_value.checked_rem(right_value)
        }
        _ => Some(left_value),
    };

    if result.is_none() {
        return Err(SemanticError::new(
            "integer overflow",
            left.span.join(right.span),
        ));
    }

    Ok(())
}

fn register_borrow_place(
    place: BorrowPlace,
    kind: BorrowKind,
    span: Span,
    call_borrows: &mut Vec<(BorrowPlace, BorrowKind)>,
) -> Result<(), SemanticError> {
    for (active_place, active_kind) in call_borrows.iter() {
        if !place.overlaps(active_place) {
            continue;
        }
        if matches!(
            (*active_kind, kind),
            (BorrowKind::Shared, BorrowKind::Shared)
        ) {
            continue;
        }
        return Err(SemanticError::new(
            format!(
                "borrow of `{}` overlaps with an active borrow in this call",
                place.display()
            ),
            span,
        ));
    }

    call_borrows.push((place, kind));
    Ok(())
}

fn register_receiver_borrow(
    receiver: &Expr,
    call_borrows: &mut Vec<(BorrowPlace, BorrowKind)>,
    kind: BorrowKind,
) -> Result<(), SemanticError> {
    let place = direct_borrow_place(
        receiver,
        "method receivers with `con` or `mut` must be direct local variables, direct local fields, or direct local array/slice elements in v0",
    )?;
    register_borrow_place(place, kind, receiver.span, call_borrows)
}

fn expect_constructor_arg<'a>(
    constructor: &str,
    args: &'a [Arg],
    span: Span,
) -> Result<&'a Arg, SemanticError> {
    if args.len() != 1 {
        return Err(SemanticError::new(
            format!("`{constructor}` expects exactly one argument"),
            span,
        ));
    }
    let arg = &args[0];
    if !matches!(arg.mode, ArgMode::Owned) {
        return Err(SemanticError::new(
            format!("`{constructor}` expects an owned argument"),
            arg.span,
        ));
    }
    Ok(arg)
}

fn merge_branch_moves(
    locals: &mut HashMap<String, Local>,
    then_locals: &HashMap<String, Local>,
    else_locals: &HashMap<String, Local>,
) {
    for (name, local) in locals {
        let moved_in_then = then_locals
            .get(name)
            .is_some_and(|branch| same_binding(local, branch) && branch.moved);
        let moved_in_else = else_locals
            .get(name)
            .is_some_and(|branch| same_binding(local, branch) && branch.moved);
        local.moved |= moved_in_then || moved_in_else;
    }
}

fn merge_many_branch_moves<'a>(
    locals: &mut HashMap<String, Local>,
    branch_locals: impl Iterator<Item = &'a HashMap<String, Local>>,
) {
    let branch_locals = branch_locals.collect::<Vec<_>>();
    for (name, local) in locals {
        local.moved |= branch_locals.iter().any(|branch| {
            branch
                .get(name)
                .is_some_and(|branch_local| same_binding(local, branch_local) && branch_local.moved)
        });
    }
}

fn merge_loop_body_moves(
    locals: &mut HashMap<String, Local>,
    body_locals: &HashMap<String, Local>,
) {
    for (name, local) in locals {
        local.moved |= body_locals
            .get(name)
            .is_some_and(|body| same_binding(local, body) && body.moved);
    }
}

fn reject_loop_persistent_moves(
    before_locals: &HashMap<String, Local>,
    after_locals: &HashMap<String, Local>,
    loop_scope_depth: usize,
    span: Span,
) -> Result<(), SemanticError> {
    for (name, after) in after_locals {
        let Some(before) = before_locals.get(name) else {
            continue;
        };
        if after.moved
            && !before.moved
            && same_binding(before, after)
            && after.scope_depth <= loop_scope_depth
            && !after.ty.is_copy()
        {
            return Err(SemanticError::new(
                format!("loop cannot move persistent value `{name}` in v0"),
                span,
            ));
        }
    }
    Ok(())
}

fn same_binding(left: &Local, right: &Local) -> bool {
    left.scope_depth == right.scope_depth
}

fn nested_scope_depth(locals: &HashMap<String, Local>) -> usize {
    locals
        .values()
        .map(|local| local.scope_depth)
        .max()
        .map_or(0, |depth| depth + 1)
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "int" | "bool" | "string" | "unit" | "Option" | "Result" | "Slice"
    )
}

fn reject_builtin_value_name(name: &str, span: Span) -> Result<(), SemanticError> {
    if is_builtin_value_name(name) {
        return Err(SemanticError::new(
            format!("`{name}` is a built-in value name"),
            span,
        ));
    }
    Ok(())
}

fn is_builtin_value_name(name: &str) -> bool {
    matches!(
        name,
        "print" | "len" | "append" | "Some" | "None" | "Ok" | "Err"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn check_ok(source: &str) {
        let program = parse(source).unwrap();
        check(&program).unwrap();
    }

    fn check_error(source: &str) -> SemanticError {
        let program = parse(source).unwrap();
        check(&program).unwrap_err()
    }

    #[test]
    fn checks_first_target_program() {
        check_ok(
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
        );
    }

    #[test]
    fn allows_named_function_values_parameters_returns_and_repeated_calls() {
        check_ok(
            r#"
func Double(value int) int {
    return value * 2
}

func Select() func(int) int {
    return Double
}

func Apply(value int, transform func(int) int) int {
    return transform(value)
}

func main() {
    transform := Select()
    print(transform(10))
    print(transform(11))
    print(Apply(21, Double))
}
"#,
        );
    }

    #[test]
    fn allows_plain_closures_with_owned_copy_captures() {
        let program = parse(
            r#"
func main() {
    offset := 10
    add := func(value int) int {
        return value + offset
    }
    print(offset)
    print(add(2))
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();

        assert_eq!(checked.closures.len(), 1);
        assert_eq!(checked.closures[0].captures.len(), 1);
        assert_eq!(checked.closures[0].captures[0].name, "offset");
        assert_eq!(checked.closures[0].captures[0].ty, Type::Int);
        assert!(!checked.closures[0].captures[0].mutable);
        assert_eq!(
            checked.closures[0].function_type,
            FunctionType {
                mutable: false,
                params: vec![FunctionParamType {
                    mode: ParamMode::Owned,
                    ty: Type::Int,
                }],
                return_type: Box::new(Type::Int),
            }
        );
    }

    #[test]
    fn closure_creation_moves_non_copy_captures() {
        let error = check_error(
            r#"
func main() {
    name := "kim"
    show := func() int {
        print(name)
        return 1
    }
    print(name)
    print(show())
}
"#,
        );

        assert!(error.message.contains("use of moved value `name`"));
    }

    #[test]
    fn rejects_borrowed_non_copy_closure_captures() {
        let error = check_error(
            r#"
func Make(con name string) func() int {
    return func() int {
        print(name)
        return 1
    }
}

func main() {}
"#,
        );

        assert!(error
            .message
            .contains("cannot capture borrowed non-Copy value `name`"));
    }

    #[test]
    fn rejects_mutation_of_plain_closure_captures() {
        let error = check_error(
            r#"
func main() {
    mut count := 0
    next := func() int {
        count = count + 1
        return count
    }
    print(next())
}
"#,
        );

        assert!(error
            .message
            .contains("cannot assign to immutable binding `count`"));
    }

    #[test]
    fn allows_mutable_closures_to_update_owned_captures() {
        let program = parse(
            r#"
func MakeCounter() func mut(int) int {
    mut count := 0
    return func mut(delta int) int {
        count = count + delta
        return count
    }
}

func main() {
    mut counter := MakeCounter()
    print(counter(1))
    print(counter(2))
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();

        assert_eq!(checked.closures.len(), 1);
        assert_eq!(checked.closures[0].captures.len(), 1);
        assert_eq!(checked.closures[0].captures[0].name, "count");
        assert!(checked.closures[0].captures[0].mutable);
        assert!(checked.closures[0].function_type.mutable);
    }

    #[test]
    fn allows_mutable_closures_to_read_immutable_captures() {
        check_ok(
            r#"
func main() {
    offset := 10
    mut add := func mut(value int) int {
        return value + offset
    }
    print(add(2))
}
"#,
        );
    }

    #[test]
    fn rejects_mutable_closure_updates_from_immutable_sources() {
        let error = check_error(
            r#"
func main() {
    count := 0
    mut next := func mut() int {
        count = count + 1
        return count
    }
    print(next())
}
"#,
        );

        assert!(error
            .message
            .contains("mutable closure capture `count` requires a mutable source binding"));
    }

    #[test]
    fn tracks_mut_receiver_access_as_capture_mutation() {
        let program = parse(
            r#"
type Counter struct {
    value int
}

func (mut counter Counter) Add(delta int) unit {
    counter.value = counter.value + delta
}

func main() {
    mut counter := Counter{value: 0}
    mut add := func mut(delta int) int {
        counter.Add(delta)
        return counter.value
    }
    print(add(2))
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();

        assert!(checked.closures[0].captures[0].mutable);
    }

    #[test]
    fn tracks_mut_argument_access_as_capture_mutation() {
        let program = parse(
            r#"
func Increment(mut value int) unit {
    value = value + 1
}

func main() {
    mut count := 0
    mut next := func mut() int {
        Increment(mut count)
        return count
    }
    print(next())
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();

        assert!(checked.closures[0].captures[0].mutable);
    }

    #[test]
    fn allows_nested_closures_with_capture_propagation() {
        let program = parse(
            r#"
func Make(offset int) func(int) func(int) int {
    return func(base int) func(int) int {
        return func(value int) int {
            return offset + base + value
        }
    }
}

func main() {
    addOffset := Make(10)
    addBase := addOffset(5)
    print(addBase(2))
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();

        assert_eq!(checked.closures.len(), 2);
        let inner = checked
            .closures
            .iter()
            .find(|closure| closure.captures.len() == 2)
            .unwrap();
        assert_eq!(inner.captures[0].name, "offset");
        assert_eq!(inner.captures[1].name, "base");
        let outer = checked
            .closures
            .iter()
            .find(|closure| closure.captures.len() == 1)
            .unwrap();
        assert_eq!(outer.captures[0].name, "offset");
    }

    #[test]
    fn rejects_moving_non_copy_outer_capture_into_nested_closure() {
        let error = check_error(
            r#"
func Make(values []int) func() func() int {
    return func() func() int {
        return func() int {
            return values[0]
        }
    }
}

func main() {}
"#,
        );

        assert!(error
            .message
            .contains("cannot capture borrowed non-Copy value `values`"));
    }

    #[test]
    fn allows_nested_mutable_copy_capture_propagation() {
        let program = parse(
            r#"
func Make() func mut() func mut() int {
    mut count := 0
    return func mut() func mut() int {
        return func mut() int {
            count = count + 1
            return count
        }
    }
}

func main() {
    mut makeNext := Make()
    mut next := makeNext()
    print(next())
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();

        assert_eq!(checked.closures.len(), 2);
        assert!(checked
            .closures
            .iter()
            .all(|closure| closure.function_type.mutable));
        assert!(checked
            .closures
            .iter()
            .all(|closure| closure.captures[0].mutable));
    }

    #[test]
    fn rejects_recursive_closure_initializers() {
        let error = check_error(
            r#"
func main() {
    recurse := func(value int) int {
        return recurse(value)
    }
    print(recurse(1))
}
"#,
        );

        assert!(error
            .message
            .contains("recursive closure `recurse` is not supported in v0.3"));
    }

    #[test]
    fn closure_initializer_can_capture_shadowed_outer_binding() {
        check_ok(
            r#"
func Double(value int) int {
    return value * 2
}

func main() {
    transform := Double
    if true {
        transform := func(value int) int {
            return transform(value)
        }
        print(transform(2))
    }
}
"#,
        );
    }

    #[test]
    fn treats_function_value_bindings_as_move_only() {
        let error = check_error(
            r#"
func Double(value int) int {
    return value * 2
}

func main() {
    transform := Double
    moved := transform
    print(transform(1))
}
"#,
        );

        assert!(error.message.contains("use of moved value `transform`"));
    }

    #[test]
    fn requires_mutable_access_for_mutable_function_values() {
        let error = check_error(
            r#"
func Invoke(con transform func mut(int) int) int {
    return transform(1)
}

func main() {}
"#,
        );

        assert!(error
            .message
            .contains("mutable function value `transform` requires mutable access"));

        check_ok(
            r#"
func Invoke(mut transform func mut(int) int) int {
    return transform(1)
}

func main() {}
"#,
        );
    }

    #[test]
    fn local_non_function_shadows_top_level_function_in_calls() {
        let error = check_error(
            r#"
func Double(value int) int {
    return value * 2
}

func main() {
    Double := 1
    print(Double(2))
}
"#,
        );

        assert!(error.message.contains("`Double` is not callable"));
    }

    #[test]
    fn allows_return_completeness_across_if_statement_branches() {
        check_ok(
            r#"
func main() {
    print(choose(true))
}

func choose(flag bool) int {
    if flag {
        return 1
    } else {
        return 2
    }
}
"#,
        );
    }

    #[test]
    fn allows_nested_return_completeness_across_if_statement_branches() {
        check_ok(
            r#"
func main() {
    print(choose(true, false))
}

func choose(left bool, right bool) int {
    if left {
        if right {
            return 1
        } else {
            return 2
        }
    } else {
        return 3
    }
}
"#,
        );
    }

    #[test]
    fn rejects_if_statement_return_without_else_branch_in_non_unit_function() {
        let error = check_error(
            r#"
func main() {
    print(choose(true))
}

func choose(flag bool) int {
    if flag {
        return 1
    }
}
"#,
        );
        assert!(error
            .message
            .contains("function `choose` must return `int`"));
    }

    #[test]
    fn rejects_if_statement_return_when_else_branch_does_not_return() {
        let error = check_error(
            r#"
func main() {
    print(choose(true))
}

func choose(flag bool) int {
    if flag {
        return 1
    } else {
        print(2)
    }
}
"#,
        );
        assert!(error
            .message
            .contains("function `choose` must return `int`"));
    }

    #[test]
    fn rejects_unknown_variable() {
        let error = check_error("func main() { print(missing) }");
        assert!(error.message.contains("unknown variable `missing`"));
    }

    #[test]
    fn rejects_nil() {
        let error = check_error("func main() { print(nil) }");
        assert!(error.message.contains("`nil` is reserved"));
    }

    #[test]
    fn rejects_main_with_method_receiver() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func (con self User) main() {
    print(self.name)
}
"#,
        );
        assert!(error
            .message
            .contains("`main` must not declare a method receiver"));
    }

    #[test]
    fn rejects_main_with_parameters() {
        let error = check_error("func main(value int) { print(value) }");
        assert!(error.message.contains("`main` must not take parameters"));
    }

    #[test]
    fn rejects_main_with_return_type() {
        let error = check_error("func main() int { return 0 }");
        assert!(error
            .message
            .contains("`main` must not declare a return type in v0"));
    }

    #[test]
    fn allows_struct_literal_and_field_access() {
        check_ok(
            r#"
type User struct {
    name string
    age int
}

func main() {
    user := User{name: "kim", age: 30}
    print(user.name)
    print(user.age)
}
"#,
        );
    }

    #[test]
    fn rejects_builtin_value_function_names() {
        let error = check_error(
            r#"
func print() {}

func main() {}
"#,
        );
        assert!(error.message.contains("`print` is a built-in value name"));

        let error = check_error(
            r#"
func Some(value int) Option[int] {
    return Some(value)
}

func main() {}
"#,
        );
        assert!(error.message.contains("`Some` is a built-in value name"));

        let error = check_error(
            r#"
func append(value int) int {
    return value
}

func main() {}
"#,
        );
        assert!(error.message.contains("`append` is a built-in value name"));
    }

    #[test]
    fn rejects_builtin_value_binding_names() {
        let error = check_error(
            r#"
func show(len int) {
    print(len)
}

func main() {
    show(1)
}
"#,
        );
        assert!(error.message.contains("`len` is a built-in value name"));

        let error = check_error(
            r#"
func main() {
    None := 1
    print(None)
}
"#,
        );
        assert!(error.message.contains("`None` is a built-in value name"));

        let error = check_error(
            r#"
func main() {
    append := 1
    print(append)
}
"#,
        );
        assert!(error.message.contains("`append` is a built-in value name"));
    }

    #[test]
    fn rejects_builtin_value_range_binding_names() {
        let error = check_error(
            r#"
func main() {
    values := [2]int{1, 2}
    for len := range values {
        print(len)
    }
}
"#,
        );
        assert!(error.message.contains("`len` is a built-in value name"));
    }

    #[test]
    fn rejects_builtin_value_match_payload_binding_names() {
        let error = check_error(
            r#"
func main() {
    value := Some(1)
    out := match value {
        case Some(len) { len }
        case None { 0 }
    }
    print(out)
}
"#,
        );
        assert!(error.message.contains("`len` is a built-in value name"));
    }

    #[test]
    fn rejects_top_level_type_and_function_name_conflicts() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func User() {
}

func main() {}
"#,
        );
        assert!(error
            .message
            .contains("top-level function `User` conflicts with struct `User`"));
    }

    #[test]
    fn rejects_builtin_names_in_top_level_declarations() {
        let error = check_error(
            r#"
type print struct {
    value int
}

func main() {}
"#,
        );
        assert!(error.message.contains("`print` is a built-in value name"));

        let error = check_error(
            r#"
func int() {
}

func main() {}
"#,
        );
        assert!(error.message.contains("`int` is a built-in type name"));
    }

    #[test]
    fn allows_shadowing_in_nested_if_block() {
        check_ok(
            r#"
func main() {
    value := "outer"
    if true {
        value := 1
        print(value)
    }
    print(value)
}
"#,
        );
    }

    #[test]
    fn rejects_shadowing_in_same_block() {
        let error = check_error(
            r#"
func main() {
    value := 1
    value := 2
    print(value)
}
"#,
        );
        assert!(error
            .message
            .contains("binding `value` already exists in this block"));
    }

    #[test]
    fn shadowed_inner_move_does_not_move_outer_binding() {
        check_ok(
            r#"
func main() {
    word := "outer"
    if true {
        word := "inner"
        consume(word)
    }
    print(word)
}

func consume(value string) {
}
"#,
        );
    }

    #[test]
    fn allows_for_body_to_shadow_init_binding() {
        check_ok(
            r#"
func main() {
    label := "outer"
    for mut label := 0; label < 1; label = label + 1 {
        label := "inner"
        print(label)
    }
    print(label)
}
"#,
        );
    }

    #[test]
    fn allows_range_body_to_shadow_range_binding() {
        check_ok(
            r#"
func main() {
    values := [1]int{7}
    for index, value := range values {
        value := "shadow"
        print(index)
        print(value)
    }
}
"#,
        );
    }

    #[test]
    fn rejects_assigning_range_value_binding() {
        let error = check_error(
            r#"
func main() {
    values := [1]int{7}
    for index, value := range values {
        value = value + index
    }
}
"#,
        );

        assert!(error
            .message
            .contains("cannot assign to immutable binding `value`"));
    }

    #[test]
    fn match_payload_shadow_move_does_not_move_outer_binding() {
        check_ok(
            r#"
func main() {
    value := "outer"
    maybe := Some("inner")
    match maybe {
        case Some(value) {
            consume(value)
        }
        case None {
            print(value)
        }
    }
    print(value)
}

func consume(value string) {
}
"#,
        );
    }

    #[test]
    fn allows_condition_for_body_to_shadow_condition_binding() {
        check_ok(
            r#"
func main() {
    mut active := true
    for active {
        active := "shadow"
        print(active)
        break
    }
    print(active)
}
"#,
        );
    }

    #[test]
    fn allows_method_name_to_match_receiver_type_name() {
        check_ok(
            r#"
type User struct {
    age int
}

func main() {
    user := User{age: 30}
    print(user.User())
}

func (con self User) User() int {
    return self.age
}
"#,
        );
    }

    #[test]
    fn rejects_direct_recursive_struct_type() {
        let error = check_error(
            r#"
type Node struct {
    next Node
}

func main() {}
"#,
        );
        assert!(error
            .message
            .contains("recursive type definition involving `Node`"));
    }

    #[test]
    fn rejects_indirect_recursive_struct_type() {
        let error = check_error(
            r#"
type A struct {
    b B
}

type B struct {
    a A
}

func main() {}
"#,
        );
        assert!(error
            .message
            .contains("recursive type definition involving `A`"));
    }

    #[test]
    fn rejects_wrapped_recursive_struct_type() {
        let error = check_error(
            r#"
type Node struct {
    next Option[Node]
}

func main() {}
"#,
        );
        assert!(error
            .message
            .contains("recursive type definition involving `Node`"));

        let error = check_error(
            r#"
type Bucket struct {
    values [1]Bucket
}

func main() {}
"#,
        );
        assert!(error
            .message
            .contains("recursive type definition involving `Bucket`"));
    }

    #[test]
    fn rejects_printing_unit_value() {
        let error = check_error(
            r#"
func main() {
    print(noop())
}

func noop() {}
"#,
        );
        assert!(error.message.contains("cannot print value of type `unit`"));
    }

    #[test]
    fn rejects_print_in_value_position() {
        let error = check_error(
            r#"
func main() {
    value := print(1)
    print(value)
}
"#,
        );
        assert!(error
            .message
            .contains("`print` is only supported as a statement"));

        let error = check_error(
            r#"
func main() {
    print(print(1))
}
"#,
        );
        assert!(error
            .message
            .contains("`print` is only supported as a statement"));

        let error = check_error(
            r#"
func value() int {
    return print(1)
}

func main() {
    print(value())
}
"#,
        );
        assert!(error
            .message
            .contains("`print` is only supported as a statement"));
    }

    #[test]
    fn rejects_print_argument_mode_marker() {
        let error = check_error(
            r#"
func main() {
    value := "mallang"
    print(con value)
}
"#,
        );
        assert!(error
            .message
            .contains("`print` arguments do not take `con` or `mut` mode markers"));
    }

    #[test]
    fn rejects_printing_fixed_size_array_value() {
        let error = check_error(
            r#"
func main() {
    values := [2]int{1, 2}
    print(values)
}
"#,
        );
        assert!(error
            .message
            .contains("cannot print value of type `[2]int`"));
    }

    #[test]
    fn rejects_printing_adt_with_non_printable_payload() {
        let error = check_error(
            r#"
func main() {
    print(makeValues())
}

func makeValues() Option[[1]int] {
    return Some([1]int{1})
}
"#,
        );
        assert!(error
            .message
            .contains("cannot print value of type `Option[[1]int]`"));

        let error = check_error(
            r#"
func main() {
    print(makeResult())
}

func makeResult() Result[int, [1]int] {
    return Err([1]int{1})
}
"#,
        );
        assert!(error
            .message
            .contains("cannot print value of type `Result[int, [1]int]`"));
    }

    #[test]
    fn rejects_printing_struct_with_non_printable_field() {
        let error = check_error(
            r#"
type Box struct {
    values [1]int
}

func main() {
    box := Box{values: [1]int{1}}
    print(box)
}
"#,
        );
        assert!(error.message.contains("cannot print value of type `Box`"));
    }

    #[test]
    fn allows_read_receiver_method_call() {
        check_ok(
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
        );
    }

    #[test]
    fn allows_mut_receiver_method_call() {
        check_ok(
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
        );
    }

    #[test]
    fn allows_array_element_read_receiver_method_call() {
        check_ok(
            r#"
type User struct {
    age int
}

func (con self User) age() int {
    return self.age
}

func main() {
    users := [1]User{User{age: 30}}
    print(users[0].age())
}
"#,
        );
    }

    #[test]
    fn allows_array_element_mut_receiver_method_call() {
        check_ok(
            r#"
type Counter struct {
    value int
}

func (mut self Counter) inc() {
    self.value = self.value + 1
}

func main() {
    mut counters := [1]Counter{Counter{value: 1}}
    counters[0].inc()
    show(con counters[0].value)
}

func show(con value int) {
    print(value)
}
"#,
        );
    }

    #[test]
    fn rejects_mut_receiver_method_call_on_immutable_binding() {
        let error = check_error(
            r#"
type Counter struct {
    value int
}

func (mut self Counter) inc() {
    self.value = self.value + 1
}

func main() {
    counter := Counter{value: 1}
    counter.inc()
}
"#,
        );
        assert!(error
            .message
            .contains("cannot mutably borrow immutable binding `counter`"));
    }

    #[test]
    fn rejects_array_element_mut_receiver_method_call_on_immutable_binding() {
        let error = check_error(
            r#"
type Counter struct {
    value int
}

func (mut self Counter) inc() {
    self.value = self.value + 1
}

func main() {
    counters := [1]Counter{Counter{value: 1}}
    counters[0].inc()
}
"#,
        );
        assert!(error
            .message
            .contains("cannot mutably borrow place of immutable binding `counters`"));
    }

    #[test]
    fn rejects_array_element_receiver_overlapping_argument_borrow() {
        let error = check_error(
            r#"
type Counter struct {
    value int
}

func (mut self Counter) touch(mut value int) {
    value = value + 1
}

func main() {
    mut counters := [1]Counter{Counter{value: 1}}
    counters[0].touch(mut counters[0].value)
}
"#,
        );
        assert!(error
            .message
            .contains("borrow of `counters[?].value` overlaps with an active borrow"));
    }

    #[test]
    fn allows_field_assignment_on_mutable_struct_binding() {
        check_ok(
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
        );
    }

    #[test]
    fn rejects_field_assignment_on_immutable_binding() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    user := User{age: 30}
    user.age = 31
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign field of immutable binding `user`"));
    }

    #[test]
    fn rejects_field_assignment_type_mismatch() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    mut user := User{age: 30}
    user.age = "old"
}
"#,
        );
        assert!(error
            .message
            .contains("field `age` assignment type mismatch"));
    }

    #[test]
    fn allows_nested_field_assignment_on_mutable_struct_binding() {
        check_ok(
            r#"
type Name struct {
    value string
}

type User struct {
    name Name
}

func main() {
    mut user := User{name: Name{value: "kim"}}
    user.name.value = "lee"
    print(user.name.value)
}
"#,
        );
    }

    #[test]
    fn allows_indexed_field_assignment_on_array_and_slice_elements() {
        check_ok(
            r#"
type Profile struct {
    name string
}

type User struct {
    profile Profile
    age int
}

func main() {
    mut arrayUsers := [1]User{User{profile: Profile{name: "kim"}, age: 30}}
    arrayUsers[0].profile.name = "lee"
    printName(con arrayUsers[0].profile.name)

    mut sliceUsers := []User{User{profile: Profile{name: "park"}, age: 20}}
    sliceUsers[0].age = 21
    sliceUsers[0].profile.name = "choi"
    printAge(con sliceUsers[0].age)
    printName(con sliceUsers[0].profile.name)
}

func printName(con name string) {
    print(name)
}

func printAge(con age int) {
    print(age)
}
"#,
        );
    }

    #[test]
    fn rejects_indexed_field_assignment_on_immutable_binding() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    users := []User{User{age: 20}}
    users[0].age = 21
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign field of immutable binding `users`"));
    }

    #[test]
    fn rejects_negative_slice_indexed_field_assignment_index() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    mut users := []User{User{age: 20}}
    users[-1].age = 21
}
"#,
        );
        assert!(error
            .message
            .contains("slice index -1 must be non-negative"));
    }

    #[test]
    fn rejects_nested_field_assignment_on_immutable_root_binding() {
        let error = check_error(
            r#"
type Name struct {
    value string
}

type User struct {
    name Name
}

func main() {
    user := User{name: Name{value: "kim"}}
    user.name.value = "lee"
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign field of immutable binding `user`"));
    }

    #[test]
    fn rejects_nested_field_assignment_through_non_struct_field() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    mut user := User{age: 30}
    user.age.value = 31
}
"#,
        );
        assert!(error
            .message
            .contains("field assignment requires a struct value"));
    }

    #[test]
    fn rejects_unknown_method_call() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    user := User{name: "kim"}
    print(user.missing())
}
"#,
        );
        assert!(error.message.contains("unknown method `missing`"));
    }

    #[test]
    fn owned_receiver_method_moves_value() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func (self User) consume() {
}

func main() {
    user := User{name: "kim"}
    user.consume()
    print(user.name)
}
"#,
        );
        assert!(error.message.contains("use of moved value `user`"));
    }

    #[test]
    fn rejects_missing_struct_literal_field() {
        let error = check_error(
            r#"
type User struct {
    name string
    age int
}

func main() {
    user := User{name: "kim"}
    print(user.age)
}
"#,
        );
        assert!(error.message.contains("missing field `age`"));
    }

    #[test]
    fn rejects_unknown_struct_field_access() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    user := User{name: "kim"}
    print(user.age)
}
"#,
        );
        assert!(error.message.contains("unknown field `age`"));
    }

    #[test]
    fn rejects_moving_non_copy_field_without_destructuring() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    user := User{name: "kim"}
    name := user.name
    print(name)
}
"#,
        );
        assert!(error
            .message
            .contains("moving non-copy field out of `user` is not supported"));
    }

    #[test]
    fn allows_if_statement() {
        check_ok(
            r#"
func main() {
    if true {
        print("yes")
    } else {
        print("no")
    }
}
"#,
        );
    }

    #[test]
    fn allows_for_statement_with_bool_condition() {
        check_ok(
            r#"
func main() {
    mut count := 0
    for count < 3 {
        count = count + 1
    }
    print(count)
}
"#,
        );
    }

    #[test]
    fn allows_for_clause_statement() {
        check_ok(
            r#"
func main() {
    mut total := 0
    for mut i := 0; i < 3; i = i + 1 {
        total = total + i
    }
    print(total)
}
"#,
        );
    }

    #[test]
    fn allows_initless_for_clause_statement() {
        check_ok(
            r#"
func main() {
    mut i := 0
    for ; i < 3; i = i + 1 {
        print(i)
    }
}
"#,
        );
    }

    #[test]
    fn allows_for_statement_without_condition() {
        check_ok(
            r#"
func main() {
    for {
        break
    }
}
"#,
        );
    }

    #[test]
    fn allows_for_clause_without_condition() {
        check_ok(
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
        );
    }

    #[test]
    fn allows_break_and_continue_inside_for_statement() {
        check_ok(
            r#"
func main() {
    mut count := 0
    for count < 5 {
        count = count + 1
        if count == 2 {
            continue
        }
        if count == 4 {
            break
        }
    }
}
"#,
        );
    }

    #[test]
    fn rejects_break_outside_loop() {
        let error = check_error("func main() { break }");
        assert!(error
            .message
            .contains("`break` can only be used inside a loop"));
    }

    #[test]
    fn rejects_continue_outside_loop() {
        let error = check_error("func main() { continue }");
        assert!(error
            .message
            .contains("`continue` can only be used inside a loop"));
    }

    #[test]
    fn rejects_non_bool_for_statement_condition() {
        let error = check_error(
            r#"
func main() {
    for 1 {
        print("bad")
    }
}
"#,
        );
        assert!(error
            .message
            .contains("for condition must have type `bool`"));
    }

    #[test]
    fn for_clause_init_binding_does_not_leak() {
        let error = check_error(
            r#"
func main() {
    for mut i := 0; i < 3; i = i + 1 {
        print(i)
    }
    print(i)
}
"#,
        );
        assert!(error.message.contains("unknown variable `i`"));
    }

    #[test]
    fn rejects_immutable_for_clause_post_assignment() {
        let error = check_error(
            r#"
func main() {
    for i := 0; i < 3; i = i + 1 {
        print(i)
    }
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign to immutable binding `i`"));
    }

    #[test]
    fn for_statement_body_locals_do_not_leak() {
        let error = check_error(
            r#"
func main() {
    for false {
        inner := 1
    }
    print(inner)
}
"#,
        );
        assert!(error.message.contains("unknown variable `inner`"));
    }

    #[test]
    fn rejects_for_statement_body_persistent_move() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    for false {
        consume(s)
    }
    print(s)
}

func consume(value string) {
}
"#,
        );
        assert!(error
            .message
            .contains("loop cannot move persistent value `s` in v0"));
    }

    #[test]
    fn rejects_for_condition_persistent_move() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    for consume(s) {
        break
    }
}

func consume(value string) bool {
    return true
}
"#,
        );
        assert!(error
            .message
            .contains("loop cannot move persistent value `s` in v0"));
    }

    #[test]
    fn rejects_for_post_persistent_move() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    mut out := ""
    for ; false; out = s {
    }
}
"#,
        );
        assert!(error
            .message
            .contains("loop cannot move persistent value `s` in v0"));
    }

    #[test]
    fn rejects_for_init_binding_persistent_move() {
        let error = check_error(
            r#"
func main() {
    for s := "hello"; true; {
        consume(s)
        break
    }
}

func consume(value string) {
}
"#,
        );
        assert!(error
            .message
            .contains("loop cannot move persistent value `s` in v0"));
    }

    #[test]
    fn rejects_range_body_persistent_move() {
        let error = check_error(
            r#"
func main() {
    values := [1]int{1}
    s := "hello"
    for _ := range values {
        consume(s)
    }
}

func consume(value string) {
}
"#,
        );
        assert!(error
            .message
            .contains("loop cannot move persistent value `s` in v0"));
    }

    #[test]
    fn allows_for_body_local_move() {
        check_ok(
            r#"
func main() {
    for true {
        s := "hello"
        consume(s)
        break
    }
}

func consume(value string) {
}
"#,
        );
    }

    #[test]
    fn rejects_non_bool_if_statement_condition() {
        let error = check_error(
            r#"
func main() {
    if 1 {
        print("bad")
    }
}
"#,
        );
        assert!(error.message.contains("if condition must have type `bool`"));
    }

    #[test]
    fn allows_logical_operators_on_bool_values() {
        check_ok(
            r#"
func main() {
    print(check(true, false, 7))
}

func check(left bool, right bool, score int) bool {
    return left || right && score > 5
}
"#,
        );
    }

    #[test]
    fn rejects_literal_division_by_zero() {
        let error = check_error("func main() { print(10 / 0) }");
        assert!(error.message.contains("division by zero"));
    }

    #[test]
    fn rejects_literal_remainder_by_zero() {
        let error = check_error("func main() { print(10 % 0) }");
        assert!(error.message.contains("division by zero"));
    }

    #[test]
    fn rejects_literal_addition_overflow() {
        let error = check_error("func main() { print(9223372036854775807 + 1) }");
        assert!(error.message.contains("integer overflow"));
    }

    #[test]
    fn rejects_literal_subtraction_overflow() {
        let error = check_error("func main() { print(-9223372036854775807 - 2) }");
        assert!(error.message.contains("integer overflow"));
    }

    #[test]
    fn rejects_literal_multiplication_overflow() {
        let error = check_error("func main() { print(3037000500 * 3037000500) }");
        assert!(error.message.contains("integer overflow"));
    }

    #[test]
    fn rejects_literal_negation_overflow() {
        let error = check_error("func main() { print(-(-9223372036854775807 - 1)) }");
        assert!(error.message.contains("integer overflow"));
    }

    #[test]
    fn rejects_literal_division_overflow() {
        let error = check_error("func main() { print((-9223372036854775807 - 1) / -1) }");
        assert!(error.message.contains("integer overflow"));
    }

    #[test]
    fn rejects_literal_remainder_overflow() {
        let error = check_error("func main() { print((-9223372036854775807 - 1) % -1) }");
        assert!(error.message.contains("integer overflow"));
    }

    #[test]
    fn allows_bool_unary_not() {
        check_ok(
            r#"
func main() {
    print(check(false, true))
}

func check(left bool, right bool) bool {
    return !left && right
}
"#,
        );
    }

    #[test]
    fn allows_pipeline_expression_call_sugar() {
        check_ok(
            r#"
func main() {
    print(7 |> double() |> add(1))
}

func double(value int) int {
    return value * 2
}

func add(value int, amount int) int {
    return value + amount
}
"#,
        );
    }

    #[test]
    fn rejects_logical_operators_on_non_bool_values() {
        let error = check_error("func main() { print(1 && 2) }");
        assert!(error
            .message
            .contains("logical operators require `bool` operands"));
    }

    #[test]
    fn rejects_unary_not_on_non_bool_values() {
        let error = check_error("func main() { print(!1) }");
        assert!(error.message.contains("`!` expects a `bool` operand"));
    }

    #[test]
    fn if_statement_branch_locals_do_not_leak() {
        let error = check_error(
            r#"
func main() {
    if true {
        inner := 1
    }
    print(inner)
}
"#,
        );
        assert!(error.message.contains("unknown variable `inner`"));
    }

    #[test]
    fn if_statement_merges_branch_moves() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    if true {
        consume(s)
    }
    print(s)
}

func consume(value string) {
}
"#,
        );
        assert!(error.message.contains("use of moved value `s`"));
    }

    #[test]
    fn rejects_argument_type_mismatch() {
        let error = check_error(
            r#"
func main() {
    print(add("x", 1))
}

func add(a int, b int) int {
    return a + b
}
"#,
        );
        assert!(error.message.contains("argument type mismatch"));
    }

    #[test]
    fn allows_option_constructors_with_expected_context() {
        check_ok(
            r#"
func find(flag bool) Option[int] {
    return if flag { Some(1) } else { None }
}

func main() {}
"#,
        );
    }

    #[test]
    fn allows_result_constructors_with_expected_context() {
        check_ok(
            r#"
func read(flag bool) Result[int, string] {
    return if flag { Ok(1) } else { Err("bad") }
}

func main() {}
"#,
        );
    }

    #[test]
    fn allows_none_as_function_argument_with_expected_option_type() {
        check_ok(
            r#"
func main() {
    accept(None)
}

func accept(value Option[int]) {
}
"#,
        );
    }

    #[test]
    fn rejects_none_without_expected_option_context() {
        let error = check_error("func main() { value := None }");
        assert!(error
            .message
            .contains("`None` requires expected `Option[T]` context"));
    }

    #[test]
    fn rejects_ok_without_expected_result_context() {
        let error = check_error("func main() { value := Ok(1) }");
        assert!(error
            .message
            .contains("`Ok` requires expected `Result[T, E]` context"));
    }

    #[test]
    fn rejects_option_constructor_payload_mismatch() {
        let error = check_error(
            r#"
func find() Option[int] {
    return Some("nope")
}

func main() {}
"#,
        );
        assert!(error
            .message
            .contains("return type mismatch: expected `Option[int]`, got `Option[string]`"));
    }

    #[test]
    fn ownership_moves_option_payloads() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    wrapped := Some(s)
    print(s)
    consume(wrapped)
}

func consume(value Option[string]) {
}
"#,
        );
        assert!(error.message.contains("use of moved value `s`"));
    }

    #[test]
    fn allows_option_match_expression() {
        check_ok(
            r#"
func main() {
    value := Some(1)
    out := match value {
        case Some(inner) { inner + 1 }
        case None { 0 }
    }
    print(out)
}
"#,
        );
    }

    #[test]
    fn allows_result_match_expression() {
        check_ok(
            r#"
func main() {
    result := read(false)
    code := match result {
        case Ok(value) { value }
        case Err(message) { 0 }
    }
    print(code)
}

func read(flag bool) Result[int, string] {
    return if flag { Ok(1) } else { Err("bad") }
}
"#,
        );
    }

    #[test]
    fn allows_option_match_statement_with_block_arms() {
        check_ok(
            r#"
func main() {
    value := Some(1)
    match value {
        case Some(inner) {
            print(inner)
        }
        case None {
            print(0)
        }
    }
}
"#,
        );
    }

    #[test]
    fn allows_result_match_statement_with_block_arms() {
        check_ok(
            r#"
func main() {
    result := read(false)
    match result {
        case Ok(value) {
            print(value)
        }
        case Err(message) {
            print(message)
        }
    }
}

func read(flag bool) Result[int, string] {
    return if flag { Ok(1) } else { Err("bad") }
}
"#,
        );
    }

    #[test]
    fn allows_match_statement_return_completeness() {
        check_ok(
            r#"
func main() {
    print(unwrap(Some(1)))
}

func unwrap(value Option[int]) int {
    match value {
        case Some(inner) {
            return inner
        }
        case None {
            return 0
        }
    }
}
"#,
        );
    }

    #[test]
    fn rejects_non_exhaustive_option_match() {
        let error = check_error(
            r#"
func main() {
    value := Some(1)
    out := match value {
        case Some(inner) { inner }
    }
    print(out)
}
"#,
        );
        assert!(error
            .message
            .contains("Option match must include `Some(name)` and `None` arms"));
    }

    #[test]
    fn rejects_non_exhaustive_option_match_statement() {
        let error = check_error(
            r#"
func main() {
    value := Some(1)
    match value {
        case Some(inner) {
            print(inner)
        }
    }
}
"#,
        );
        assert!(error
            .message
            .contains("Option match must include `Some(name)` and `None` arms"));
    }

    #[test]
    fn rejects_match_statement_return_when_an_arm_does_not_return() {
        let error = check_error(
            r#"
func main() {
    print(unwrap(Some(1)))
}

func unwrap(value Option[int]) int {
    match value {
        case Some(inner) {
            return inner
        }
        case None {
            print(0)
        }
    }
}
"#,
        );
        assert!(error
            .message
            .contains("function `unwrap` must return `int`"));
    }

    #[test]
    fn rejects_result_pattern_in_option_match() {
        let error = check_error(
            r#"
func main() {
    value := Some(1)
    out := match value {
        case Ok(inner) { inner }
        case None { 0 }
    }
    print(out)
}
"#,
        );
        assert!(error
            .message
            .contains("Option match patterns must be `Some(name)` and `None`"));
    }

    #[test]
    fn rejects_match_arm_type_mismatch() {
        let error = check_error(
            r#"
func main() {
    value := Some(1)
    out := match value {
        case Some(inner) { inner }
        case None { false }
    }
    print(out)
}
"#,
        );
        assert!(error.message.contains("match arms must have the same type"));
    }

    #[test]
    fn ownership_moves_match_scrutinee() {
        let error = check_error(
            r#"
func main() {
    value := Some("hello")
    out := match value {
        case Some(inner) { inner }
        case None { "fallback" }
    }
    print(value)
    print(out)
}
"#,
        );
        assert!(error.message.contains("use of moved value `value`"));
    }

    #[test]
    fn reports_v04_declaration_and_specialization_errors() {
        let enum_error = check_error("type Maybe[T] enum { None Some(T) }\nfunc main() {}\n");
        assert_eq!(
            enum_error.message,
            "user-defined enum declarations require v0.4 semantic lowering"
        );

        check_ok("type Box[T] struct { value T }\nfunc main() {}\n");
        check_ok("func Identity[T](value T) T { return value }\nfunc main() {}\n");

        let duplicate = check_error("type Box[T, T] struct { value T }\nfunc main() {}\n");
        assert!(duplicate.message.contains("duplicate type parameter `T`"));

        let missing = check_error(
            "func Identity[T](value T) T { return value }\nfunc main() { value := Identity }\n",
        );
        assert!(missing
            .message
            .contains("generic function `Identity` requires explicit type arguments"));
    }

    #[test]
    fn validates_unused_generic_bodies_with_non_copy_symbolic_types() {
        let print_error = check_error("func Debug[T](value T) { print(value) }\nfunc main() {}\n");
        assert!(print_error
            .message
            .contains("cannot print value of type `T`"));
        assert!(!print_error.message.contains("__mlg_symbolic"));

        let nested_error = check_error(
            "type Box[T] struct { value T }\nfunc Debug[T](value Box[T]) { print(value) }\nfunc main() {}\n",
        );
        assert!(nested_error
            .message
            .contains("cannot print value of type `Box[T]`"));
        assert!(!nested_error.message.contains("__mlg_spec"));

        let arithmetic_error =
            check_error("func Add[T](left T, right T) T { return left + right }\nfunc main() {}\n");
        assert!(arithmetic_error
            .message
            .contains("arithmetic operators currently require `int` operands"));
    }

    #[test]
    fn allows_generic_receiver_specialization() {
        check_ok(
            r#"
type Box[T] struct { value T }
func (mut box Box[T]) replace(value T) { box.value = value }
func main() {
    mut box := Box[string]{value: "before"}
    box.replace("after")
    print(box.value)
}
"#,
        );
    }

    #[test]
    fn rejects_generic_args_on_primitive_types() {
        let error = check_error(
            r#"
func bad(value int[string]) {
}

func main() {}
"#,
        );
        assert!(error
            .message
            .contains("primitive type `int` does not take type arguments"));
    }

    #[test]
    fn allows_fixed_size_array_types_and_literals() {
        check_ok(
            r#"
func consume(values [3]int) {
}

func main() {
    values := [3]int{1, 2, 3}
    consume(values)
}
"#,
        );
    }

    #[test]
    fn allows_slice_literals_len_and_copy_index() {
        check_ok(
            r#"
func main() {
    values := []int{1, 2, 3}
    first := values[0]
    count := len(values)
    print(first + count)
}
"#,
        );
    }

    #[test]
    fn allows_append_to_reassign_owned_slice() {
        check_ok(
            r#"
func main() {
    mut values := []int{1, 2}
    values = append(values, 3)
    print(values[2] + len(values))
}
"#,
        );
    }

    #[test]
    fn allows_append_to_reassign_same_slice_field() {
        check_ok(
            r#"
type Bag struct {
    values []int
}

type Shelf struct {
    bag Bag
}

func main() {
    mut bag := Bag{values: []int{1, 2}}
    bag.values = append(bag.values, 3)
    print(bag.values[2] + len(bag.values))

    mut shelf := Shelf{bag: Bag{values: []int{4}}}
    shelf.bag.values = append(shelf.bag.values, 5)
    print(shelf.bag.values[1] + len(shelf.bag.values))
}
"#,
        );
    }

    #[test]
    fn allows_append_to_reassign_same_indexed_slice_field() {
        check_ok(
            r#"
type Bag struct {
    values []int
}

type Store struct {
    bags []Bag
}

func main() {
    mut store := Store{bags: []Bag{Bag{values: []int{1}}, Bag{values: []int{2, 3}}}}
    i := 1
    store.bags[i].values = append(store.bags[i].values, 4)
    print(len(store.bags[i].values))
    print(store.bags[i].values[2])
}
"#,
        );
    }

    #[test]
    fn append_consumes_source_slice() {
        let error = check_error(
            r#"
func main() {
    values := []int{1}
    grown := append(values, 2)
    print(len(grown))
    print(len(values))
}
"#,
        );
        assert!(error.message.contains("use of moved value `values`"));
    }

    #[test]
    fn allows_append_from_slice_field_by_taking_source_field() {
        check_ok(
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
        );
    }

    #[test]
    fn allows_owned_slice_field_take_expression() {
        check_ok(
            r#"
type Bag struct {
    values []int
}

func main() {
    bag := Bag{values: []int{1, 2}}
    taken := bag.values
    print(len(taken))
    print(len(bag.values))
    consume(bag.values)
    print(len(bag.values))
}

func consume(values []int) {
    print(len(values))
}
"#,
        );
    }

    #[test]
    fn rejects_moving_non_slice_cleanup_field_without_partial_move_semantics() {
        let error = check_error(
            r#"
type Profile struct {
    tags []int
}

type User struct {
    profile Profile
}

func main() {
    user := User{profile: Profile{tags: []int{1}}}
    profile := user.profile
    print(len(profile.tags))
}
"#,
        );
        assert!(error
            .message
            .contains("moving non-copy field out of `user` is not supported"));
    }

    #[test]
    fn allows_indexed_slice_field_append_take_with_different_target_index() {
        check_ok(
            r#"
type Bag struct {
    values []int
}

type Store struct {
    bags []Bag
}

func main() {
    mut store := Store{bags: []Bag{Bag{values: []int{1}}, Bag{values: []int{2}}}}
    i := 0
    store.bags[i].values = append(store.bags[i + 1].values, 3)
    print(len(store.bags[i].values))
    print(len(store.bags[i + 1].values))
}
"#,
        );
    }

    #[test]
    fn allows_indexed_slice_field_append_take_with_call_index() {
        check_ok(
            r#"
type Bag struct {
    values []int
}

type Store struct {
    bags []Bag
}

func pick() int {
    return 0
}

func main() {
    mut store := Store{bags: []Bag{Bag{values: []int{1}}}}
    store.bags[pick()].values = append(store.bags[pick()].values, 2)
    print(len(store.bags[0].values))
}
"#,
        );
    }

    #[test]
    fn rejects_discarded_cleanup_expression_statement() {
        let error = check_error(
            r#"
func main() {
    values := []int{1}
    append(values, 2)
}
"#,
        );
        assert!(error
            .message
            .contains("expression statements cannot discard cleanup value of type `[]int`"));
    }

    #[test]
    fn rejects_append_with_mode_markers() {
        let error = check_error(
            r#"
func main() {
    values := []int{1}
    grown := append(con values, 2)
    print(len(grown))
}
"#,
        );
        assert!(error
            .message
            .contains("`append` arguments do not take `con` or `mut` mode markers"));
    }

    #[test]
    fn rejects_append_item_type_mismatch() {
        let error = check_error(
            r#"
func main() {
    values := []int{1}
    grown := append(values, "kim")
    print(len(grown))
}
"#,
        );
        assert!(error
            .message
            .contains("`append` item type mismatch: expected `int`, got `string`"));
    }

    #[test]
    fn allows_slice_function_params_returns_and_nested_payloads() {
        check_ok(
            r#"
func first(values []int) int {
    return values[0]
}

func wrap(values []int) Option[[]int] {
    return Some(values)
}

func main() {
    values := []int{1, 2}
    print(first(values))
}
"#,
        );
    }

    #[test]
    fn allows_slice_fields_with_struct_cleanup() {
        check_ok(
            r#"
type Bag struct {
    values []int
}

func main() {
    bag := Bag{values: []int{1, 2}}
}
"#,
        );
    }

    #[test]
    fn allows_local_rooted_slice_field_read_sources() {
        check_ok(
            r#"
type Bag struct {
    values []int
}

func main() {
    mut bag := Bag{values: []int{1, 2, 3}}
    count := len(bag.values)
    first := bag.values[0]
    show(con bag.values[1])
    bump(mut bag.values[2])

    mut total := 0
    for _, value := range bag.values {
        total = total + value
    }
    print(count + first + total)
}

func show(con value int) {
    print(value)
}

func bump(mut value int) {
    value = value + 10
}
"#,
        );
    }

    #[test]
    fn rejects_slice_index_for_non_copy_elements() {
        let error = check_error(
            r#"
func main() {
    values := []string{"kim"}
    name := values[0]
    print(name)
}
"#,
        );
        assert!(error
            .message
            .contains("slice indexing requires a Copy element type"));
    }

    #[test]
    fn allows_borrowed_slice_index_field_access_for_non_copy_elements() {
        check_ok(
            r#"
type User struct {
    name string
    age int
}

func main() {
    users := []User{User{name: "kim", age: 30}}
    print(users[0])
    print(users[0].name)
    age := users[0].age
    print(age)
}
"#,
        );
    }

    #[test]
    fn rejects_inline_slice_len_until_temporary_cleanup_exists() {
        let error = check_error(
            r#"
func main() {
    count := len([]int{1, 2})
    print(count)
}
"#,
        );
        assert!(error
            .message
            .contains("`len` on slices requires a local-rooted slice source"));
    }

    #[test]
    fn rejects_inline_slice_index_until_temporary_cleanup_exists() {
        let error = check_error(
            r#"
func main() {
    value := []int{1, 2}[0]
    print(value)
}
"#,
        );
        assert!(error
            .message
            .contains("slice indexing requires a local-rooted slice source"));
    }

    #[test]
    fn classifies_internal_slice_types_as_cleanup_resources() {
        assert!(Type::Slice(Box::new(Type::Int)).needs_cleanup());
        assert!(Type::Struct("Bag".to_string()).needs_cleanup());
        assert!(Type::Option(Box::new(Type::Slice(Box::new(Type::Int)))).needs_cleanup());
        assert!(Type::Array {
            len: 2,
            element: Box::new(Type::Slice(Box::new(Type::String))),
        }
        .needs_cleanup());
        assert!(!Type::Array {
            len: 2,
            element: Box::new(Type::Int),
        }
        .needs_cleanup());
    }

    #[test]
    fn rejects_fixed_size_array_literal_length_mismatch() {
        let error = check_error(
            r#"
func main() {
    values := [3]int{1, 2}
}
"#,
        );
        assert!(error
            .message
            .contains("array literal length mismatch: expected 3 elements, got 2"));
    }

    #[test]
    fn rejects_fixed_size_array_literal_element_type_mismatch() {
        let error = check_error(
            r#"
func main() {
    values := [2]int{1, "bad"}
}
"#,
        );
        assert!(error
            .message
            .contains("array literal element 1 type mismatch: expected `int`, got `string`"));
    }

    #[test]
    fn rejects_fixed_size_array_literal_expected_type_mismatch() {
        let error = check_error(
            r#"
func consume(values [3]int) {
}

func main() {
    consume([2]int{1, 2})
}
"#,
        );
        assert!(error
            .message
            .contains("array literal type mismatch: expected `[3]int`, got `[2]int`"));
    }

    #[test]
    fn treats_fixed_size_arrays_as_move_only_values() {
        let error = check_error(
            r#"
func consume(values [2]int) {
}

func main() {
    values := [2]int{1, 2}
    consume(values)
    consume(values)
}
"#,
        );
        assert!(error.message.contains("use of moved value `values`"));
    }

    #[test]
    fn allows_array_range_loop_and_source_reuse() {
        check_ok(
            r#"
func consume(values [3]int) {
}

func main() {
    values := [3]int{1, 2, 3}
    mut total := 0
    for i, value := range values {
        total = total + i + value
    }
    consume(values)
}
"#,
        );
    }

    #[test]
    fn allows_array_range_blank_identifiers() {
        check_ok(
            r#"
type User struct {
    age int
}

func main() {
    values := [3]int{1, 2, 3}
    for _, value := range values {
        print(value)
    }

    users := [1]User{User{age: 1}}
    for i, _ := range users {
        print(i)
    }
}
"#,
        );
    }

    #[test]
    fn allows_one_variable_array_range() {
        check_ok(
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
        );
    }

    #[test]
    fn rejects_reading_range_blank_identifier() {
        let error = check_error(
            r#"
func main() {
    values := [1]int{1}
    for _, value := range values {
        print(_)
        print(value)
    }
}
"#,
        );
        assert!(error.message.contains("unknown variable `_`"));
    }

    #[test]
    fn allows_fixed_size_array_indexing_and_len_without_move() {
        check_ok(
            r#"
func consume(values [3]int) {
}

func main() {
    values := [3]int{1, 2, 3}
    first := values[0]
    count := len(values)
    print(first + count)
    consume(values)
}
"#,
        );
    }

    #[test]
    fn allows_fixed_size_array_element_assignment() {
        check_ok(
            r#"
func consume(values [3]int) {
}

func main() {
    mut values := [3]int{1, 2, 3}
    index := 1
    values[index] = 5
    print(values[index])
    consume(values)
}
"#,
        );
    }

    #[test]
    fn allows_fixed_size_array_element_assignment_in_for_post() {
        check_ok(
            r#"
func main() {
    mut values := [3]int{0, 0, 0}
    mut slot := 0
    mut i := 0
    for ; i < 3; values[slot] = i {
        slot = i
        i = i + 1
    }
    print(values[0])
}
"#,
        );
    }

    #[test]
    fn rejects_array_element_assignment_on_immutable_binding() {
        let error = check_error(
            r#"
func main() {
    values := [1]int{1}
    values[0] = 2
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign through immutable indexed binding `values`"));
    }

    #[test]
    fn allows_array_element_assignment_for_non_copy_elements() {
        check_ok(
            r#"
type User struct {
    age int
}

func main() {
    mut users := [1]User{User{age: 1}}
    users[0] = User{age: 2}
}
"#,
        );
    }

    #[test]
    fn array_element_assignment_moves_non_copy_rhs() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    mut users := [1]User{User{age: 1}}
    replacement := User{age: 2}
    users[0] = replacement
    print(replacement.age)
}
"#,
        );
        assert!(error.message.contains("use of moved value `replacement`"));
    }

    #[test]
    fn rejects_array_element_assignment_literal_index_out_of_bounds() {
        let error = check_error(
            r#"
func main() {
    mut values := [1]int{1}
    values[1] = 2
}
"#,
        );
        assert!(error
            .message
            .contains("array index 1 is out of bounds for length 1"));
    }

    #[test]
    fn rejects_indexing_non_array_values() {
        let error = check_error(
            r#"
func main() {
    value := 1
    first := value[0]
}
"#,
        );
        assert!(error
            .message
            .contains("indexing requires a fixed-size array or slice, got `int`"));
    }

    #[test]
    fn rejects_non_int_array_index() {
        let error = check_error(
            r#"
func main() {
    values := [1]int{1}
    first := values["bad"]
}
"#,
        );
        assert!(error
            .message
            .contains("array index must have type `int`, got `string`"));
    }

    #[test]
    fn rejects_literal_array_index_out_of_bounds() {
        let error = check_error(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    first := values[3]
}
"#,
        );
        assert!(error
            .message
            .contains("array index 3 is out of bounds for length 3"));
    }

    #[test]
    fn rejects_negative_literal_array_index_out_of_bounds() {
        let error = check_error(
            r#"
func main() {
    values := [3]int{1, 2, 3}
    first := values[-1]
}
"#,
        );
        assert!(error
            .message
            .contains("array index -1 must be non-negative"));
    }

    #[test]
    fn rejects_indexing_non_copy_array_elements() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    users := [1]User{User{age: 1}}
    user := users[0]
}
"#,
        );
        assert!(error
            .message
            .contains("array indexing requires a Copy element type in v0, got `User`"));
    }

    #[test]
    fn allows_borrowed_array_index_field_access_for_non_copy_elements() {
        check_ok(
            r#"
type Profile struct {
    label string
    score int
}

type User struct {
    name string
    age int
    profile Profile
}

func main() {
    users := [2]User{
        User{name: "kim", age: 30, profile: Profile{label: "a", score: 7}},
        User{name: "lee", age: 20, profile: Profile{label: "b", score: 9}},
    }
    print(users[0])
    print(users[1].name)
    age := users[1].age
    print(age)
    print(users[0].profile.label)
    score := users[0].profile.score
    print(score)
}
"#,
        );
    }

    #[test]
    fn rejects_moving_non_copy_field_out_of_indexed_element() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    users := [1]User{User{name: "kim"}}
    name := users[0].name
    print(name)
}
"#,
        );
        assert!(error
            .message
            .contains("moving non-copy field out of indexed element"));
    }

    #[test]
    fn rejects_len_on_non_array_values() {
        let error = check_error(
            r#"
func main() {
    count := len(1)
}
"#,
        );
        assert!(error
            .message
            .contains("`len` expects a fixed-size array or slice, got `int`"));
    }

    #[test]
    fn allows_slice_range_loop_and_source_reuse() {
        check_ok(
            r#"
func consume(values []int) {
}

func main() {
    values := []int{1, 2, 3}
    mut total := 0
    for i, value := range values {
        total = total + i + value
    }
    print(total)
    consume(values)
}
"#,
        );
    }

    #[test]
    fn allows_index_only_slice_range_for_non_copy_elements() {
        check_ok(
            r#"
func main() {
    values := []string{"kim", "lee"}
    for i := range values {
        print(i)
    }
}
"#,
        );
    }

    #[test]
    fn rejects_inline_slice_range_until_temporary_cleanup_exists() {
        let error = check_error(
            r#"
func main() {
    for i, value := range []int{1, 2} {
        print(i + value)
    }
}
"#,
        );
        assert!(error
            .message
            .contains("range over slices requires a local-rooted slice source"));
    }

    #[test]
    fn rejects_assigning_active_range_source() {
        let error = check_error(
            r#"
func main() {
    mut values := []int{1, 2}
    for i := range values {
        values = append(values, i)
    }
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign to active range source `values`"));
    }

    #[test]
    fn rejects_slice_range_value_binding_for_non_copy_elements() {
        let error = check_error(
            r#"
func main() {
    values := []string{"kim"}
    for i, value := range values {
        print(i)
        print(value)
    }
}
"#,
        );
        assert!(error
            .message
            .contains("range value binding requires a Copy element type in v0, got `string`"));
    }

    #[test]
    fn rejects_range_over_non_array_source() {
        let error = check_error(
            r#"
func main() {
    for i, value := range 1 {
        print(i)
    }
}
"#,
        );
        assert!(error
            .message
            .contains("range source must be a fixed-size array or slice, got `int`"));
    }

    #[test]
    fn rejects_range_value_binding_for_non_copy_elements() {
        let error = check_error(
            r#"
type User struct {
    age int
}

func main() {
    users := [1]User{User{age: 1}}
    for i, user := range users {
        print(i)
    }
}
"#,
        );
        assert!(error
            .message
            .contains("range value binding requires a Copy element type in v0, got `User`"));
    }

    #[test]
    fn rejects_immutable_assignment() {
        let error = check_error("func main() { x := 1 x = 2 }");
        assert!(error
            .message
            .contains("cannot assign to immutable binding `x`"));
    }

    #[test]
    fn allows_mutable_assignment() {
        check_ok("func main() { mut x := 1 x = 2 print(x) }");
    }

    #[test]
    fn allows_string_equality_without_move() {
        check_ok(
            r#"
func main() {
    word := "mallang"
    if word == "mallang" {
        print(word)
    }
    if word != "rust" {
        print(word)
    }
}
"#,
        );
    }

    #[test]
    fn allows_if_expression_value() {
        check_ok(
            r#"
func main() {
    score := 70
    label := if score >= 60 { "pass" } else { "fail" }
    print(label)
}
"#,
        );
    }

    #[test]
    fn rejects_if_condition_that_is_not_bool() {
        let error = check_error("func main() { x := if 1 { 1 } else { 2 } }");
        assert!(error.message.contains("if condition must have type `bool`"));
    }

    #[test]
    fn rejects_if_branch_type_mismatch() {
        let error = check_error("func main() { x := if true { 1 } else { false } }");
        assert!(error
            .message
            .contains("if branches must have the same type"));
    }

    #[test]
    fn ownership_merges_moves_from_if_branches() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    flag := true
    picked := if flag { s } else { "fallback" }
    print(s)
    print(picked)
}
"#,
        );
        assert!(error.message.contains("use of moved value `s`"));
    }

    #[test]
    fn ownership_rejects_use_after_move_for_string() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    consume(s)
    print(s)
}

func consume(s string) {
    print(s)
}
"#,
        );
        assert!(error.message.contains("use of moved value `s`"));
    }

    #[test]
    fn ownership_allows_copy_reuse_for_int() {
        check_ok(
            r#"
func main() {
    x := 1
    printInt(x)
    print(x)
}

func printInt(x int) {
    print(x)
}
"#,
        );
    }

    #[test]
    fn ownership_allows_con_borrow_without_move() {
        check_ok(
            r#"
func main() {
    s := "hello"
    show(con s)
    show(con s)
}

func show(con s string) {
    print(s)
}
"#,
        );
    }

    #[test]
    fn ownership_rejects_missing_con_call_mode() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    show(s)
}

func show(con s string) {
    print(s)
}
"#,
        );
        assert!(error.message.contains("expects `con` argument"));
    }

    #[test]
    fn ownership_rejects_mut_borrow_of_immutable_binding() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    touch(mut s)
}

func touch(mut s string) {
    print(s)
}
"#,
        );
        assert!(error
            .message
            .contains("cannot mutably borrow immutable binding `s`"));
    }

    #[test]
    fn ownership_allows_mut_borrow_of_mutable_binding() {
        check_ok(
            r#"
func main() {
    mut s := "hello"
    touch(mut s)
    print(s)
}

func touch(mut s string) {
    print(s)
}
"#,
        );
    }

    #[test]
    fn ownership_allows_array_element_borrow_arguments() {
        check_ok(
            r#"
type User struct {
    name string
    age int
}

func main() {
    mut users := [2]User{User{name: "kim", age: 30}, User{name: "lee", age: 20}}
    printName(con users[0].name)
    rename(mut users[1].name)
}

func printName(con name string) {
    print(name)
}

func rename(mut name string) {
    name = "park"
}
"#,
        );
    }

    #[test]
    fn ownership_rejects_mut_array_element_borrow_of_immutable_binding() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    users := [1]User{User{name: "kim"}}
    rename(mut users[0].name)
}

func rename(mut name string) {
    name = "lee"
}
"#,
        );
        assert!(error
            .message
            .contains("cannot mutably borrow place of immutable binding `users`"));
    }

    #[test]
    fn ownership_rejects_overlapping_array_element_borrows_in_one_call() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    mut users := [2]User{User{name: "kim"}, User{name: "lee"}}
    touch(mut users[0].name, mut users[1].name)
}

func touch(mut left string, mut right string) {
}
"#,
        );
        assert!(error
            .message
            .contains("borrow of `users[?].name` overlaps with an active borrow"));
    }

    #[test]
    fn ownership_allows_slice_element_borrow_arguments() {
        check_ok(
            r#"
type User struct {
    name string
    age int
}

func main() {
    mut users := []User{User{name: "kim", age: 30}, User{name: "lee", age: 20}}
    show(con users[0])
    rename(mut users[1].name)
}

func show(con user User) {
    print(user.age)
}

func rename(mut name string) {
    name = "park"
}
"#,
        );
    }

    #[test]
    fn ownership_rejects_mut_slice_element_borrow_of_immutable_binding() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    users := []User{User{name: "kim"}}
    rename(mut users[0].name)
}

func rename(mut name string) {
    name = "lee"
}
"#,
        );
        assert!(error
            .message
            .contains("cannot mutably borrow place of immutable binding `users`"));
    }

    #[test]
    fn ownership_rejects_overlapping_slice_element_borrows_in_one_call() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    mut users := []User{User{name: "kim"}, User{name: "lee"}}
    touch(mut users[0].name, mut users[1].name)
}

func touch(mut left string, mut right string) {
}
"#,
        );
        assert!(error
            .message
            .contains("borrow of `users[?].name` overlaps with an active borrow"));
    }

    #[test]
    fn ownership_rejects_negative_slice_element_borrow_index() {
        let error = check_error(
            r#"
func main() {
    values := []int{1}
    show(con values[-1])
}

func show(con value int) {
    print(value)
}
"#,
        );
        assert!(error
            .message
            .contains("slice index -1 must be non-negative"));
    }

    #[test]
    fn allows_slice_element_assignment_for_copy_and_non_copy_elements() {
        check_ok(
            r#"
type User struct {
    name string
    age int
}

func main() {
    mut values := []int{1, 2}
    values[1] = 5
    print(values[1])

    mut users := []User{User{name: "kim", age: 30}}
    users[0] = User{name: "lee", age: 20}
    show(con users[0])
}

func show(con user User) {
    print(user.age)
}
"#,
        );
    }

    #[test]
    fn allows_slice_element_assignment_in_for_body() {
        check_ok(
            r#"
func main() {
    mut values := []int{0, 0, 0}
    mut i := 0
    for i < 3 {
        values[i] = i
        i = i + 1
    }
}
"#,
        );
    }

    #[test]
    fn allows_local_rooted_slice_element_assignment() {
        check_ok(
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
    print(bag.values[1])

    mut store := Store{bags: []Bag{Bag{values: []int{3}}, Bag{values: []int{4}}}}
    store.bags[0] = Bag{values: []int{7, 8}}
    print(len(store.bags[0].values))
}
"#,
        );
    }

    #[test]
    fn rejects_slice_element_assignment_on_immutable_binding() {
        let error = check_error(
            r#"
func main() {
    values := []int{1, 2}
    values[0] = 5
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign through immutable indexed binding `values`"));
    }

    #[test]
    fn rejects_local_rooted_slice_element_assignment_on_immutable_root() {
        let error = check_error(
            r#"
type Bag struct {
    values []int
}

func main() {
    bag := Bag{values: []int{1, 2}}
    bag.values[0] = 5
}
"#,
        );
        assert!(error
            .message
            .contains("cannot assign through immutable indexed place of binding `bag`"));
    }

    #[test]
    fn rejects_negative_slice_element_assignment_index() {
        let error = check_error(
            r#"
func main() {
    mut values := []int{1}
    values[-1] = 2
}
"#,
        );
        assert!(error
            .message
            .contains("slice index -1 must be non-negative"));
    }

    #[test]
    fn rejects_slice_element_assignment_type_mismatch() {
        let error = check_error(
            r#"
func main() {
    mut values := []int{1}
    values[0] = "bad"
}
"#,
        );
        assert!(error
            .message
            .contains("indexed assignment type mismatch: expected `int`, got `string`"));
    }

    #[test]
    fn ownership_rejects_returning_non_copy_borrowed_param() {
        let error = check_error(
            r#"
func main() {
    s := "hello"
    print(leak(con s))
}

func leak(con s string) string {
    return s
}
"#,
        );
        assert!(error.message.contains("cannot move borrowed value `s`"));
    }

    #[test]
    fn ownership_rejects_storing_non_copy_borrowed_param() {
        let error = check_error(
            r#"
func main() {
    mut s := "hello"
    leak(mut s)
}

func leak(mut s string) {
    alias := s
    print(alias)
}
"#,
        );
        assert!(error.message.contains("cannot move borrowed value `s`"));
    }

    #[test]
    fn ownership_rejects_passing_non_copy_borrowed_param_as_owned() {
        let error = check_error(
            r#"
func main() {
    mut s := "hello"
    leak(mut s)
}

func leak(mut s string) {
    consume(s)
}

func consume(s string) {
    print(s)
}
"#,
        );
        assert!(error.message.contains("cannot move borrowed value `s`"));
    }

    #[test]
    fn ownership_allows_returning_copy_borrowed_param() {
        check_ok(
            r#"
func main() {
    x := 1
    print(id(con x))
}

func id(con x int) int {
    return x
}
"#,
        );
    }

    #[test]
    fn ownership_allows_field_level_read_borrow_argument() {
        check_ok(
            r#"
type User struct {
    name string
    age int
}

func main() {
    user := User{name: "kim", age: 30}
    showName(con user.name)
    print(user.age)
}

func showName(con name string) {
    print(name)
}
"#,
        );
    }

    #[test]
    fn ownership_allows_field_level_mut_borrow_argument_on_mutable_binding() {
        check_ok(
            r#"
type User struct {
    name string
}

func main() {
    mut user := User{name: "kim"}
    touchName(mut user.name)
}

func touchName(mut name string) {
    print(name)
}
"#,
        );
    }

    #[test]
    fn ownership_rejects_field_level_mut_borrow_of_immutable_binding() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    user := User{name: "kim"}
    touchName(mut user.name)
}

func touchName(mut name string) {
    print(name)
}
"#,
        );
        assert!(error
            .message
            .contains("cannot mutably borrow field of immutable binding `user`"));
    }

    #[test]
    fn borrow_conflict_rejects_same_field_shared_then_mut_borrow_in_one_call() {
        let error = check_error(
            r#"
type User struct {
    name string
}

func main() {
    mut user := User{name: "kim"}
    compare(con user.name, mut user.name)
}

func compare(con left string, mut right string) {
    print(left)
    print(right)
}
"#,
        );
        assert!(error.message.contains("overlaps with an active borrow"));
    }

    #[test]
    fn borrow_conflict_allows_disjoint_field_mut_borrows_in_one_call() {
        check_ok(
            r#"
type Pair struct {
    left int
    right int
}

func main() {
    mut pair := Pair{left: 1, right: 2}
    touchBoth(mut pair.left, mut pair.right)
}

func touchBoth(mut left int, mut right int) {
    print(left)
    print(right)
}
"#,
        );
    }

    #[test]
    fn borrow_conflict_rejects_field_mut_borrow_overlapping_whole_struct_borrow() {
        let error = check_error(
            r#"
type Pair struct {
    left int
    right int
}

func main() {
    mut pair := Pair{left: 1, right: 2}
    touchBoth(mut pair.left, con pair)
}

func touchBoth(mut left int, con whole Pair) {
    print(left)
    print(whole.right)
}
"#,
        );
        assert!(error.message.contains("overlaps with an active borrow"));
    }

    #[test]
    fn ownership_allows_nested_field_borrow_argument() {
        check_ok(
            r#"
type Name struct {
    value string
}

type User struct {
    name Name
}

func main() {
    user := User{name: Name{value: "kim"}}
    show(con user.name.value)
}

func show(con value string) {
    print(value)
}
"#,
        );
    }

    #[test]
    fn ownership_allows_nested_field_mut_borrow_argument_on_mutable_binding() {
        check_ok(
            r#"
type Name struct {
    value string
}

type User struct {
    name Name
}

func main() {
    mut user := User{name: Name{value: "kim"}}
    touch(mut user.name.value)
}

func touch(mut value string) {
    print(value)
}
"#,
        );
    }

    #[test]
    fn borrow_conflict_allows_disjoint_nested_field_mut_borrows_in_one_call() {
        check_ok(
            r#"
type Name struct {
    first string
    last string
}

type User struct {
    name Name
}

func main() {
    mut user := User{name: Name{first: "kim", last: "lee"}}
    touchBoth(mut user.name.first, mut user.name.last)
}

func touchBoth(mut first string, mut last string) {
    print(first)
    print(last)
}
"#,
        );
    }

    #[test]
    fn borrow_conflict_rejects_nested_field_mut_borrow_overlapping_parent_field_borrow() {
        let error = check_error(
            r#"
type Name struct {
    first string
    last string
}

type User struct {
    name Name
}

func main() {
    mut user := User{name: Name{first: "kim", last: "lee"}}
    touchBoth(mut user.name.first, con user.name)
}

func touchBoth(mut first string, con name Name) {
    print(first)
    print(name.last)
}
"#,
        );
        assert!(error.message.contains("overlaps with an active borrow"));
    }

    #[test]
    fn borrow_conflict_allows_multiple_shared_borrows_in_one_call() {
        check_ok(
            r#"
func main() {
    s := "hello"
    compare(con s, con s)
}

func compare(con left string, con right string) {
    print(left)
    print(right)
}
"#,
        );
    }

    #[test]
    fn borrow_conflict_rejects_shared_then_mut_borrow_in_one_call() {
        let error = check_error(
            r#"
func main() {
    mut s := "hello"
    compare(con s, mut s)
}

func compare(con left string, mut right string) {
    print(left)
    print(right)
}
"#,
        );
        assert!(error.message.contains("overlaps with an active borrow"));
    }

    #[test]
    fn borrow_conflict_rejects_two_mut_borrows_in_one_call() {
        let error = check_error(
            r#"
func main() {
    mut s := "hello"
    compare(mut s, mut s)
}

func compare(mut left string, mut right string) {
    print(left)
    print(right)
}
"#,
        );
        assert!(error.message.contains("overlaps with an active borrow"));
    }
}
