use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use crate::{
    ast::{
        Block, EnumDecl, Expr, ExprKind, FieldDecl, ForInit, ForPost, Function, FunctionLiteral,
        MatchArm, MatchBlockArm, Program, Stmt, StmtKind, StructDecl, TypeParam, TypeRef,
        Visibility,
    },
    token::Span,
};

const MAX_SPECIALIZATIONS: usize = 1024;

struct EnumConstructorParts {
    enum_name: String,
    type_args: Vec<TypeRef>,
    variant: String,
    args: Option<Vec<crate::ast::Arg>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecializationError {
    pub message: String,
    pub span: Span,
}

impl SpecializationError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for SpecializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for SpecializationError {}

pub fn specialize(program: &Program) -> Result<Program, SpecializationError> {
    Ok(Specializer::new(program)?.run(program, false)?.program)
}

pub(crate) fn specialize_for_checking(
    program: &Program,
) -> Result<SymbolicProgram, SpecializationError> {
    Specializer::new(program)?.run(program, false)
}

pub fn specialize_for_validation(
    program: &Program,
) -> Result<SymbolicProgram, SpecializationError> {
    Specializer::new(program)?.run(program, true)
}

#[derive(Debug, Clone)]
pub struct SymbolicProgram {
    pub program: Program,
    type_names: HashMap<String, String>,
}

impl SymbolicProgram {
    pub fn display_message(&self, message: &str) -> String {
        let mut names = self.type_names.iter().collect::<Vec<_>>();
        names.sort_by(|(left, _), (right, _)| {
            right.len().cmp(&left.len()).then_with(|| left.cmp(right))
        });
        names
            .into_iter()
            .fold(message.to_string(), |message, (internal, source)| {
                message.replace(internal, source)
            })
    }
}

struct Specializer {
    generic_structs: HashMap<String, StructDecl>,
    generic_enums: HashMap<String, EnumDecl>,
    concrete_enums: HashMap<String, EnumDecl>,
    generic_functions: HashMap<String, Function>,
    generic_methods: HashMap<String, Vec<Function>>,
    struct_specializations: HashMap<String, String>,
    enum_specializations: HashMap<String, String>,
    function_specializations: HashMap<String, String>,
    active_structs: HashMap<String, String>,
    active_enums: HashMap<String, String>,
    active_functions: HashMap<String, String>,
    generated_structs: Vec<StructDecl>,
    generated_enums: Vec<EnumDecl>,
    generated_functions: Vec<Function>,
    used_type_names: HashSet<String>,
    symbolic_type_names: HashMap<String, String>,
    enforce_budget: bool,
}

impl Specializer {
    fn new(program: &Program) -> Result<Self, SpecializationError> {
        let mut generic_structs = HashMap::new();
        let mut generic_enums = HashMap::new();
        let mut concrete_enums = HashMap::new();
        let mut generic_functions = HashMap::new();
        let mut generic_methods: HashMap<String, Vec<Function>> = HashMap::new();

        for declaration in &program.structs {
            if declaration.type_params.is_empty() {
                continue;
            }
            validate_type_params(
                &declaration.name,
                &declaration.type_params,
                declaration.span,
            )?;
            if generic_structs
                .insert(declaration.name.clone(), declaration.clone())
                .is_some()
            {
                return Err(SpecializationError::new(
                    format!("duplicate generic struct `{}`", declaration.name),
                    declaration.span,
                ));
            }
        }

        for declaration in &program.enums {
            let target = if declaration.type_params.is_empty() {
                &mut concrete_enums
            } else {
                validate_type_params(
                    &declaration.name,
                    &declaration.type_params,
                    declaration.span,
                )?;
                &mut generic_enums
            };
            if target
                .insert(declaration.name.clone(), declaration.clone())
                .is_some()
            {
                return Err(SpecializationError::new(
                    format!("duplicate enum `{}`", declaration.name),
                    declaration.span,
                ));
            }
        }

        for function in &program.functions {
            if let Some(receiver) = &function.receiver {
                if let Some(declaration) = generic_structs.get(&receiver.ty.name) {
                    if !function.type_params.is_empty() {
                        return Err(SpecializationError::new(
                            "methods cannot declare independent type parameters in v0.4",
                            function.span,
                        ));
                    }
                    validate_generic_receiver(function, declaration)?;
                    let methods = generic_methods.entry(receiver.ty.name.clone()).or_default();
                    if methods
                        .iter()
                        .any(|candidate| candidate.name == function.name)
                    {
                        return Err(SpecializationError::new(
                            format!(
                                "duplicate generic method `{}` on `{}`",
                                function.name, receiver.ty.name
                            ),
                            function.span,
                        ));
                    }
                    methods.push(function.clone());
                    continue;
                }
            }
            if function.type_params.is_empty() {
                continue;
            }
            validate_type_params(&function.name, &function.type_params, function.span)?;
            if function.name == "main" {
                return Err(SpecializationError::new(
                    "`main` cannot declare type parameters",
                    function.span,
                ));
            }
            if function.receiver.is_some() {
                return Err(SpecializationError::new(
                    "methods cannot declare independent type parameters in v0.4",
                    function.span,
                ));
            }
            if generic_functions
                .insert(function.name.clone(), function.clone())
                .is_some()
            {
                return Err(SpecializationError::new(
                    format!("duplicate generic function `{}`", function.name),
                    function.span,
                ));
            }
        }

        for name in generic_structs.keys() {
            if program
                .structs
                .iter()
                .any(|declaration| declaration.type_params.is_empty() && declaration.name == *name)
                || generic_enums.contains_key(name)
                || concrete_enums.contains_key(name)
                || generic_functions.contains_key(name)
                || program.functions.iter().any(|function| {
                    function.type_params.is_empty()
                        && function.receiver.is_none()
                        && function.name == *name
                })
            {
                return Err(SpecializationError::new(
                    format!("generic declaration `{name}` conflicts with another declaration"),
                    generic_structs[name].span,
                ));
            }
        }

        for name in generic_enums.keys() {
            if program
                .structs
                .iter()
                .any(|declaration| declaration.name == *name)
                || concrete_enums.contains_key(name)
                || generic_functions.contains_key(name)
                || program.functions.iter().any(|function| {
                    function.type_params.is_empty()
                        && function.receiver.is_none()
                        && function.name == *name
                })
            {
                return Err(SpecializationError::new(
                    format!("generic declaration `{name}` conflicts with another declaration"),
                    generic_enums[name].span,
                ));
            }
        }

        for name in generic_functions.keys() {
            if program.functions.iter().any(|function| {
                function.type_params.is_empty()
                    && function.receiver.is_none()
                    && function.name == *name
            }) {
                return Err(SpecializationError::new(
                    format!("generic declaration `{name}` conflicts with another declaration"),
                    generic_functions[name].span,
                ));
            }
        }

        Ok(Self {
            generic_structs,
            generic_enums,
            concrete_enums,
            generic_functions,
            generic_methods,
            struct_specializations: HashMap::new(),
            enum_specializations: HashMap::new(),
            function_specializations: HashMap::new(),
            active_structs: HashMap::new(),
            active_enums: HashMap::new(),
            active_functions: HashMap::new(),
            generated_structs: Vec::new(),
            generated_enums: Vec::new(),
            generated_functions: Vec::new(),
            used_type_names: program
                .structs
                .iter()
                .map(|declaration| declaration.name.clone())
                .chain(
                    program
                        .enums
                        .iter()
                        .map(|declaration| declaration.name.clone()),
                )
                .collect(),
            symbolic_type_names: HashMap::new(),
            enforce_budget: true,
        })
    }

