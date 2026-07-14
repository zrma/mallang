use std::{collections::HashMap, fmt};

use crate::{
    ast::{
        Block, Expr, ExprKind, ForInit, ForPost, Function, FunctionLiteral, MatchArm,
        MatchBlockArm, Program, Stmt, StmtKind, StructDecl, TypeRef,
    },
    token::Span,
};

const MAX_SPECIALIZATIONS: usize = 1024;

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
    Specializer::new(program)?.run(program)
}

struct Specializer {
    generic_structs: HashMap<String, StructDecl>,
    generic_functions: HashMap<String, Function>,
    struct_specializations: HashMap<String, String>,
    function_specializations: HashMap<String, String>,
    active_structs: HashMap<String, String>,
    active_functions: HashMap<String, String>,
    generated_structs: Vec<StructDecl>,
    generated_functions: Vec<Function>,
}

impl Specializer {
    fn new(program: &Program) -> Result<Self, SpecializationError> {
        let mut generic_structs = HashMap::new();
        let mut generic_functions = HashMap::new();

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

        for function in &program.functions {
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
            generic_functions,
            struct_specializations: HashMap::new(),
            function_specializations: HashMap::new(),
            active_structs: HashMap::new(),
            active_functions: HashMap::new(),
            generated_structs: Vec::new(),
            generated_functions: Vec::new(),
        })
    }

    fn run(mut self, program: &Program) -> Result<Program, SpecializationError> {
        let mut output = program.clone();
        output.structs.clear();
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

        for mut function in program
            .functions
            .iter()
            .filter(|function| function.type_params.is_empty())
            .cloned()
        {
            self.rewrite_function(&mut function, &HashMap::new())?;
            output.functions.push(function);
        }

        output.structs.append(&mut self.generated_structs);
        output.functions.append(&mut self.generated_functions);
        Ok(output)
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

        self.active_structs.remove(name);
        self.generated_structs.push(specialized);
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
        if self.struct_specializations.len() + self.function_specializations.len()
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
}
