use std::{collections::HashMap, fmt};

use crate::{
    ast::{
        Arg, ArgMode, BinaryOp, Block, Expr, ExprKind, ForInit, ForPost, Function, MatchArm,
        MatchBlockArm, MatchPattern, ParamMode, Program, Stmt, StmtKind, TypeRef, UnaryOp,
    },
    token::Span,
};

pub fn check(program: &Program) -> Result<CheckedProgram<'_>, SemanticError> {
    Checker::new(program).check()
}

#[derive(Debug, Clone)]
pub struct CheckedProgram<'a> {
    pub program: &'a Program,
    pub signatures: HashMap<&'a str, FunctionSig>,
    pub methods: HashMap<MethodKey, MethodSig>,
    pub structs: HashMap<&'a str, StructSig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSig {
    pub return_type: Type,
    pub params: Vec<ParamSig>,
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
    Struct(String),
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
            Self::Struct(name) => name.clone(),
        }
    }

    pub fn is_copy(&self) -> bool {
        match self {
            Self::Int | Self::Bool | Self::Unit => true,
            Self::String => false,
            Self::Option(inner) => inner.is_copy(),
            Self::Result(ok, err) => ok.is_copy() && err.is_copy(),
            Self::Array { .. } => false,
            Self::Struct(_) => false,
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
    signatures: HashMap<&'a str, FunctionSig>,
    methods: HashMap<MethodKey, MethodSig>,
    structs: HashMap<&'a str, StructSig>,
}

impl<'a> Checker<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            program,
            signatures: HashMap::new(),
            methods: HashMap::new(),
            structs: HashMap::new(),
        }
    }

    fn check(mut self) -> Result<CheckedProgram<'a>, SemanticError> {
        self.collect_structs()?;
        self.collect_signatures()?;
        for function in &self.program.functions {
            self.check_function(function)?;
        }

        Ok(CheckedProgram {
            program: self.program,
            signatures: self.signatures,
            methods: self.methods,
            structs: self.structs,
        })
    }

    fn collect_structs(&mut self) -> Result<(), SemanticError> {
        for struct_decl in &self.program.structs {
            if is_builtin_type_name(&struct_decl.name) {
                return Err(SemanticError::new(
                    format!("`{}` is a built-in type name", struct_decl.name),
                    struct_decl.span,
                ));
            }
            if self.structs.contains_key(struct_decl.name.as_str()) {
                return Err(SemanticError::new(
                    format!("duplicate struct `{}`", struct_decl.name),
                    struct_decl.span,
                ));
            }
            self.structs
                .insert(struct_decl.name.as_str(), StructSig { fields: Vec::new() });
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
                });
            }

            self.structs
                .insert(struct_decl.name.as_str(), StructSig { fields });
        }

        Ok(())
    }

    fn collect_signatures(&mut self) -> Result<(), SemanticError> {
        for function in &self.program.functions {
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
                if self.signatures.contains_key(function.name.as_str()) {
                    return Err(SemanticError::new(
                        format!("duplicate function `{}`", function.name),
                        function.span,
                    ));
                }
                self.signatures.insert(function.name.as_str(), function_sig);
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
            self.check_block_statements(&function.body, &mut locals, &sig.return_type, 0)?;

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
    ) -> Result<bool, SemanticError> {
        match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                expr,
            } => {
                self.check_let_binding(*mutable, name, expr, locals, stmt.span)?;
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
                )?;
                let mut else_locals = locals.clone();
                let else_returns = if let Some(else_block) = else_block {
                    self.check_block_statements(
                        else_block,
                        &mut else_locals,
                        return_type,
                        loop_depth,
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
                if let Some(init) = init {
                    self.check_for_init(init, &mut loop_locals, stmt.span)?;
                }

                if let Some(condition) = condition {
                    let condition_ty =
                        self.check_expr(condition, &mut loop_locals, ValueUse::Owned)?;
                    if condition_ty != Type::Bool {
                        return Err(SemanticError::new(
                            "for condition must have type `bool`",
                            condition.span,
                        ));
                    }
                }

                let mut body_locals = loop_locals.clone();
                self.check_block_statements(body, &mut body_locals, return_type, loop_depth + 1)?;
                let mut post_locals = loop_locals.clone();
                merge_loop_body_moves(&mut post_locals, &body_locals);
                if let Some(post) = post {
                    self.check_for_post(post, &mut post_locals, stmt.span)?;
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
            StmtKind::Match { scrutinee, arms } => {
                self.check_match_stmt(scrutinee, arms, locals, return_type, loop_depth, stmt.span)
            }
            StmtKind::Expr { expr } => {
                self.check_expr(expr, locals, ValueUse::Owned)?;
                Ok(false)
            }
        }
    }

    fn check_let_binding(
        &self,
        mutable: bool,
        name: &str,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<(), SemanticError> {
        let ty = self.check_expr(expr, locals, ValueUse::Owned)?;
        if locals
            .insert(
                name.to_string(),
                Local {
                    ty: ty.clone(),
                    mutable,
                    borrowed: false,
                    moved: false,
                },
            )
            .is_some()
        {
            return Err(SemanticError::new(
                format!("binding `{name}` already exists in this block"),
                span,
            ));
        }
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
        Ok(())
    }

    fn check_for_init(
        &self,
        init: &ForInit,
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<(), SemanticError> {
        match init {
            ForInit::Let {
                mutable,
                name,
                expr,
            } => self.check_let_binding(*mutable, name, expr, locals, span),
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
    ) -> Result<bool, SemanticError> {
        let mut returns = false;
        for stmt in &block.statements {
            returns |= self.check_stmt(stmt, locals, return_type, loop_depth)?;
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
            ExprKind::StructLiteral { type_name, fields } => {
                self.check_struct_literal(type_name, fields, locals, expected, expr.span)
            }
            ExprKind::ArrayLiteral { ty, elements } => {
                self.check_array_literal(ty, elements, locals, expected, expr.span)
            }
            ExprKind::FieldAccess { base, field } => {
                self.check_field_access(base, field, locals, value_use, expr.span)
            }
            ExprKind::Index { base, index } => {
                self.check_index_access(base, index, locals, expr.span)
            }
            ExprKind::Call { callee, args } => {
                self.check_call(callee, args, locals, expected, expr.span)
            }
            ExprKind::Unary { op, expr } => {
                let ty = self.check_expr(expr, locals, ValueUse::Owned)?;
                match (*op, &ty) {
                    (UnaryOp::Negate, Type::Int) => Ok(Type::Int),
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
        scrutinee: &Expr,
        arms: &[MatchBlockArm],
        locals: &mut HashMap<String, Local>,
        return_type: &Type,
        loop_depth: usize,
        span: Span,
    ) -> Result<bool, SemanticError> {
        if arms.is_empty() {
            return Err(SemanticError::new("match requires at least one arm", span));
        }

        let scrutinee_ty = self.check_expr(scrutinee, locals, ValueUse::Owned)?;
        let prepared_arms = self.prepare_match_block_arms(&scrutinee_ty, arms, span)?;
        let mut checks = Vec::new();
        for arm in &prepared_arms {
            checks.push(self.check_prepared_match_block_arm(
                arm,
                locals,
                return_type,
                loop_depth,
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

        let Type::Array { len, element } = &array_ty else {
            return Err(SemanticError::new(
                "array literal requires a fixed-size array type",
                ty_ref.span,
            ));
        };

        if elements.len() != *len {
            return Err(SemanticError::new(
                format!(
                    "array literal length mismatch: expected {len} elements, got {}",
                    elements.len()
                ),
                span,
            ));
        }

        for (index, element_expr) in elements.iter().enumerate() {
            let value_ty = self.check_expr_with_expected(
                element_expr,
                locals,
                ValueUse::Owned,
                Some(element),
            )?;
            if value_ty != **element {
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
    ) -> Result<(), SemanticError> {
        if parts.index_name == parts.value_name {
            return Err(SemanticError::new(
                "range index and value bindings must use different names",
                parts.span,
            ));
        }

        let source_ty = self.check_expr(parts.source, locals, ValueUse::Borrow)?;
        let Type::Array { element, .. } = source_ty else {
            return Err(SemanticError::new(
                format!(
                    "range source must be a fixed-size array, got `{}`",
                    source_ty.source_name()
                ),
                parts.source.span,
            ));
        };
        if !element.is_copy() {
            return Err(SemanticError::new(
                format!(
                    "range value binding requires a Copy element type in v0, got `{}`",
                    element.source_name()
                ),
                parts.source.span,
            ));
        }

        let mut body_locals = locals.clone();
        body_locals.insert(
            parts.index_name.to_string(),
            Local {
                ty: Type::Int,
                mutable: false,
                borrowed: false,
                moved: false,
            },
        );
        body_locals.insert(
            parts.value_name.to_string(),
            Local {
                ty: element.as_ref().clone(),
                mutable: false,
                borrowed: false,
                moved: false,
            },
        );

        self.check_block_statements(parts.body, &mut body_locals, return_type, loop_depth + 1)?;
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

        if matches!(value_use, ValueUse::Owned) && !field_sig.ty.is_copy() {
            self.mark_field_base_moved(base)?;
        }

        Ok(field_sig.ty.clone())
    }

    fn check_index_access(
        &self,
        base: &Expr,
        index: &Expr,
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<Type, SemanticError> {
        let base_ty = self.check_expr(base, locals, ValueUse::Borrow)?;
        let index_ty = self.check_expr(index, locals, ValueUse::Owned)?;
        let Type::Array { len, element } = base_ty else {
            return Err(SemanticError::new(
                format!(
                    "indexing requires a fixed-size array, got `{}`",
                    base_ty.source_name()
                ),
                base.span,
            ));
        };

        self.validate_index_type_and_bounds(index, &index_ty, len)?;

        if !element.is_copy() {
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
        if index_ty != &Type::Int {
            return Err(SemanticError::new(
                format!(
                    "array index must have type `int`, got `{}`",
                    index_ty.source_name()
                ),
                index.span,
            ));
        }

        if let Some(index_value) = const_int_expr(index) {
            let out_of_bounds = if index_value < 0 {
                true
            } else {
                match usize::try_from(index_value) {
                    Ok(index_value) => index_value >= len,
                    Err(_) => true,
                }
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

    fn check_field_assign(
        &self,
        base: &Expr,
        field: &str,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
    ) -> Result<(), SemanticError> {
        let mut place = direct_local_place(
            base,
            "field assignment target must start from a direct local variable in v0",
        )?;
        place.fields.push(field.to_string());
        let (base_ty, base_mutable) = {
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
            (local.ty.clone(), local.mutable)
        };
        if !base_mutable {
            return Err(SemanticError::new(
                format!("cannot assign field of immutable binding `{}`", place.root),
                base.span,
            ));
        }

        let field_ty =
            self.resolve_field_path_type(&base_ty, &place.fields, base.span, "field assignment")?;
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

    fn check_index_assign(
        &self,
        base: &Expr,
        index: &Expr,
        expr: &Expr,
        locals: &mut HashMap<String, Local>,
        span: Span,
    ) -> Result<(), SemanticError> {
        let ExprKind::Var(name) = &base.kind else {
            return Err(SemanticError::new(
                "array assignment target must be a direct mutable local array in v0",
                base.span,
            ));
        };

        let (base_ty, mutable) = {
            let Some(local) = locals.get(name) else {
                return Err(SemanticError::new(
                    format!("unknown variable `{name}`"),
                    base.span,
                ));
            };
            if local.moved {
                return Err(SemanticError::new(
                    format!("use of moved value `{name}`"),
                    base.span,
                ));
            }
            (local.ty.clone(), local.mutable)
        };
        if !mutable {
            return Err(SemanticError::new(
                format!("cannot assign through immutable array binding `{name}`"),
                base.span,
            ));
        }

        let Type::Array { len, element } = base_ty else {
            return Err(SemanticError::new(
                format!(
                    "array assignment target must be a fixed-size array, got `{}`",
                    base_ty.source_name()
                ),
                base.span,
            ));
        };

        self.check_index_expr(index, locals, len)?;

        let value_ty =
            self.check_expr_with_expected(expr, locals, ValueUse::Owned, Some(&element))?;
        if value_ty != *element {
            return Err(SemanticError::new(
                format!(
                    "array assignment type mismatch: expected `{}`, got `{}`",
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
        if let Some((name, ty)) = &arm.binding {
            arm_locals.insert(
                (*name).to_string(),
                Local {
                    ty: ty.clone(),
                    mutable: false,
                    borrowed: false,
                    moved: false,
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
                },
            );
        }
        let returns =
            self.check_block_statements(arm.block, &mut arm_locals, return_type, loop_depth)?;
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
        if let ExprKind::Var(name) = &base.kind {
            return Err(SemanticError::new(
                format!(
                    "moving non-copy field out of `{name}` is not supported without destructuring"
                ),
                base.span,
            ));
        }

        Ok(())
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

        if name == "print" {
            if args.len() != 1 {
                return Err(SemanticError::new(
                    "`print` expects exactly one argument",
                    span,
                ));
            }
            let arg_ty = self.check_expr(&args[0].expr, locals, ValueUse::Borrow)?;
            if matches!(arg_ty, Type::Unit) {
                return Err(SemanticError::new("cannot print unit value", args[0].span));
            }
            return Ok(Type::Unit);
        }

        let sig = self.function_sig(name, callee.span)?;
        self.check_call_args(name, args, &sig.params, locals, Vec::new(), span)?;
        Ok(sig.return_type.clone())
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
        if !matches!(arg_ty, Type::Array { .. }) {
            return Err(SemanticError::new(
                format!(
                    "`len` expects a fixed-size array, got `{}`",
                    arg_ty.source_name()
                ),
                args[0].span,
            ));
        }

        Ok(Type::Int)
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
                let Type::Array { len, element } = base_ty else {
                    return Err(SemanticError::new(
                        format!(
                            "array element borrow target must be a fixed-size array, got `{}`",
                            base_ty.source_name()
                        ),
                        base.span,
                    ));
                };
                self.check_index_expr(index, locals, len)?;
                Ok(*element)
            }
            _ => Err(SemanticError::new(
                "borrow arguments must be direct local variables, direct local fields, or direct local array elements in v0",
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
            "method receivers with `con` or `mut` must be direct local variables, direct local fields, or direct local array elements in v0",
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

    fn type_from_optional_ref(&self, ty: Option<&TypeRef>) -> Result<Type, SemanticError> {
        ty.map_or(Ok(Type::Unit), |ty| self.type_from_ref(ty))
    }

    fn type_from_ref(&self, ty: &TypeRef) -> Result<Type, SemanticError> {
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
        "borrow arguments must be direct local variables, direct local fields, or direct local array elements in v0",
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

fn direct_local_place(expr: &Expr, message: &'static str) -> Result<BorrowPlace, SemanticError> {
    match &expr.kind {
        ExprKind::Var(name) => Ok(BorrowPlace::root(name.clone())),
        ExprKind::FieldAccess { base, field } => {
            let mut place = direct_local_place(base, message)?;
            place.fields.push(field.clone());
            Ok(place)
        }
        _ => Err(SemanticError::new(message, expr.span)),
    }
}

fn const_int_expr(expr: &Expr) -> Option<i64> {
    match &expr.kind {
        ExprKind::Int(value) => Some(*value),
        ExprKind::Unary {
            op: UnaryOp::Negate,
            expr,
        } => const_int_expr(expr)?.checked_neg(),
        _ => None,
    }
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
        "method receivers with `con` or `mut` must be direct local variables, direct local fields, or direct local array elements in v0",
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
        let moved_in_then = then_locals.get(name).is_some_and(|branch| branch.moved);
        let moved_in_else = else_locals.get(name).is_some_and(|branch| branch.moved);
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
                .is_some_and(|branch_local| branch_local.moved)
        });
    }
}

fn merge_loop_body_moves(
    locals: &mut HashMap<String, Local>,
    body_locals: &HashMap<String, Local>,
) {
    for (name, local) in locals {
        local.moved |= body_locals.get(name).is_some_and(|body| body.moved);
    }
}

fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "int" | "bool" | "string" | "unit" | "Option" | "Result"
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
    fn for_statement_merges_body_moves() {
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
        assert!(error.message.contains("use of moved value `s`"));
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
            .contains("cannot assign through immutable array binding `values`"));
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
            .contains("indexing requires a fixed-size array, got `int`"));
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
            .contains("array index -1 is out of bounds for length 3"));
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
            .contains("`len` expects a fixed-size array, got `int`"));
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
            .contains("range source must be a fixed-size array, got `int`"));
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