    fn run(
        mut self,
        program: &Program,
        symbolic: bool,
    ) -> Result<SymbolicProgram, SpecializationError> {
        self.enforce_budget = !symbolic;
        let mut output = program.clone();
        output.structs.clear();
        output.enums.clear();
        output.functions.clear();

        for mut declaration in program
            .structs
            .iter()
            .filter(|declaration| declaration.type_params.is_empty())
            .cloned()
        {
            for field in &mut declaration.fields {
                self.rewrite_type_ref(&mut field.ty, &HashMap::new())?;
            }
            output.structs.push(declaration);
        }

        for mut declaration in program
            .enums
            .iter()
            .filter(|declaration| declaration.type_params.is_empty())
            .cloned()
        {
            for variant in &mut declaration.variants {
                if let Some(payload) = &mut variant.payload {
                    self.rewrite_type_ref(payload, &HashMap::new())?;
                }
            }
            output.enums.push(declaration);
        }

        let concrete_functions = program
            .functions
            .iter()
            .filter(|function| {
                function.type_params.is_empty() && !self.is_generic_receiver_method(function)
            })
            .cloned()
            .collect::<Vec<_>>();
        for mut function in concrete_functions {
            self.rewrite_function(&mut function, &HashMap::new())?;
            output.functions.push(function);
        }

        if symbolic {
            self.add_symbolic_demands(&mut output)?;
        }

        output.structs.append(&mut self.generated_structs);
        output.enums.append(&mut self.generated_enums);
        output.functions.append(&mut self.generated_functions);
        Ok(SymbolicProgram {
            program: output,
            type_names: self.symbolic_type_names,
        })
    }

    fn is_generic_receiver_method(&self, function: &Function) -> bool {
        function.receiver.as_ref().is_some_and(|receiver| {
            self.generic_methods
                .get(&receiver.ty.name)
                .is_some_and(|methods| methods.iter().any(|method| method.span == function.span))
        })
    }

