use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ast::{
        Block, Expr, ExprKind, ForInit, ForPost, MatchPattern, Program, Stmt, StmtKind, TypeRef,
        Visibility,
    },
    package::{PackageDeclaration, PackageDeclarationKind, PackageGraph},
    project::Project,
    token::{SourceId, Span},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkError {
    pub message: String,
    pub span: Span,
}

impl LinkError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for LinkError {}

pub fn link_project(
    project: &Project,
    graph: &PackageGraph,
    program: &Program,
) -> Result<Program, LinkError> {
    let linker = Linker::new(project, graph, program)?;
    linker.link(program)
}

pub fn display_linked_message(message: &str) -> String {
    const PREFIX: &str = "__mlg_pkg_";

    let mut output = String::with_capacity(message.len());
    let mut remaining = message;
    while let Some(start) = remaining.find(PREFIX) {
        output.push_str(&remaining[..start]);
        let symbol = &remaining[start + PREFIX.len()..];
        let Some(separator) = symbol.find('_') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let encoded_path = &symbol[..separator];
        let name_source = &symbol[separator + 1..];
        let name_len = name_source
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        let Some(package_path) = decode_package_path(encoded_path) else {
            output.push_str(&remaining[start..start + PREFIX.len()]);
            remaining = symbol;
            continue;
        };
        if name_len == 0 {
            output.push_str(&remaining[start..start + PREFIX.len()]);
            remaining = symbol;
            continue;
        }

        output.push_str(&package_path);
        output.push('.');
        output.push_str(&name_source[..name_len]);
        remaining = &name_source[name_len..];
    }
    output.push_str(remaining);
    output
}

fn decode_package_path(encoded: &str) -> Option<String> {
    if encoded.is_empty() || !encoded.len().is_multiple_of(2) {
        return None;
    }

    let bytes = encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

#[derive(Debug, Clone)]
struct FileContext {
    package_path: String,
    imports: BTreeMap<String, String>,
}

struct Linker<'a> {
    project: &'a Project,
    graph: &'a PackageGraph,
    contexts: BTreeMap<SourceId, FileContext>,
}

impl<'a> Linker<'a> {
    fn new(
        project: &'a Project,
        graph: &'a PackageGraph,
        program: &Program,
    ) -> Result<Self, LinkError> {
        let mut contexts = BTreeMap::new();
        for unit in &program.source_units {
            let package = graph.package_for_source(unit.span.source).ok_or_else(|| {
                LinkError::new(
                    format!(
                        "source ID {} is not part of the package graph",
                        unit.span.source.index()
                    ),
                    unit.span,
                )
            })?;
            let mut imports = BTreeMap::new();
            for import in &unit.imports {
                let qualifier = import
                    .path
                    .rsplit_once('/')
                    .map_or(import.path.as_str(), |(_, qualifier)| qualifier);
                imports.insert(qualifier.to_string(), import.path.clone());
            }
            contexts.insert(
                unit.span.source,
                FileContext {
                    package_path: package.path.clone(),
                    imports,
                },
            );
        }

        let linker = Self {
            project,
            graph,
            contexts,
        };
        linker.validate_declaration_names()?;
        linker.validate_method_receivers(program)?;
        linker.validate_public_api(program)?;
        Ok(linker)
    }

    fn link(&self, program: &Program) -> Result<Program, LinkError> {
        let mut linked = program.clone();

        for declaration in &mut linked.structs {
            let context = self.context(declaration.span)?;
            let type_params = declaration
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect::<BTreeSet<_>>();
            for field in &mut declaration.fields {
                self.link_type_ref(&mut field.ty, context, &type_params)?;
            }
            declaration.name = internal_symbol(&context.package_path, &declaration.name);
        }

        for declaration in &mut linked.enums {
            let context = self.context(declaration.span)?;
            let type_params = declaration
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect::<BTreeSet<_>>();
            for variant in &mut declaration.variants {
                if let Some(payload) = &mut variant.payload {
                    self.link_type_ref(payload, context, &type_params)?;
                }
            }
            declaration.name = internal_symbol(&context.package_path, &declaration.name);
        }

        for function in &mut linked.functions {
            let context = self.context(function.span)?;
            let type_params = self.function_type_params(function, context);
            if let Some(receiver) = &mut function.receiver {
                self.link_type_ref(&mut receiver.ty, context, &type_params)?;
            }
            for param in &mut function.params {
                self.link_type_ref(&mut param.ty, context, &type_params)?;
            }
            if let Some(return_type) = &mut function.return_type {
                self.link_type_ref(return_type, context, &type_params)?;
            }

            let mut scopes = vec![BTreeSet::new()];
            if let Some(receiver) = &function.receiver {
                scopes[0].insert(receiver.name.clone());
            }
            for param in &function.params {
                scopes[0].insert(param.name.clone());
            }
            self.link_block(
                &mut function.body,
                context,
                &mut scopes,
                &type_params,
                false,
            )?;

            if function.receiver.is_none() {
                function.name = self.function_symbol(&context.package_path, &function.name);
            }
        }

        Ok(linked)
    }

    fn context(&self, span: Span) -> Result<&FileContext, LinkError> {
        self.contexts.get(&span.source).ok_or_else(|| {
            LinkError::new(
                format!(
                    "source ID {} has no package link context",
                    span.source.index()
                ),
                span,
            )
        })
    }

    fn function_type_params(
        &self,
        function: &crate::ast::Function,
        context: &FileContext,
    ) -> BTreeSet<String> {
        let mut type_params = function
            .type_params
            .iter()
            .map(|param| param.name.clone())
            .collect::<BTreeSet<_>>();
        if let Some(receiver) = &function.receiver {
            let package = self
                .graph
                .package(&context.package_path)
                .expect("every link context has a package");
            if let Some(declaration) = package.declarations.get(&receiver.ty.name) {
                if declaration.kind == PackageDeclarationKind::Struct {
                    type_params.extend(declaration.type_params.iter().cloned());
                }
            }
        }
        type_params
    }