    fn add_symbolic_demands(&mut self, output: &mut Program) -> Result<(), SpecializationError> {
        let mut structs = self
            .generic_structs
            .values()
            .map(|declaration| {
                (
                    declaration.name.clone(),
                    declaration.type_params.clone(),
                    declaration.span,
                )
            })
            .collect::<Vec<_>>();
        structs.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, params, span) in structs {
            let args = self.symbolic_args(&name, &params, output);
            self.specialize_struct(&name, args, span)?;
        }

        let mut enums = self
            .generic_enums
            .values()
            .map(|declaration| {
                (
                    declaration.name.clone(),
                    declaration.type_params.clone(),
                    declaration.span,
                )
            })
            .collect::<Vec<_>>();
        enums.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, params, span) in enums {
            let args = self.symbolic_args(&name, &params, output);
            self.specialize_enum(&name, args, span)?;
        }

        let mut functions = self
            .generic_functions
            .values()
            .map(|declaration| {
                (
                    declaration.name.clone(),
                    declaration.type_params.clone(),
                    declaration.span,
                )
            })
            .collect::<Vec<_>>();
        functions.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, params, span) in functions {
            let args = self.symbolic_args(&name, &params, output);
            self.specialize_function(&name, args, span)?;
        }
        Ok(())
    }

    fn symbolic_args(
        &mut self,
        declaration: &str,
        params: &[TypeParam],
        output: &mut Program,
    ) -> Vec<TypeRef> {
        params
            .iter()
            .enumerate()
            .map(|(index, param)| {
                let key = format!("{declaration}:{}:{index}", param.name);
                let base = format!("__mlg_symbolic_type_{}", hex_encode(key.as_bytes()));
                let mut name = base.clone();
                let mut suffix = 0usize;
                while !self.used_type_names.insert(name.clone()) {
                    suffix += 1;
                    name = format!("{base}_{suffix}");
                }
                self.symbolic_type_names
                    .insert(name.clone(), param.name.clone());

                output.structs.push(StructDecl {
                    visibility: Visibility::Package,
                    name: name.clone(),
                    type_params: Vec::new(),
                    fields: vec![FieldDecl {
                        name: "__non_printable".to_string(),
                        ty: TypeRef {
                            name: "Array".to_string(),
                            args: vec![simple_type_ref("int".to_string(), param.span)],
                            array_len: Some(1),
                            slice: false,
                            function: None,
                            span: param.span,
                        },
                        span: param.span,
                    }],
                    span: param.span,
                });
                simple_type_ref(name, param.span)
            })
            .collect()
    }

    fn rewrite_function(
        &mut self,
        function: &mut Function,
        substitutions: &HashMap<String, TypeRef>,
    ) -> Result<(), SpecializationError> {
        if let Some(receiver) = &mut function.receiver {
            self.substitute_and_rewrite_type_ref(&mut receiver.ty, substitutions)?;
        }
        for param in &mut function.params {
            self.substitute_and_rewrite_type_ref(&mut param.ty, substitutions)?;
        }
        if let Some(return_type) = &mut function.return_type {
            self.substitute_and_rewrite_type_ref(return_type, substitutions)?;
        }
        self.rewrite_block(&mut function.body, substitutions)
    }

    fn rewrite_block(
        &mut self,
        block: &mut Block,
        substitutions: &HashMap<String, TypeRef>,
    ) -> Result<(), SpecializationError> {
        for statement in &mut block.statements {
            self.rewrite_stmt(statement, substitutions)?;
        }
        Ok(())
    }

    fn rewrite_stmt(
        &mut self,
        statement: &mut Stmt,
        substitutions: &HashMap<String, TypeRef>,
    ) -> Result<(), SpecializationError> {
        match &mut statement.kind {
            StmtKind::Let { expr, .. }
            | StmtKind::Assign { expr, .. }
            | StmtKind::Return { expr }
            | StmtKind::Expr { expr } => self.rewrite_expr(expr, substitutions),
            StmtKind::FieldAssign { base, expr, .. } => {
                self.rewrite_expr(base, substitutions)?;
                self.rewrite_expr(expr, substitutions)
            }
            StmtKind::IndexAssign { base, index, expr } => {
                self.rewrite_expr(base, substitutions)?;
                self.rewrite_expr(index, substitutions)?;
                self.rewrite_expr(expr, substitutions)
            }
            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.rewrite_expr(condition, substitutions)?;
                self.rewrite_block(then_block, substitutions)?;
                if let Some(else_block) = else_block {
                    self.rewrite_block(else_block, substitutions)?;
                }
                Ok(())
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
            } => {
                if let Some(ForInit::Let { expr, .. }) = init {
                    self.rewrite_expr(expr, substitutions)?;
                }
                if let Some(condition) = condition {
                    self.rewrite_expr(condition, substitutions)?;
                }
                if let Some(ForPost::Assign { target, expr }) = post {
                    self.rewrite_expr(target, substitutions)?;
                    self.rewrite_expr(expr, substitutions)?;
                }
                self.rewrite_block(body, substitutions)
            }
            StmtKind::RangeFor { source, body, .. } => {
                self.rewrite_expr(source, substitutions)?;
                self.rewrite_block(body, substitutions)
            }
            StmtKind::Match { scrutinee, arms } => {
                self.rewrite_expr(scrutinee, substitutions)?;
                for MatchBlockArm { block, .. } in arms {
                    self.rewrite_block(block, substitutions)?;
                }
                Ok(())
            }
            StmtKind::Break | StmtKind::Continue => Ok(()),
        }
    }

    fn rewrite_expr(
        &mut self,
        expr: &mut Expr,
        substitutions: &HashMap<String, TypeRef>,
    ) -> Result<(), SpecializationError> {
        if let Some(mut constructor) = enum_constructor_parts(expr) {
            if self.generic_enums.contains_key(&constructor.enum_name)
                || self.concrete_enums.contains_key(&constructor.enum_name)
            {
                for type_arg in &mut constructor.type_args {
                    self.substitute_and_rewrite_type_ref(type_arg, substitutions)?;
                }
                let enum_name =
                    self.specialize_enum(&constructor.enum_name, constructor.type_args, expr.span)?;
                if let Some(args) = &mut constructor.args {
                    for arg in args {
                        self.rewrite_expr(&mut arg.expr, substitutions)?;
                    }
                }
                expr.kind = ExprKind::EnumConstructor {
                    enum_name,
                    variant: constructor.variant,
                    args: constructor.args,
                };
                return Ok(());
            }
        }

        match &mut expr.kind {
            ExprKind::Int(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Nil => Ok(()),
            ExprKind::Var(name) => {
                if self.generic_functions.contains_key(name) {
                    return Err(SpecializationError::new(
                        format!("generic function `{name}` requires explicit type arguments"),
                        expr.span,
                    ));
                }
                Ok(())
            }
            ExprKind::FunctionLiteral(function) => {
                self.rewrite_function_literal(function, substitutions)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.rewrite_expr(condition, substitutions)?;
                self.rewrite_expr(then_branch, substitutions)?;
                self.rewrite_expr(else_branch, substitutions)
            }
            ExprKind::Match { scrutinee, arms } => {
                self.rewrite_expr(scrutinee, substitutions)?;
                for MatchArm { expr, .. } in arms {
                    self.rewrite_expr(expr, substitutions)?;
                }
                Ok(())
            }
            ExprKind::StructLiteral {
                type_name,
                type_args,
                fields,
            } => {
                for type_arg in type_args.iter_mut() {
                    self.substitute_and_rewrite_type_ref(type_arg, substitutions)?;
                }
                if self.generic_structs.contains_key(type_name) {
                    let specialized =
                        self.specialize_struct(type_name, type_args.clone(), expr.span)?;
                    *type_name = specialized;
                    type_args.clear();
                } else if !type_args.is_empty() {
                    return Err(SpecializationError::new(
                        format!("type `{type_name}` is not a generic struct"),
                        expr.span,
                    ));
                }
                for field in fields {
                    self.rewrite_expr(&mut field.expr, substitutions)?;
                }
                Ok(())
            }
            ExprKind::ArrayLiteral { ty, elements } => {
                self.substitute_and_rewrite_type_ref(ty, substitutions)?;
                for element in elements {
                    self.rewrite_expr(element, substitutions)?;
                }
                Ok(())
            }
            ExprKind::FieldAccess { base, .. } => self.rewrite_expr(base, substitutions),
            ExprKind::Index { base, index } => {
                if let Some(name) = expression_path(base) {
                    if self.generic_functions.contains_key(&name) {
                        let mut type_arg = expression_as_type_ref(index).ok_or_else(|| {
                            SpecializationError::new(
                                format!("type argument for `{name}` must be a type"),
                                index.span,
                            )
                        })?;
                        self.substitute_and_rewrite_type_ref(&mut type_arg, substitutions)?;
                        let specialized =
                            self.specialize_function(&name, vec![type_arg], expr.span)?;
                        expr.kind = ExprKind::Var(specialized);
                        return Ok(());
                    }
                }
                self.rewrite_expr(base, substitutions)?;
                self.rewrite_expr(index, substitutions)
            }
            ExprKind::TypeApply { base, args } => {
                let name = expression_path(base).ok_or_else(|| {
                    SpecializationError::new(
                        "generic type arguments require a named function",
                        base.span,
                    )
                })?;
                if !self.generic_functions.contains_key(&name) {
                    return Err(SpecializationError::new(
                        format!("function `{name}` is not generic"),
                        expr.span,
                    ));
                }
                for arg in args.iter_mut() {
                    self.substitute_and_rewrite_type_ref(arg, substitutions)?;
                }
                let specialized = self.specialize_function(&name, args.clone(), expr.span)?;
                expr.kind = ExprKind::Var(specialized);
                Ok(())
            }
            ExprKind::EnumConstructor { args, .. } => {
                if let Some(args) = args {
                    for arg in args {
                        self.rewrite_expr(&mut arg.expr, substitutions)?;
                    }
                }
                Ok(())
            }
            ExprKind::Call { callee, args } => {
                self.rewrite_expr(callee, substitutions)?;
                for arg in args {
                    self.rewrite_expr(&mut arg.expr, substitutions)?;
                }
                Ok(())
            }
            ExprKind::Unary { expr, .. } => self.rewrite_expr(expr, substitutions),
            ExprKind::Binary { left, right, .. } => {
                self.rewrite_expr(left, substitutions)?;
                self.rewrite_expr(right, substitutions)
            }
        }
    }

    fn rewrite_function_literal(
        &mut self,
        function: &mut FunctionLiteral,
        substitutions: &HashMap<String, TypeRef>,
    ) -> Result<(), SpecializationError> {
        for param in &mut function.params {
            self.substitute_and_rewrite_type_ref(&mut param.ty, substitutions)?;
        }
        if let Some(return_type) = &mut function.return_type {
            self.substitute_and_rewrite_type_ref(return_type, substitutions)?;
        }
        self.rewrite_block(&mut function.body, substitutions)
    }

    fn substitute_and_rewrite_type_ref(
        &mut self,
        ty: &mut TypeRef,
        substitutions: &HashMap<String, TypeRef>,
    ) -> Result<(), SpecializationError> {
        if ty.function.is_none()
            && ty.array_len.is_none()
            && !ty.slice
            && ty.args.is_empty()
            && substitutions.contains_key(&ty.name)
        {
            let span = ty.span;
            *ty = substitutions[&ty.name].clone();
            ty.span = span;
        }
        self.rewrite_type_ref(ty, substitutions)
    }

    fn rewrite_type_ref(
        &mut self,
        ty: &mut TypeRef,
        substitutions: &HashMap<String, TypeRef>,
    ) -> Result<(), SpecializationError> {
        for arg in &mut ty.args {
            self.substitute_and_rewrite_type_ref(arg, substitutions)?;
        }
        if let Some(function) = &mut ty.function {
            for param in &mut function.params {
                self.substitute_and_rewrite_type_ref(&mut param.ty, substitutions)?;
            }
            self.substitute_and_rewrite_type_ref(&mut function.return_type, substitutions)?;
        }

        if self.generic_structs.contains_key(&ty.name) {
            let specialized = self.specialize_struct(&ty.name, ty.args.clone(), ty.span)?;
            ty.name = specialized;
            ty.args.clear();
        } else if self.generic_enums.contains_key(&ty.name) {
            let specialized = self.specialize_enum(&ty.name, ty.args.clone(), ty.span)?;
            ty.name = specialized;
            ty.args.clear();
        } else if self.concrete_enums.contains_key(&ty.name) && !ty.args.is_empty() {
            return Err(SpecializationError::new(
                format!("enum type `{}` does not take type arguments", ty.name),
                ty.span,
            ));
        }
        Ok(())
    }

    fn specialize_struct(
        &mut self,
        name: &str,
        args: Vec<TypeRef>,
        span: Span,
    ) -> Result<String, SpecializationError> {
        let declaration = self
            .generic_structs
            .get(name)
            .cloned()
            .expect("generic struct existence checked before specialization");
        check_arity(name, declaration.type_params.len(), &args, span)?;
        let key = specialization_key(name, &args);
        if let Some(specialized) = self.struct_specializations.get(&key) {
            return Ok(specialized.clone());
        }
        self.check_specialization_budget(span)?;
        if let Some(active_key) = self.active_structs.get(name) {
            return Err(SpecializationError::new(
                format!(
                    "generic struct `{name}` creates an expanding specialization cycle: `{active_key}` -> `{key}`"
                ),
                span,
            ));
        }

        let specialized_name = specialized_name("type", name, &args);
        self.symbolic_type_names
            .insert(specialized_name.clone(), specialization_key(name, &args));
        self.struct_specializations
            .insert(key.clone(), specialized_name.clone());
        self.active_structs.insert(name.to_string(), key);

        let substitutions = declaration
            .type_params
            .iter()
            .zip(args)
            .map(|(param, arg)| (param.name.clone(), arg))
            .collect::<HashMap<_, _>>();
        let mut specialized = declaration;
        specialized.name = specialized_name.clone();
        specialized.type_params.clear();
        for field in &mut specialized.fields {
            self.substitute_and_rewrite_type_ref(&mut field.ty, &substitutions)?;
        }

        let methods = self.generic_methods.get(name).cloned().unwrap_or_default();
        for mut method in methods {
            self.rewrite_function(&mut method, &substitutions)?;
            self.generated_functions.push(method);
        }

        self.active_structs.remove(name);
        self.generated_structs.push(specialized);
        Ok(specialized_name)
    }

    fn specialize_enum(
        &mut self,
        name: &str,
        args: Vec<TypeRef>,
        span: Span,
    ) -> Result<String, SpecializationError> {
        if let Some(declaration) = self.concrete_enums.get(name) {
            check_arity(name, 0, &args, span)?;
            return Ok(declaration.name.clone());
        }

        let declaration = self
            .generic_enums
            .get(name)
            .cloned()
            .expect("enum existence checked before specialization");
        check_arity(name, declaration.type_params.len(), &args, span)?;
        let key = specialization_key(name, &args);
        if let Some(specialized) = self.enum_specializations.get(&key) {
            return Ok(specialized.clone());
        }
        self.check_specialization_budget(span)?;
        if let Some(active_key) = self.active_enums.get(name) {
            return Err(SpecializationError::new(
                format!(
                    "generic enum `{name}` creates an expanding specialization cycle: `{active_key}` -> `{key}`"
                ),
                span,
            ));
        }

        let specialized_name = specialized_name("enum", name, &args);
        self.symbolic_type_names
            .insert(specialized_name.clone(), specialization_key(name, &args));
        self.enum_specializations
            .insert(key.clone(), specialized_name.clone());
        self.active_enums.insert(name.to_string(), key);

        let substitutions = declaration
            .type_params
            .iter()
            .zip(args)
            .map(|(param, arg)| (param.name.clone(), arg))
            .collect::<HashMap<_, _>>();
        let mut specialized = declaration;
        specialized.name = specialized_name.clone();
        specialized.type_params.clear();
        for variant in &mut specialized.variants {
            if let Some(payload) = &mut variant.payload {
                self.substitute_and_rewrite_type_ref(payload, &substitutions)?;
            }
        }

        self.active_enums.remove(name);
        self.generated_enums.push(specialized);
        Ok(specialized_name)
    }

    fn specialize_function(
        &mut self,
        name: &str,
        args: Vec<TypeRef>,
        span: Span,
    ) -> Result<String, SpecializationError> {
        let declaration = self
            .generic_functions
            .get(name)
            .cloned()
            .expect("generic function existence checked before specialization");
        check_arity(name, declaration.type_params.len(), &args, span)?;
        let key = specialization_key(name, &args);
        if let Some(specialized) = self.function_specializations.get(&key) {
            return Ok(specialized.clone());
        }
        self.check_specialization_budget(span)?;
        if let Some(active_key) = self.active_functions.get(name) {
            return Err(SpecializationError::new(
                format!(
                    "generic function `{name}` creates an expanding specialization cycle: `{active_key}` -> `{key}`"
                ),
                span,
            ));
        }

        let specialized_name = specialized_name("func", name, &args);
        self.symbolic_type_names
            .insert(specialized_name.clone(), specialization_key(name, &args));
        self.function_specializations
            .insert(key.clone(), specialized_name.clone());
        self.active_functions.insert(name.to_string(), key);

        let substitutions = declaration
            .type_params
            .iter()
            .zip(args)
            .map(|(param, arg)| (param.name.clone(), arg))
            .collect::<HashMap<_, _>>();
        let mut specialized = declaration;
        specialized.name = specialized_name.clone();
        specialized.type_params.clear();
        self.rewrite_function(&mut specialized, &substitutions)?;

        self.active_functions.remove(name);
        self.generated_functions.push(specialized);
        Ok(specialized_name)
    }

    fn check_specialization_budget(&self, span: Span) -> Result<(), SpecializationError> {
        if self.enforce_budget
            && self.struct_specializations.len()
                + self.enum_specializations.len()
                + self.function_specializations.len()
                >= MAX_SPECIALIZATIONS
        {
            return Err(SpecializationError::new(
                format!("generic specialization limit of {MAX_SPECIALIZATIONS} exceeded"),
                span,
            ));
        }
        Ok(())
    }
}

fn validate_generic_receiver(
    function: &Function,
    declaration: &StructDecl,
) -> Result<(), SpecializationError> {
    let receiver = function
        .receiver
        .as_ref()
        .expect("generic receiver validation requires a method");
    if receiver.ty.function.is_some()
        || receiver.ty.slice
        || receiver.ty.array_len.is_some()
        || receiver.ty.args.len() != declaration.type_params.len()
        || receiver
            .ty
            .args
            .iter()
            .zip(&declaration.type_params)
            .any(|(arg, param)| {
                arg.function.is_some()
                    || arg.slice
                    || arg.array_len.is_some()
                    || !arg.args.is_empty()
                    || arg.name != param.name
            })
    {
        let expected = declaration
            .type_params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(SpecializationError::new(
            format!(
                "generic receiver for `{}` must bind declared type parameters as `{}[{expected}]`",
                declaration.name, declaration.name
            ),
            receiver.ty.span,
        ));
    }
    Ok(())
}

fn validate_type_params(
    declaration: &str,
    params: &[crate::ast::TypeParam],
    span: Span,
) -> Result<(), SpecializationError> {
    let mut seen = HashMap::new();
    for param in params {
        if seen.insert(param.name.as_str(), param.span).is_some() {
            return Err(SpecializationError::new(
                format!(
                    "duplicate type parameter `{}` in `{declaration}`",
                    param.name
                ),
                param.span,
            ));
        }
    }
    if params.is_empty() {
        return Err(SpecializationError::new(
            format!("generic declaration `{declaration}` has no type parameters"),
            span,
        ));
    }
    Ok(())
}