    fn validate_declaration_names(&self) -> Result<(), LinkError> {
        const BUILTIN_TYPES: &[&str] = &[
            "int", "bool", "string", "unit", "Option", "Result", "Array", "Slice",
        ];
        const BUILTIN_VALUES: &[&str] = &["print", "len", "append", "Some", "None", "Ok", "Err"];

        for package in self.graph.packages().values() {
            for declaration in package.declarations.values() {
                if BUILTIN_TYPES.contains(&declaration.name.as_str()) {
                    return Err(LinkError::new(
                        format!("`{}` is a built-in type name", declaration.name),
                        declaration.span,
                    ));
                }
                if BUILTIN_VALUES.contains(&declaration.name.as_str()) {
                    return Err(LinkError::new(
                        format!("`{}` is a reserved built-in value name", declaration.name),
                        declaration.span,
                    ));
                }
                if declaration.kind == PackageDeclarationKind::Function
                    && declaration.name == "main"
                    && package.path != self.project.name()
                {
                    return Err(LinkError::new(
                        "`main` may only be declared in the project root package",
                        declaration.span,
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_public_api(&self, program: &Program) -> Result<(), LinkError> {
        for declaration in &program.structs {
            if declaration.visibility != Visibility::Public {
                continue;
            }
            let context = self.context(declaration.span)?;
            let type_params = declaration
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect::<BTreeSet<_>>();
            for field in &declaration.fields {
                self.validate_public_type(&field.ty, context, &declaration.name, &type_params)?;
            }
        }

        for declaration in &program.enums {
            if declaration.visibility != Visibility::Public {
                continue;
            }
            let context = self.context(declaration.span)?;
            let type_params = declaration
                .type_params
                .iter()
                .map(|param| param.name.clone())
                .collect::<BTreeSet<_>>();
            for variant in &declaration.variants {
                if let Some(payload) = &variant.payload {
                    self.validate_public_type(payload, context, &declaration.name, &type_params)?;
                }
            }
        }

        for declaration in &program.functions {
            if declaration.visibility != Visibility::Public {
                continue;
            }
            let context = self.context(declaration.span)?;
            let type_params = self.function_type_params(declaration, context);
            if let Some(receiver) = &declaration.receiver {
                self.validate_public_type(&receiver.ty, context, &declaration.name, &type_params)?;
            }
            for param in &declaration.params {
                self.validate_public_type(&param.ty, context, &declaration.name, &type_params)?;
            }
            if let Some(return_type) = &declaration.return_type {
                self.validate_public_type(return_type, context, &declaration.name, &type_params)?;
            }
        }
        Ok(())
    }

    fn validate_method_receivers(&self, program: &Program) -> Result<(), LinkError> {
        for declaration in &program.functions {
            let Some(receiver) = &declaration.receiver else {
                continue;
            };
            let context = self.context(declaration.span)?;
            let package = self
                .graph
                .package(&context.package_path)
                .expect("every link context has a package");
            if receiver.ty.name.contains('.')
                || !package
                    .declarations
                    .get(&receiver.ty.name)
                    .is_some_and(|candidate| candidate.kind == PackageDeclarationKind::Struct)
            {
                return Err(LinkError::new(
                    "method receiver type must be declared in the same package",
                    receiver.ty.span,
                ));
            }
        }
        Ok(())
    }

    fn validate_public_type(
        &self,
        ty: &TypeRef,
        context: &FileContext,
        declaration_name: &str,
        type_params: &BTreeSet<String>,
    ) -> Result<(), LinkError> {
        if let Some(function) = &ty.function {
            for param in &function.params {
                self.validate_public_type(&param.ty, context, declaration_name, type_params)?;
            }
            self.validate_public_type(
                &function.return_type,
                context,
                declaration_name,
                type_params,
            )?;
            return Ok(());
        }
        for arg in &ty.args {
            self.validate_public_type(arg, context, declaration_name, type_params)?;
        }
        if ty.slice || ty.array_len.is_some() {
            return Ok(());
        }
        if type_params.contains(&ty.name) {
            return Ok(());
        }

        let referenced = if let Some((qualifier, name)) = ty.name.split_once('.') {
            Some(self.imported_declaration(context, qualifier, name, ty.span)?)
        } else {
            self.graph
                .package(&context.package_path)
                .expect("every link context has a package")
                .declarations
                .get(&ty.name)
        };

        if let Some(referenced) = referenced {
            if matches!(
                referenced.kind,
                PackageDeclarationKind::Struct | PackageDeclarationKind::Enum
            ) && referenced.visibility != Visibility::Public
            {
                return Err(LinkError::new(
                    format!(
                        "public declaration `{declaration_name}` exposes private type `{}`",
                        ty.name
                    ),
                    ty.span,
                ));
            }
        }
        Ok(())
    }

    fn link_type_ref(
        &self,
        ty: &mut TypeRef,
        context: &FileContext,
        type_params: &BTreeSet<String>,
    ) -> Result<(), LinkError> {
        if let Some(function) = &mut ty.function {
            for param in &mut function.params {
                self.link_type_ref(&mut param.ty, context, type_params)?;
            }
            self.link_type_ref(&mut function.return_type, context, type_params)?;
            return Ok(());
        }
        for arg in &mut ty.args {
            self.link_type_ref(arg, context, type_params)?;
        }
        if ty.slice || ty.array_len.is_some() {
            return Ok(());
        }
        if type_params.contains(&ty.name) {
            return Ok(());
        }

        if let Some((qualifier, name)) = ty.name.split_once('.') {
            if name.contains('.') {
                return Err(LinkError::new(
                    format!("invalid qualified type `{}`", ty.name),
                    ty.span,
                ));
            }
            let declaration = self.imported_declaration(context, qualifier, name, ty.span)?;
            self.require_type_kind(declaration, ty.span)?;
            self.require_public(context, qualifier, declaration, ty.span)?;
            let package_path = context
                .imports
                .get(qualifier)
                .expect("an imported declaration has a package path");
            ty.name = internal_symbol(package_path, name);
            return Ok(());
        }

        let package = self
            .graph
            .package(&context.package_path)
            .expect("every link context has a package");
        if package
            .declarations
            .get(&ty.name)
            .is_some_and(|declaration| {
                matches!(
                    declaration.kind,
                    PackageDeclarationKind::Struct | PackageDeclarationKind::Enum
                )
            })
        {
            ty.name = internal_symbol(&context.package_path, &ty.name);
        }
        Ok(())
    }

    fn link_block(
        &self,
        block: &mut Block,
        context: &FileContext,
        scopes: &mut Vec<BTreeSet<String>>,
        type_params: &BTreeSet<String>,
        push_scope: bool,
    ) -> Result<(), LinkError> {
        if push_scope {
            scopes.push(BTreeSet::new());
        }
        for statement in &mut block.statements {
            self.link_stmt(statement, context, scopes, type_params)?;
        }
        if push_scope {
            scopes.pop();
        }
        Ok(())
    }

    fn link_stmt(
        &self,
        statement: &mut Stmt,
        context: &FileContext,
        scopes: &mut Vec<BTreeSet<String>>,
        type_params: &BTreeSet<String>,
    ) -> Result<(), LinkError> {
        match &mut statement.kind {
            StmtKind::Let { name, expr, .. } => {
                self.link_expr(expr, context, scopes, type_params)?;
                scopes
                    .last_mut()
                    .expect("link scopes are never empty")
                    .insert(name.clone());
            }
            StmtKind::Assign { expr, .. } | StmtKind::Return { expr } => {
                self.link_expr(expr, context, scopes, type_params)?;
            }
            StmtKind::FieldAssign { base, expr, .. } => {
                self.link_expr(base, context, scopes, type_params)?;
                self.link_expr(expr, context, scopes, type_params)?;
            }
            StmtKind::IndexAssign { base, index, expr } => {
                self.link_expr(base, context, scopes, type_params)?;
                self.link_expr(index, context, scopes, type_params)?;
                self.link_expr(expr, context, scopes, type_params)?;
            }
            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.link_expr(condition, context, scopes, type_params)?;
                self.link_block(then_block, context, scopes, type_params, true)?;
                if let Some(else_block) = else_block {
                    self.link_block(else_block, context, scopes, type_params, true)?;
                }
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
            } => {
                let mut loop_scopes = scopes.clone();
                loop_scopes.push(BTreeSet::new());
                if let Some(ForInit::Let { name, expr, .. }) = init {
                    self.link_expr(expr, context, &loop_scopes, type_params)?;
                    loop_scopes
                        .last_mut()
                        .expect("loop link scope exists")
                        .insert(name.clone());
                }
                if let Some(condition) = condition {
                    self.link_expr(condition, context, &loop_scopes, type_params)?;
                }
                self.link_block(body, context, &mut loop_scopes, type_params, true)?;
                if let Some(ForPost::Assign { target, expr }) = post {
                    self.link_expr(target, context, &loop_scopes, type_params)?;
                    self.link_expr(expr, context, &loop_scopes, type_params)?;
                }
            }
            StmtKind::RangeFor {
                index_name,
                value_name,
                source,
                body,
            } => {
                self.link_expr(source, context, scopes, type_params)?;
                let mut range_scopes = scopes.clone();
                range_scopes.push(BTreeSet::from([index_name.clone(), value_name.clone()]));
                self.link_block(body, context, &mut range_scopes, type_params, false)?;
            }
            StmtKind::Match { scrutinee, arms } => {
                self.link_expr(scrutinee, context, scopes, type_params)?;
                for arm in arms {
                    let mut arm_scopes = scopes.clone();
                    arm_scopes.push(BTreeSet::new());
                    if let Some(binding) = pattern_binding(&arm.pattern) {
                        arm_scopes
                            .last_mut()
                            .expect("match arm link scope exists")
                            .insert(binding.to_string());
                    }
                    self.link_block(&mut arm.block, context, &mut arm_scopes, type_params, false)?;
                }
            }
            StmtKind::Expr { expr } => self.link_expr(expr, context, scopes, type_params)?,
            StmtKind::Break | StmtKind::Continue => {}
        }
        Ok(())
    }

    fn link_expr(
        &self,
        expr: &mut Expr,
        context: &FileContext,
        scopes: &[BTreeSet<String>],
        type_params: &BTreeSet<String>,
    ) -> Result<(), LinkError> {
        if enum_constructor_base(expr)
            .is_some_and(|base| self.is_enum_type_expr(base, context, scopes))
        {
            match &mut expr.kind {
                ExprKind::Call { callee, args } => {
                    let ExprKind::FieldAccess { base, .. } = &mut callee.kind else {
                        unreachable!("enum constructor target is a field access");
                    };
                    self.link_type_expr(base, context, type_params)?;
                    for arg in args {
                        self.link_expr(&mut arg.expr, context, scopes, type_params)?;
                    }
                }
                ExprKind::FieldAccess { base, .. } => {
                    self.link_type_expr(base, context, type_params)?;
                }
                _ => unreachable!("enum constructor has call or field-access syntax"),
            }
            return Ok(());
        }

        match &mut expr.kind {
            ExprKind::Int(_) | ExprKind::String(_) | ExprKind::Bool(_) | ExprKind::Nil => {}
            ExprKind::Var(name) => {
                if !type_params.contains(name) {
                    if let Some(symbol) = self.local_function_symbol(context, scopes, name) {
                        *name = symbol;
                    }
                }
            }
            ExprKind::FunctionLiteral(function) => {
                for param in function.params.iter_mut() {
                    self.link_type_ref(&mut param.ty, context, type_params)?;
                }
                if let Some(return_type) = &mut function.return_type {
                    self.link_type_ref(return_type, context, type_params)?;
                }
                let mut closure_scopes = scopes.to_vec();
                closure_scopes.push(
                    function
                        .params
                        .iter()
                        .map(|param| param.name.clone())
                        .collect(),
                );
                self.link_block(
                    &mut function.body,
                    context,
                    &mut closure_scopes,
                    type_params,
                    false,
                )?;
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.link_expr(condition, context, scopes, type_params)?;
                self.link_expr(then_branch, context, scopes, type_params)?;
                self.link_expr(else_branch, context, scopes, type_params)?;
            }
            ExprKind::Match { scrutinee, arms } => {
                self.link_expr(scrutinee, context, scopes, type_params)?;
                for arm in arms {
                    let mut arm_scopes = scopes.to_vec();
                    arm_scopes.push(BTreeSet::new());
                    if let Some(binding) = pattern_binding(&arm.pattern) {
                        arm_scopes
                            .last_mut()
                            .expect("match expression link scope exists")
                            .insert(binding.to_string());
                    }
                    self.link_expr(&mut arm.expr, context, &arm_scopes, type_params)?;
                }
            }
            ExprKind::StructLiteral {
                type_name,
                type_args,
                fields,
            } => {
                *type_name =
                    self.resolve_struct_name(context, type_name, expr.span, type_params)?;
                for type_arg in type_args {
                    self.link_type_ref(type_arg, context, type_params)?;
                }
                for field in fields {
                    self.link_expr(&mut field.expr, context, scopes, type_params)?;
                }
            }
            ExprKind::ArrayLiteral { ty, elements } => {
                self.link_type_ref(ty, context, type_params)?;
                for element in elements {
                    self.link_expr(element, context, scopes, type_params)?;
                }
            }
            ExprKind::FieldAccess { base, field } => {
                if let ExprKind::Var(qualifier) = &base.kind {
                    if !is_bound(scopes, qualifier) && context.imports.contains_key(qualifier) {
                        let symbol =
                            self.imported_function_symbol(context, qualifier, field, expr.span)?;
                        expr.kind = ExprKind::Var(symbol);
                        return Ok(());
                    }
                }
                self.link_expr(base, context, scopes, type_params)?;
            }
            ExprKind::Index { base, index } => {
                if self.is_generic_function_expr(base, context, scopes) {
                    self.link_expr(base, context, scopes, type_params)?;
                    self.link_type_expr(index, context, type_params)?;
                    return Ok(());
                }
                self.link_expr(base, context, scopes, type_params)?;
                self.link_expr(index, context, scopes, type_params)?;
            }
            ExprKind::TypeApply { base, args } => {
                self.link_expr(base, context, scopes, type_params)?;
                for arg in args {
                    self.link_type_ref(arg, context, type_params)?;
                }
            }
            ExprKind::EnumConstructor {
                enum_name, args, ..
            } => {
                let mut ty = TypeRef {
                    name: enum_name.clone(),
                    args: Vec::new(),
                    array_len: None,
                    slice: false,
                    function: None,
                    span: expr.span,
                };
                self.link_type_ref(&mut ty, context, type_params)?;
                *enum_name = ty.name;
                if let Some(args) = args {
                    for arg in args {
                        self.link_expr(&mut arg.expr, context, scopes, type_params)?;
                    }
                }
            }
            ExprKind::Call { callee, args } => {
                let package_call = match &callee.kind {
                    ExprKind::FieldAccess { base, field } => match &base.kind {
                        ExprKind::Var(qualifier)
                            if !is_bound(scopes, qualifier)
                                && context.imports.contains_key(qualifier) =>
                        {
                            Some((qualifier.clone(), field.clone()))
                        }
                        _ => None,
                    },
                    _ => None,
                };

                if let Some((qualifier, name)) = package_call {
                    let declaration =
                        self.imported_declaration(context, &qualifier, &name, callee.span)?;
                    self.require_kind(
                        declaration,
                        PackageDeclarationKind::Function,
                        "function",
                        callee.span,
                    )?;
                    self.require_public(context, &qualifier, declaration, callee.span)?;
                    let package_path = context
                        .imports
                        .get(&qualifier)
                        .expect("an imported declaration has a package path");
                    callee.kind = ExprKind::Var(self.function_symbol(package_path, &name));
                } else {
                    if let ExprKind::Var(name) = &mut callee.kind {
                        if let Some(symbol) = self.local_function_symbol(context, scopes, name) {
                            *name = symbol;
                        }
                    }
                    self.link_expr(callee, context, scopes, type_params)?;
                }
                for arg in args {
                    self.link_expr(&mut arg.expr, context, scopes, type_params)?;
                }
            }
            ExprKind::Unary { expr, .. } => self.link_expr(expr, context, scopes, type_params)?,
            ExprKind::Binary { left, right, .. } => {
                self.link_expr(left, context, scopes, type_params)?;
                self.link_expr(right, context, scopes, type_params)?;
            }
        }
        Ok(())
    }

    fn link_type_expr(
        &self,
        expr: &mut Expr,
        context: &FileContext,
        type_params: &BTreeSet<String>,
    ) -> Result<(), LinkError> {
        match &mut expr.kind {
            ExprKind::Var(name) => {
                if type_params.contains(name) {
                    return Ok(());
                }
                let package = self
                    .graph
                    .package(&context.package_path)
                    .expect("every link context has a package");
                if package.declarations.get(name).is_some_and(|declaration| {
                    matches!(
                        declaration.kind,
                        PackageDeclarationKind::Struct | PackageDeclarationKind::Enum
                    )
                }) {
                    *name = internal_symbol(&context.package_path, name);
                }
                Ok(())
            }
            ExprKind::FieldAccess { base, field } => {
                if let ExprKind::Var(qualifier) = &base.kind {
                    if context.imports.contains_key(qualifier) {
                        let declaration =
                            self.imported_declaration(context, qualifier, field, expr.span)?;
                        self.require_type_kind(declaration, expr.span)?;
                        self.require_public(context, qualifier, declaration, expr.span)?;
                        let package_path = context
                            .imports
                            .get(qualifier)
                            .expect("an imported declaration has a package path");
                        expr.kind = ExprKind::Var(internal_symbol(package_path, field));
                        return Ok(());
                    }
                }
                Err(LinkError::new(
                    "type argument must use a known type name",
                    expr.span,
                ))
            }
            ExprKind::Index { base, index } => {
                self.link_type_expr(base, context, type_params)?;
                self.link_type_expr(index, context, type_params)
            }
            ExprKind::TypeApply { base, args } => {
                self.link_type_expr(base, context, type_params)?;
                for arg in args {
                    self.link_type_ref(arg, context, type_params)?;
                }
                Ok(())
            }
            _ => Err(LinkError::new("type argument must be a type", expr.span)),
        }
    }

    fn is_generic_function_expr(
        &self,
        expr: &Expr,
        context: &FileContext,
        scopes: &[BTreeSet<String>],
    ) -> bool {
        match &expr.kind {
            ExprKind::Var(name) if !is_bound(scopes, name) => self
                .graph
                .package(&context.package_path)
                .and_then(|package| package.declarations.get(name))
                .is_some_and(|declaration| {
                    declaration.kind == PackageDeclarationKind::Function
                        && !declaration.type_params.is_empty()
                }),
            ExprKind::FieldAccess { base, field } => {
                let ExprKind::Var(qualifier) = &base.kind else {
                    return false;
                };
                if is_bound(scopes, qualifier) {
                    return false;
                }
                context
                    .imports
                    .get(qualifier)
                    .and_then(|path| self.graph.package(path))
                    .and_then(|package| package.declarations.get(field))
                    .is_some_and(|declaration| {
                        declaration.kind == PackageDeclarationKind::Function
                            && !declaration.type_params.is_empty()
                    })
            }
            _ => false,
        }
    }

    fn is_enum_type_expr(
        &self,
        expr: &Expr,
        context: &FileContext,
        scopes: &[BTreeSet<String>],
    ) -> bool {
        match &expr.kind {
            ExprKind::Index { base, .. } | ExprKind::TypeApply { base, .. } => {
                self.is_enum_type_expr(base, context, scopes)
            }
            ExprKind::Var(name) if !is_bound(scopes, name) => self
                .graph
                .package(&context.package_path)
                .and_then(|package| package.declarations.get(name))
                .is_some_and(|declaration| declaration.kind == PackageDeclarationKind::Enum),
            ExprKind::FieldAccess { base, field } => {
                let ExprKind::Var(qualifier) = &base.kind else {
                    return false;
                };
                if is_bound(scopes, qualifier) {
                    return false;
                }
                context
                    .imports
                    .get(qualifier)
                    .and_then(|path| self.graph.package(path))
                    .and_then(|package| package.declarations.get(field))
                    .is_some_and(|declaration| declaration.kind == PackageDeclarationKind::Enum)
            }
            _ => false,
        }
    }

    fn resolve_struct_name(
        &self,
        context: &FileContext,
        source_name: &str,
        span: Span,
        type_params: &BTreeSet<String>,
    ) -> Result<String, LinkError> {
        if type_params.contains(source_name) {
            return Ok(source_name.to_string());
        }
        if let Some((qualifier, name)) = source_name.split_once('.') {
            let declaration = self.imported_declaration(context, qualifier, name, span)?;
            self.require_kind(declaration, PackageDeclarationKind::Struct, "struct", span)?;
            self.require_public(context, qualifier, declaration, span)?;
            let package_path = context
                .imports
                .get(qualifier)
                .expect("an imported declaration has a package path");
            return Ok(internal_symbol(package_path, name));
        }

        let package = self
            .graph
            .package(&context.package_path)
            .expect("every link context has a package");
        if package
            .declarations
            .get(source_name)
            .is_some_and(|declaration| declaration.kind == PackageDeclarationKind::Struct)
        {
            Ok(internal_symbol(&context.package_path, source_name))
        } else {
            Ok(source_name.to_string())
        }
    }

    fn imported_declaration<'b>(
        &'b self,
        context: &FileContext,
        qualifier: &str,
        name: &str,
        span: Span,
    ) -> Result<&'b PackageDeclaration, LinkError> {
        let package_path = context.imports.get(qualifier).ok_or_else(|| {
            LinkError::new(format!("unknown import qualifier `{qualifier}`"), span)
        })?;
        let package = self
            .graph
            .package(package_path)
            .expect("package graph validation resolved every import");
        package.declarations.get(name).ok_or_else(|| {
            LinkError::new(
                format!("package `{package_path}` has no declaration `{name}`"),
                span,
            )
        })
    }

    fn imported_function_symbol(
        &self,
        context: &FileContext,
        qualifier: &str,
        name: &str,
        span: Span,
    ) -> Result<String, LinkError> {
        let declaration = self.imported_declaration(context, qualifier, name, span)?;
        self.require_kind(
            declaration,
            PackageDeclarationKind::Function,
            "function",
            span,
        )?;
        self.require_public(context, qualifier, declaration, span)?;
        let package_path = context
            .imports
            .get(qualifier)
            .expect("an imported declaration has a package path");
        Ok(self.function_symbol(package_path, name))
    }

    fn local_function_symbol(
        &self,
        context: &FileContext,
        scopes: &[BTreeSet<String>],
        name: &str,
    ) -> Option<String> {
        if is_bound(scopes, name) {
            return None;
        }
        let package = self
            .graph
            .package(&context.package_path)
            .expect("every link context has a package");
        package
            .declarations
            .get(name)
            .is_some_and(|declaration| declaration.kind == PackageDeclarationKind::Function)
            .then(|| self.function_symbol(&context.package_path, name))
    }

    fn require_kind(
        &self,
        declaration: &PackageDeclaration,
        expected: PackageDeclarationKind,
        expected_name: &str,
        span: Span,
    ) -> Result<(), LinkError> {
        if declaration.kind != expected {
            return Err(LinkError::new(
                format!(
                    "`{}` is not a {expected_name} declaration",
                    declaration.name
                ),
                span,
            ));
        }
        Ok(())
    }

    fn require_type_kind(
        &self,
        declaration: &PackageDeclaration,
        span: Span,
    ) -> Result<(), LinkError> {
        if !matches!(
            declaration.kind,
            PackageDeclarationKind::Struct | PackageDeclarationKind::Enum
        ) {
            return Err(LinkError::new(
                format!("`{}` is not a type declaration", declaration.name),
                span,
            ));
        }
        Ok(())
    }

    fn require_public(
        &self,
        context: &FileContext,
        qualifier: &str,
        declaration: &PackageDeclaration,
        span: Span,
    ) -> Result<(), LinkError> {
        if declaration.visibility != Visibility::Public {
            let package_path = context
                .imports
                .get(qualifier)
                .expect("an imported declaration has a package path");
            return Err(LinkError::new(
                format!(
                    "`{qualifier}.{}` is private to package `{package_path}`",
                    declaration.name
                ),
                span,
            ));
        }
        Ok(())
    }

    fn function_symbol(&self, package_path: &str, name: &str) -> String {
        if package_path == self.project.name() && name == "main" {
            "main".to_string()
        } else {
            internal_symbol(package_path, name)
        }
    }
}

fn internal_symbol(package_path: &str, name: &str) -> String {
    let mut encoded = String::with_capacity(package_path.len() * 2);
    for byte in package_path.bytes() {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    format!("__mlg_pkg_{encoded}_{name}")
}

fn is_bound(scopes: &[BTreeSet<String>], name: &str) -> bool {
    scopes.iter().rev().any(|scope| scope.contains(name))
}

fn enum_constructor_base(expr: &Expr) -> Option<&Expr> {
    let target = match &expr.kind {
        ExprKind::Call { callee, .. } => callee.as_ref(),
        ExprKind::FieldAccess { .. } => expr,
        _ => return None,
    };
    let ExprKind::FieldAccess { base, .. } = &target.kind else {
        return None;
    };
    Some(base)
}

fn pattern_binding(pattern: &MatchPattern) -> Option<&str> {
    match pattern {
        MatchPattern::Some(binding) | MatchPattern::Ok(binding) | MatchPattern::Err(binding) => {
            Some(binding)
        }
        MatchPattern::Binding(binding) => Some(binding),
        MatchPattern::Variant { payload, .. } => payload.as_deref().and_then(pattern_binding),
        MatchPattern::NestedBuiltin { payload, .. } => pattern_binding(payload),
        MatchPattern::None | MatchPattern::Wildcard => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use crate::{
        build_package_graph, check_project, discover_project, generate_c_from_ir,
        load_source_files, lower, parse_sources, PackageError, SourceSet,
    };

    use super::*;

    #[test]
    fn restores_internal_symbols_for_user_facing_diagnostics() {
        let message = format!(
            "expected `{}`, found `{}`",
            internal_symbol("hello/greet", "Message"),
            internal_symbol("hello", "User")
        );

        assert_eq!(
            display_linked_message(&message),
            "expected `hello/greet.Message`, found `hello.User`"
        );
        assert_eq!(
            display_linked_message("plain diagnostic"),
            "plain diagnostic"
        );
    }

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(label: &str) -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "mallang-linker-test-{}-{label}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            let root = fs::canonicalize(root).unwrap();
            let project = Self { root };
            project.write("mallang.toml", "[project]\nname = \"hello\"\n");
            project
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }

        fn parse(&self) -> Result<(Project, SourceSet, Program, PackageGraph), PackageError> {
            let project = discover_project(&self.root).unwrap();
            let loaded = load_source_files(project.source_files().iter().cloned()).unwrap();
            let program = parse_sources(&loaded.sources, &loaded.source_ids).unwrap();
            let graph = build_package_graph(&project, &loaded.sources, &program)?;
            Ok((project, loaded, program, graph))
        }

        fn link(&self) -> Result<(Project, SourceSet, Program, PackageGraph), LinkError> {
            let (project, loaded, program, graph) = self.parse().unwrap();
            let linked = link_project(&project, &graph, &program)?;
            Ok((project, loaded, linked, graph))
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn links_public_functions_structs_and_methods_through_native_ir() {
        let project = TempProject::new("public-api");
        project.write(
            "src/main.mlg",
            r#"package main
import "hello/greet"

func main() {
    message := greet.New("hello")
    message.Print()
}
"#,
        );
        project.write(
            "src/greet/greet.mlg",
            r#"package greet

pub type Message struct {
    text string
}

pub func New(text string) Message {
    return Message{text: text}
}

pub func (con self Message) Print() {
    print(self.text)
}
"#,
        );

        let (_, _, linked, graph) = project.link().unwrap();
        let checked = check_project(&linked, &graph).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert!(linked
            .functions
            .iter()
            .any(|function| function.name == "main"));
        assert!(linked
            .structs
            .iter()
            .any(|declaration| declaration.name.starts_with("__mlg_pkg_")));
        assert!(c.contains("int main(void)"));
        assert!(c.contains("hello"));
    }

    #[test]
    fn links_cross_package_generic_types_functions_and_receivers() {
        let project = TempProject::new("generic-api");
        project.write(
            "src/main.mlg",
            r#"package main
import "hello/greet"

func main() {
    print(greet.Identity[int](7))
    wrapped := greet.Identity[greet.Box[int]](greet.Box[int]{value: 8})
    print(wrapped.value)
    mut box := greet.Box[string]{value: "before"}
    box.Replace("after")
    print(box.value)
}
"#,
        );
        project.write(
            "src/greet/greet.mlg",
            r#"package greet

type T struct { hidden int }

pub type Box[T] struct { value T }

pub func Identity[T](value T) T { return value }

pub func (mut box Box[T]) Replace(value T) {
    box.value = value
}
"#,
        );

        let (_, _, linked, graph) = project.link().unwrap();
        assert!(linked.structs.iter().any(|declaration| {
            declaration.name.starts_with("__mlg_pkg_") && !declaration.type_params.is_empty()
        }));
        assert!(linked.functions.iter().any(|function| {
            function.name.starts_with("__mlg_pkg_") && !function.type_params.is_empty()
        }));

        let checked = check_project(&linked, &graph).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();
        assert!(c.contains("before"));
        assert!(c.contains("after"));
    }

    #[test]
    fn links_cross_package_generic_enum_constructors() {
        let project = TempProject::new("generic-enum-api");
        project.write(
            "src/main.mlg",
            r#"package main
import "hello/model"

func main() {
    value := model.Maybe[int].Some(7)
    empty := model.Maybe[string].None
}
"#,
        );
        project.write(
            "src/model/model.mlg",
            r#"package model

pub type Maybe[T] enum {
    None
    Some(T)
}
"#,
        );

        let (_, _, linked, graph) = project.link().unwrap();
        let checked = check_project(&linked, &graph).unwrap();
        assert_eq!(checked.enums.len(), 2);
        assert!(checked
            .enums
            .keys()
            .all(|name| name.starts_with("__mlg_spec_enum_")));
    }

    #[test]
    fn links_package_function_values_and_closure_returns() {
        let project = TempProject::new("function-values");
        project.write(
            "src/main.mlg",
            r#"package main
import "hello/greet"

func main() {
    transform := greet.Double
    print(greet.Apply(21, transform))
    selected := greet.Select()
    print(selected(11))
    add := greet.MakeAdder(10)
    print(add(5))
}
"#,
        );
        project.write(
            "src/greet/greet.mlg",
            r#"package greet

pub func Double(value int) int {
    return value * 2
}

pub func Select() func(int) int {
    return Double
}

pub func Apply(value int, transform func(int) int) int {
    return transform(value)
}

pub func MakeAdder(offset int) func(int) int {
    return func(value int) int {
        return value + offset
    }
}
"#,
        );

        let (_, _, linked, graph) = project.link().unwrap();
        let checked = check_project(&linked, &graph).unwrap();
        let ir = lower(&checked).unwrap();
        let c = generate_c_from_ir(&ir).unwrap();

        assert_eq!(ir.closures.len(), 1);
        assert!(c.contains("mallang_callable_thunk_mlg___mlg_pkg_"));
        assert!(c.contains("closure environment allocation failed"));
    }

    #[test]
    fn rejects_private_imported_functions_and_types() {
        let private_function = TempProject::new("private-function");
        private_function.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { greet.Secret() }\n",
        );
        private_function.write("src/greet/greet.mlg", "package greet\nfunc Secret() {}\n");
        let function_error = private_function.link().unwrap_err();
        assert_eq!(
            function_error.message,
            "`greet.Secret` is private to package `hello/greet`"
        );

        let private_function_value = TempProject::new("private-function-value");
        private_function_value.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { secret := greet.Secret print(secret) }\n",
        );
        private_function_value.write(
            "src/greet/greet.mlg",
            "package greet\nfunc Secret() int { return 1 }\n",
        );
        let function_value_error = private_function_value.link().unwrap_err();
        assert_eq!(
            function_value_error.message,
            "`greet.Secret` is private to package `hello/greet`"
        );

        let non_function_value = TempProject::new("non-function-value");
        non_function_value.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { value := greet.Message print(value) }\n",
        );
        non_function_value.write(
            "src/greet/greet.mlg",
            "package greet\npub type Message struct {}\n",
        );
        let non_function_error = non_function_value.link().unwrap_err();
        assert_eq!(
            non_function_error.message,
            "`Message` is not a function declaration"
        );

        let private_type = TempProject::new("private-type");
        private_type.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc use(value greet.Message) {}\nfunc main() {}\n",
        );
        private_type.write(
            "src/greet/greet.mlg",
            "package greet\ntype Message struct {}\n",
        );
        let type_error = private_type.link().unwrap_err();
        assert_eq!(
            type_error.message,
            "`greet.Message` is private to package `hello/greet`"
        );

        let private_enum = TempProject::new("private-enum");
        private_enum.write(
            "src/main.mlg",
            "package main\nimport \"hello/model\"\nfunc main() { value := model.Maybe[int].Some(7) }\n",
        );
        private_enum.write(
            "src/model/model.mlg",
            "package model\ntype Maybe[T] enum { None Some(T) }\n",
        );
        let enum_error = private_enum.link().unwrap_err();
        assert_eq!(
            enum_error.message,
            "`model.Maybe` is private to package `hello/model`"
        );
    }

    #[test]
    fn rejects_private_methods_across_packages() {
        let project = TempProject::new("private-method");
        project.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc main() { value := greet.New() value.Secret() }\n",
        );
        project.write(
            "src/greet/greet.mlg",
            "package greet\npub type Message struct {}\npub func New() Message { return Message{} }\nfunc (con self Message) Secret() {}\n",
        );

        let (_, _, linked, graph) = project.link().unwrap();
        let error = check_project(&linked, &graph).unwrap_err();

        assert_eq!(
            error.message,
            "method `Secret` is private to package `hello/greet`"
        );
    }