fn check_arity(
    name: &str,
    expected: usize,
    args: &[TypeRef],
    span: Span,
) -> Result<(), SpecializationError> {
    if args.len() != expected {
        return Err(SpecializationError::new(
            format!(
                "generic declaration `{name}` expects {expected} type argument(s), got {}",
                args.len()
            ),
            span,
        ));
    }
    Ok(())
}

fn specialization_key(name: &str, args: &[TypeRef]) -> String {
    format!(
        "{name}[{}]",
        args.iter()
            .map(render_type_ref)
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn specialized_name(kind: &str, name: &str, args: &[TypeRef]) -> String {
    let key = specialization_key(name, args);
    format!("__mlg_spec_{kind}_{}", hex_encode(key.as_bytes()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn render_type_ref(ty: &TypeRef) -> String {
    if let Some(function) = &ty.function {
        let params = function
            .params
            .iter()
            .map(|param| format!("{:?}:{}", param.mode, render_type_ref(&param.ty)))
            .collect::<Vec<_>>()
            .join(",");
        return format!(
            "func{}({params}){}",
            if function.mutable { " mut" } else { "" },
            render_type_ref(&function.return_type)
        );
    }
    if ty.slice {
        return format!("[]{}", render_type_ref(&ty.args[0]));
    }
    if let Some(length) = ty.array_len {
        return format!("[{length}]{}", render_type_ref(&ty.args[0]));
    }
    if ty.args.is_empty() {
        ty.name.clone()
    } else {
        format!(
            "{}[{}]",
            ty.name,
            ty.args
                .iter()
                .map(render_type_ref)
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

fn expression_path(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Var(name) => Some(name.clone()),
        ExprKind::FieldAccess { base, field } => {
            Some(format!("{}.{}", expression_path(base)?, field))
        }
        _ => None,
    }
}

fn enum_constructor_parts(expr: &Expr) -> Option<EnumConstructorParts> {
    let (callee, args) = match &expr.kind {
        ExprKind::Call { callee, args } => (callee.as_ref(), Some(args.clone())),
        ExprKind::FieldAccess { .. } => (expr, None),
        _ => return None,
    };
    let ExprKind::FieldAccess { base, field } = &callee.kind else {
        return None;
    };
    let ty = expression_as_type_ref(base)?;
    Some(EnumConstructorParts {
        enum_name: ty.name,
        type_args: ty.args,
        variant: field.clone(),
        args,
    })
}

fn expression_as_type_ref(expr: &Expr) -> Option<TypeRef> {
    match &expr.kind {
        ExprKind::Var(name) => Some(simple_type_ref(name.clone(), expr.span)),
        ExprKind::FieldAccess { .. } => Some(simple_type_ref(expression_path(expr)?, expr.span)),
        ExprKind::Index { base, index } => Some(TypeRef {
            name: expression_path(base)?,
            args: vec![expression_as_type_ref(index)?],
            array_len: None,
            slice: false,
            function: None,
            span: expr.span,
        }),
        ExprKind::TypeApply { base, args } => Some(TypeRef {
            name: expression_path(base)?,
            args: args.clone(),
            array_len: None,
            slice: false,
            function: None,
            span: expr.span,
        }),
        _ => None,
    }
}

fn simple_type_ref(name: String, span: Span) -> TypeRef {
    TypeRef {
        name,
        args: Vec::new(),
        array_len: None,
        slice: false,
        function: None,
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn specializes_generic_structs_functions_and_function_values() {
        let program = parse(
            r#"
type Pair[A, B] struct {
    first A
    second B
}

func Identity[T](value T) T {
    return value
}

func main() {
    pair := Pair[int, string]{first: 7, second: "mallang"}
    identity := Identity[int]
    print(identity(pair.first))
    print(Identity[string](pair.second))
}
"#,
        )
        .unwrap();

        let specialized = specialize(&program).unwrap();
        assert_eq!(specialized.structs.len(), 1);
        assert!(specialized.structs[0].type_params.is_empty());
        assert!(specialized.structs[0].name.starts_with("__mlg_spec_type_"));
        assert_eq!(specialized.functions.len(), 3);
        assert!(specialized
            .functions
            .iter()
            .all(|function| function.type_params.is_empty()));
        assert_eq!(
            specialized
                .functions
                .iter()
                .filter(|function| function.name.starts_with("__mlg_spec_func_"))
                .count(),
            2
        );
    }

    #[test]
    fn specializes_generic_enums_and_normalizes_constructors() {
        let program = parse(
            r#"
type Maybe[T] enum {
    None
    Some(T)
}

func main() {
    first := Maybe[int].Some(7)
    second := Maybe[string].Some("mallang")
    empty := Maybe[int].None
}
"#,
        )
        .unwrap();

        let specialized = specialize(&program).unwrap();
        assert_eq!(specialized.enums.len(), 2);
        assert!(specialized.enums.iter().all(|declaration| {
            declaration.type_params.is_empty() && declaration.name.starts_with("__mlg_spec_enum_")
        }));

        let main = specialized
            .functions
            .iter()
            .find(|function| function.name == "main")
            .unwrap();
        let constructors = main
            .body
            .statements
            .iter()
            .map(|statement| match &statement.kind {
                StmtKind::Let { expr, .. } => &expr.kind,
                _ => panic!("expected constructor binding"),
            })
            .collect::<Vec<_>>();
        assert!(matches!(
            constructors[0],
            ExprKind::EnumConstructor {
                variant,
                args: Some(args),
                ..
            } if variant == "Some" && args.len() == 1
        ));
        assert!(matches!(
            constructors[2],
            ExprKind::EnumConstructor {
                variant,
                args: None,
                ..
            } if variant == "None"
        ));
    }

    #[test]
    fn reuses_identical_specializations_and_rejects_wrong_arity() {
        let program = parse(
            r#"
func Identity[T](value T) T { return value }
func main() {
    print(Identity[int](1))
    print(Identity[int](2))
}
"#,
        )
        .unwrap();
        let specialized = specialize(&program).unwrap();
        assert_eq!(
            specialized
                .functions
                .iter()
                .filter(|function| function.name.starts_with("__mlg_spec_func_"))
                .count(),
            1
        );

        let wrong = parse(
            "func Pick[A, B](left A, right B) A { return left }\nfunc main() { print(Pick[int](1, 2)) }\n",
        )
        .unwrap();
        let error = specialize(&wrong).unwrap_err();
        assert!(error.message.contains("expects 2 type argument(s), got 1"));
    }

    #[test]
    fn rejects_expanding_specialization_cycles_but_reuses_same_key_recursion() {
        let expanding = parse(
            r#"
type Grow[T] struct {
    next Grow[Option[T]]
}
func consume(value Grow[int]) {}
func main() {}
"#,
        )
        .unwrap();
        let error = specialize(&expanding).unwrap_err();
        assert!(error
            .message
            .contains("generic struct `Grow` creates an expanding specialization cycle"));

        let recursive = parse(
            r#"
func Loop[T](value T) T {
    return Loop[T](value)
}
func main() {
    loop := Loop[int]
}
"#,
        )
        .unwrap();
        let specialized = specialize(&recursive).unwrap();
        assert_eq!(
            specialized
                .functions
                .iter()
                .filter(|function| function.name.starts_with("__mlg_spec_func_"))
                .count(),
            1
        );
    }

    #[test]
    fn specializes_generic_receiver_methods_with_their_struct() {
        let program = parse(
            r#"
type Box[T] struct { value T }
func (mut box Box[T]) replace(value T) { box.value = value }
func main() {
    mut box := Box[string]{value: "before"}
    box.replace("after")
}
"#,
        )
        .unwrap();

        let specialized = specialize(&program).unwrap();
        let receiver = specialized
            .functions
            .iter()
            .find(|function| function.name == "replace")
            .and_then(|function| function.receiver.as_ref())
            .unwrap();
        assert!(receiver.ty.name.starts_with("__mlg_spec_type_"));
        assert!(receiver.ty.args.is_empty());
        assert!(specialized.functions.iter().all(|function| {
            function.type_params.is_empty()
                && function
                    .receiver
                    .as_ref()
                    .is_none_or(|receiver| receiver.ty.args.is_empty())
        }));
    }
}