    #[test]
    fn allows_same_names_in_different_packages() {
        let project = TempProject::new("same-names");
        project.write(
            "src/main.mlg",
            "package main\nimport \"hello/a\"\nimport \"hello/b\"\nfunc main() { a.Open() b.Open() }\n",
        );
        project.write("src/a/a.mlg", "package a\npub func Open() {}\n");
        project.write("src/b/b.mlg", "package b\npub func Open() {}\n");

        let (_, _, linked, graph) = project.link().unwrap();

        check_project(&linked, &graph).unwrap();
        let linked_names = linked
            .functions
            .iter()
            .filter(|function| function.name != "main")
            .map(|function| function.name.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(linked_names.len(), 2);
    }

    #[test]
    fn respects_local_shadowing_of_an_import_qualifier() {
        let project = TempProject::new("qualifier-shadow");
        project.write(
            "src/main.mlg",
            r#"package main
import "hello/greet"

type Local struct {
    Identity []int
}

func (con self Local) Print() {}

func main() {
    greet := Local{Identity: []int{7}}
    greet.Print()
    print(greet.Identity[0])
}
"#,
        );
        project.write(
            "src/greet/greet.mlg",
            "package greet\npub func Print() {}\npub func Identity[T](value T) T { return value }\n",
        );

        let (_, _, linked, graph) = project.link().unwrap();

        check_project(&linked, &graph).unwrap();
    }

    #[test]
    fn preserves_project_builtin_and_entrypoint_name_rules() {
        let builtin = TempProject::new("builtin-name");
        builtin.write(
            "src/main.mlg",
            "package main\nfunc len() {}\nfunc main() {}\n",
        );
        let builtin_error = builtin.link().unwrap_err();
        assert_eq!(
            builtin_error.message,
            "`len` is a reserved built-in value name"
        );

        let nested_main = TempProject::new("nested-main");
        nested_main.write("src/main.mlg", "package main\nfunc main() {}\n");
        nested_main.write("src/worker/worker.mlg", "package worker\nfunc main() {}\n");
        let main_error = nested_main.link().unwrap_err();
        assert_eq!(
            main_error.message,
            "`main` may only be declared in the project root package"
        );
    }

    #[test]
    fn rejects_private_types_exposed_by_public_apis() {
        let function = TempProject::new("private-return");
        function.write("src/main.mlg", "package main\nfunc main() {}\n");
        function.write(
            "src/greet/greet.mlg",
            "package greet\ntype Message struct {}\npub func New() Message { return Message{} }\n",
        );
        let function_error = function.link().unwrap_err();
        assert_eq!(
            function_error.message,
            "public declaration `New` exposes private type `Message`"
        );

        let structure = TempProject::new("private-field");
        structure.write("src/main.mlg", "package main\nfunc main() {}\n");
        structure.write(
            "src/model/model.mlg",
            "package model\ntype Detail struct {}\npub type Record struct { detail Detail }\n",
        );
        let structure_error = structure.link().unwrap_err();
        assert_eq!(
            structure_error.message,
            "public declaration `Record` exposes private type `Detail`"
        );

        let function_type = TempProject::new("private-function-type");
        function_type.write("src/main.mlg", "package main\nfunc main() {}\n");
        function_type.write(
            "src/worker/worker.mlg",
            "package worker\ntype Secret struct {}\npub func Apply(transform func(Secret) int) int { return 0 }\n",
        );
        let function_type_error = function_type.link().unwrap_err();
        assert_eq!(
            function_type_error.message,
            "public declaration `Apply` exposes private type `Secret`"
        );

        let enumeration = TempProject::new("private-enum-payload");
        enumeration.write("src/main.mlg", "package main\nfunc main() {}\n");
        enumeration.write(
            "src/model/model.mlg",
            "package model\ntype Detail struct {}\npub type State enum { Empty Full(Detail) }\n",
        );
        let enum_error = enumeration.link().unwrap_err();
        assert_eq!(
            enum_error.message,
            "public declaration `State` exposes private type `Detail`"
        );
    }

    #[test]
    fn rejects_methods_on_imported_receiver_types() {
        let project = TempProject::new("foreign-receiver");
        project.write(
            "src/main.mlg",
            "package main\nimport \"hello/greet\"\nfunc (con self greet.Message) Extend() {}\nfunc main() {}\n",
        );
        project.write(
            "src/greet/greet.mlg",
            "package greet\npub type Message struct {}\n",
        );

        let error = project.link().unwrap_err();

        assert_eq!(
            error.message,
            "method receiver type must be declared in the same package"
        );
    }
}
