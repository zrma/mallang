use std::{collections::HashMap, fmt};

use crate::{
    ast::{
        Arg, ArgMode, BinaryOp, Block, Expr, ExprKind, ForInit, ForPost, Function, MatchArm,
        MatchBlockArm, MatchPattern, ParamMode, Stmt, StmtKind, UnaryOp,
    },
    semantic::{CheckedProgram, FunctionSig, MethodKey, MethodSig, ParamSig, StructSig, Type},
    token::Span,
};

pub fn lower(checked: &CheckedProgram<'_>) -> Result<IrProgram, IrError> {
    Lowerer::new(checked).lower_program()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrProgram {
    pub structs: Vec<IrStruct>,
    pub functions: Vec<IrFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrStruct {
    pub name: String,
    pub fields: Vec<IrStructField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrStructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<IrParam>,
    pub return_type: Type,
    pub body: Vec<IrStmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrParam {
    pub name: String,
    pub mode: ParamMode,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrStmt {
    pub kind: IrStmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrStmtKind {
    Let {
        mutable: bool,
        name: String,
        ty: Type,
        expr: IrExpr,
    },
    Assign {
        name: String,
        expr: IrExpr,
    },
    FieldAssign {
        base: IrExpr,
        field: String,
        expr: IrExpr,
    },
    IndexAssign {
        base: IrExpr,
        index: IrExpr,
        expr: IrExpr,
    },
    Return {
        expr: IrExpr,
    },
    If {
        condition: IrExpr,
        then_body: Vec<IrStmt>,
        else_body: Vec<IrStmt>,
    },
    For {
        init: Option<Box<IrForInit>>,
        condition: Option<Box<IrExpr>>,
        post: Option<Box<IrForPost>>,
        body: Vec<IrStmt>,
    },
    RangeFor {
        index_name: String,
        value_name: String,
        source: IrExpr,
        element_ty: Type,
        body: Vec<IrStmt>,
    },
    Break,
    Continue,
    Match {
        scrutinee: IrExpr,
        arms: Vec<IrMatchBlockArm>,
    },
    Expr {
        expr: IrExpr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrForInit {
    Let {
        mutable: bool,
        name: String,
        ty: Type,
        expr: IrExpr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrForPost {
    Assign { target: IrExpr, expr: IrExpr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrExpr {
    pub kind: IrExprKind,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrExprKind {
    Int(i64),
    String(String),
    Bool(bool),
    Var(String),
    If {
        condition: Box<IrExpr>,
        then_branch: Box<IrExpr>,
        else_branch: Box<IrExpr>,
    },
    AdtConstructor {
        constructor: IrAdtConstructor,
        payload: Option<Box<IrExpr>>,
    },
    Match {
        scrutinee: Box<IrExpr>,
        arms: Vec<IrMatchArm>,
    },
    StructLiteral {
        type_name: String,
        fields: Vec<IrFieldValue>,
    },
    ArrayLiteral {
        elements: Vec<IrExpr>,
    },
    FieldAccess {
        base: Box<IrExpr>,
        field: String,
    },
    Index {
        base: Box<IrExpr>,
        index: Box<IrExpr>,
    },
    ArrayLen {
        array: Box<IrExpr>,
    },
    Call {
        callee: String,
        args: Vec<IrArg>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<IrExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<IrExpr>,
        right: Box<IrExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrAdtConstructor {
    Some,
    None,
    Ok,
    Err,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrFieldValue {
    pub name: String,
    pub expr: IrExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrMatchArm {
    pub pattern: IrMatchPattern,
    pub expr: IrExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrMatchBlockArm {
    pub pattern: IrMatchPattern,
    pub body: Vec<IrStmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrMatchPattern {
    Some(String),
    None,
    Ok(String),
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrArg {
    pub mode: ArgMode,
    pub expr: IrExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrError {
    pub message: String,
    pub span: Span,
}

impl IrError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl fmt::Display for IrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at {}..{}",
            self.message, self.span.start, self.span.end
        )
    }
}

impl std::error::Error for IrError {}

struct Lowerer<'a> {
    checked: &'a CheckedProgram<'a>,
}

impl<'a> Lowerer<'a> {
    fn new(checked: &'a CheckedProgram<'a>) -> Self {
        Self { checked }
    }

    fn lower_program(&self) -> Result<IrProgram, IrError> {
        let structs = self.lower_structs()?;
        let mut functions = Vec::new();
        for function in &self.checked.program.functions {
            let (receiver, sig, ir_name) = self.callable_sig(function)?;
            let mut locals = HashMap::new();
            if let Some(receiver) = receiver {
                locals.insert(receiver.name.clone(), receiver.ty.clone());
            }
            for param in &sig.params {
                locals.insert(param.name.clone(), param.ty.clone());
            }
            let mut body = Vec::new();
            for stmt in &function.body.statements {
                body.push(self.lower_stmt(stmt, &mut locals, &sig.return_type)?);
            }
            let mut params = Vec::new();
            if let Some(receiver) = receiver {
                params.push(lower_param(receiver));
            }
            params.extend(sig.params.iter().map(lower_param));
            functions.push(IrFunction {
                name: ir_name,
                params,
                return_type: sig.return_type.clone(),
                body,
            });
        }

        Ok(IrProgram { structs, functions })
    }

    fn lower_structs(&self) -> Result<Vec<IrStruct>, IrError> {
        let mut structs = Vec::new();
        for struct_decl in &self.checked.program.structs {
            let sig = self.struct_sig(&struct_decl.name, struct_decl.span)?;
            structs.push(IrStruct {
                name: struct_decl.name.clone(),
                fields: sig
                    .fields
                    .iter()
                    .map(|field| IrStructField {
                        name: field.name.clone(),
                        ty: field.ty.clone(),
                    })
                    .collect(),
            });
        }
        Ok(structs)
    }

    fn lower_stmt(
        &self,
        stmt: &Stmt,
        locals: &mut HashMap<String, Type>,
        return_type: &Type,
    ) -> Result<IrStmt, IrError> {
        let kind = match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                expr,
            } => {
                let expr = self.lower_expr(expr, locals)?;
                let ty = expr.ty.clone();
                locals.insert(name.clone(), ty.clone());
                IrStmtKind::Let {
                    mutable: *mutable,
                    name: name.clone(),
                    ty,
                    expr,
                }
            }
            StmtKind::Assign { name, expr } => {
                let Some(expected) = locals.get(name).cloned() else {
                    return Err(IrError::new(
                        format!("unknown assignment target `{name}` during IR lowering"),
                        stmt.span,
                    ));
                };
                let expr = self.lower_expr_with_expected(expr, locals, Some(&expected))?;
                IrStmtKind::Assign {
                    name: name.clone(),
                    expr,
                }
            }
            StmtKind::FieldAssign { base, field, expr } => {
                let base = self.lower_expr(base, locals)?;
                let Type::Struct(type_name) = &base.ty else {
                    return Err(IrError::new(
                        "semantic analysis accepted field assignment on non-struct value",
                        stmt.span,
                    ));
                };
                let sig = self.struct_sig(type_name, stmt.span)?;
                let Some(field_sig) = sig.fields.iter().find(|candidate| candidate.name == *field)
                else {
                    return Err(IrError::new(
                        "semantic analysis accepted unknown field assignment",
                        stmt.span,
                    ));
                };
                let expr = self.lower_expr_with_expected(expr, locals, Some(&field_sig.ty))?;
                IrStmtKind::FieldAssign {
                    base,
                    field: field.clone(),
                    expr,
                }
            }
            StmtKind::IndexAssign { base, index, expr } => {
                let base = self.lower_expr(base, locals)?;
                let Type::Array { element, .. } = &base.ty else {
                    return Err(IrError::new(
                        "semantic analysis accepted array assignment on non-array value",
                        stmt.span,
                    ));
                };
                if !element.is_copy() {
                    return Err(IrError::new(
                        "semantic analysis accepted assignment to non-copy array element",
                        stmt.span,
                    ));
                }
                let index = self.lower_expr(index, locals)?;
                if index.ty != Type::Int {
                    return Err(IrError::new(
                        "semantic analysis accepted array assignment with non-int index",
                        index.span,
                    ));
                }
                let expr = self.lower_expr_with_expected(expr, locals, Some(element))?;
                IrStmtKind::IndexAssign { base, index, expr }
            }
            StmtKind::Return { expr } => IrStmtKind::Return {
                expr: self.lower_expr_with_expected(expr, locals, Some(return_type))?,
            },
            StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition = self.lower_expr(condition, locals)?;
                if condition.ty != Type::Bool {
                    return Err(IrError::new(
                        "semantic analysis accepted a non-bool if condition",
                        condition.span,
                    ));
                }

                let mut then_locals = locals.clone();
                let then_body =
                    self.lower_block_statements(then_block, &mut then_locals, return_type)?;
                let else_body = if let Some(else_block) = else_block {
                    let mut else_locals = locals.clone();
                    self.lower_block_statements(else_block, &mut else_locals, return_type)?
                } else {
                    Vec::new()
                };

                IrStmtKind::If {
                    condition,
                    then_body,
                    else_body,
                }
            }
            StmtKind::For {
                init,
                condition,
                post,
                body,
            } => {
                let mut loop_locals = locals.clone();
                let init = init
                    .as_ref()
                    .map(|init| self.lower_for_init(init, &mut loop_locals))
                    .map(|result| result.map(Box::new))
                    .transpose()?;
                let condition = condition
                    .as_ref()
                    .map(|condition| self.lower_expr(condition, &loop_locals))
                    .map(|result| result.map(Box::new))
                    .transpose()?;
                if let Some(condition) = condition.as_ref() {
                    if condition.ty != Type::Bool {
                        return Err(IrError::new(
                            "semantic analysis accepted a non-bool for condition",
                            condition.span,
                        ));
                    }
                }

                let mut body_locals = loop_locals.clone();
                let body = self.lower_block_statements(body, &mut body_locals, return_type)?;
                let post = post
                    .as_ref()
                    .map(|post| self.lower_for_post(post, &mut loop_locals))
                    .map(|result| result.map(Box::new))
                    .transpose()?;

                IrStmtKind::For {
                    init,
                    condition,
                    post,
                    body,
                }
            }
            StmtKind::RangeFor {
                index_name,
                value_name,
                source,
                body,
            } => self.lower_range_for(
                IrRangeForParts {
                    index_name,
                    value_name,
                    source,
                    body,
                    span: stmt.span,
                },
                locals,
                return_type,
            )?,
            StmtKind::Break => IrStmtKind::Break,
            StmtKind::Continue => IrStmtKind::Continue,
            StmtKind::Match { scrutinee, arms } => {
                let scrutinee = self.lower_expr(scrutinee, locals)?;
                let prepared_arms =
                    self.prepare_match_block_arms(&scrutinee.ty, arms, stmt.span)?;
                let arms = self.lower_match_block_arms(&prepared_arms, locals, return_type)?;
                IrStmtKind::Match { scrutinee, arms }
            }
            StmtKind::Expr { expr } => IrStmtKind::Expr {
                expr: self.lower_expr(expr, locals)?,
            },
        };

        Ok(IrStmt {
            kind,
            span: stmt.span,
        })
    }

    fn lower_range_for(
        &self,
        parts: IrRangeForParts<'_>,
        locals: &HashMap<String, Type>,
        return_type: &Type,
    ) -> Result<IrStmtKind, IrError> {
        let source = self.lower_expr(parts.source, locals)?;
        let Type::Array { element, .. } = &source.ty else {
            return Err(IrError::new(
                "semantic analysis accepted range over non-array source",
                parts.span,
            ));
        };
        if !element.is_copy() {
            return Err(IrError::new(
                "semantic analysis accepted range over non-Copy element type",
                parts.span,
            ));
        }

        let element_ty = element.as_ref().clone();
        let mut body_locals = locals.clone();
        body_locals.insert(parts.index_name.to_string(), Type::Int);
        body_locals.insert(parts.value_name.to_string(), element_ty.clone());
        let body = self.lower_block_statements(parts.body, &mut body_locals, return_type)?;

        Ok(IrStmtKind::RangeFor {
            index_name: parts.index_name.to_string(),
            value_name: parts.value_name.to_string(),
            source,
            element_ty,
            body,
        })
    }

    fn lower_for_init(
        &self,
        init: &ForInit,
        locals: &mut HashMap<String, Type>,
    ) -> Result<IrForInit, IrError> {
        match init {
            ForInit::Let {
                mutable,
                name,
                expr,
            } => {
                let expr = self.lower_expr(expr, locals)?;
                let ty = expr.ty.clone();
                locals.insert(name.clone(), ty.clone());
                Ok(IrForInit::Let {
                    mutable: *mutable,
                    name: name.clone(),
                    ty,
                    expr,
                })
            }
        }
    }

    fn lower_for_post(
        &self,
        post: &ForPost,
        locals: &mut HashMap<String, Type>,
    ) -> Result<IrForPost, IrError> {
        match post {
            ForPost::Assign { target, expr } => {
                let target = self.lower_expr(target, locals)?;
                match &target.kind {
                    IrExprKind::Var(_)
                    | IrExprKind::FieldAccess { .. }
                    | IrExprKind::Index { .. } => {}
                    _ => {
                        return Err(IrError::new(
                            "semantic analysis accepted invalid for post target",
                            target.span,
                        ));
                    }
                }
                let expr = self.lower_expr_with_expected(expr, locals, Some(&target.ty))?;
                Ok(IrForPost::Assign { target, expr })
            }
        }
    }

    fn lower_block_statements(
        &self,
        block: &Block,
        locals: &mut HashMap<String, Type>,
        return_type: &Type,
    ) -> Result<Vec<IrStmt>, IrError> {
        let mut body = Vec::new();
        for stmt in &block.statements {
            body.push(self.lower_stmt(stmt, locals, return_type)?);
        }
        Ok(body)
    }

    fn lower_expr(&self, expr: &Expr, locals: &HashMap<String, Type>) -> Result<IrExpr, IrError> {
        self.lower_expr_with_expected(expr, locals, None)
    }

    fn lower_expr_with_expected(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
    ) -> Result<IrExpr, IrError> {
        let (kind, ty) = match &expr.kind {
            ExprKind::Int(value) => (IrExprKind::Int(*value), Type::Int),
            ExprKind::String(value) => (IrExprKind::String(value.clone()), Type::String),
            ExprKind::Bool(value) => (IrExprKind::Bool(*value), Type::Bool),
            ExprKind::Nil => {
                return Err(IrError::new(
                    "`nil` should have been rejected by semantic analysis",
                    expr.span,
                ));
            }
            ExprKind::Var(name) => {
                if matches!(name.as_str(), "None") {
                    self.lower_none_constructor(expected, expr.span)?
                } else {
                    let Some(ty) = locals.get(name).cloned() else {
                        return Err(IrError::new(
                            format!("unknown variable `{name}` during IR lowering"),
                            expr.span,
                        ));
                    };
                    (IrExprKind::Var(name.clone()), ty)
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.lower_if_expr(
                condition,
                then_branch,
                else_branch,
                locals,
                expected,
                expr.span,
            )?,
            ExprKind::Match { scrutinee, arms } => {
                self.lower_match_expr(scrutinee, arms, locals, expected, expr.span)?
            }
            ExprKind::StructLiteral { type_name, fields } => {
                self.lower_struct_literal(type_name, fields, locals, expected, expr.span)?
            }
            ExprKind::ArrayLiteral { ty, elements } => {
                self.lower_array_literal(ty, elements, locals, expected, expr.span)?
            }
            ExprKind::FieldAccess { base, field } => {
                self.lower_field_access(base, field, locals, expr.span)?
            }
            ExprKind::Index { base, index } => {
                self.lower_index_access(base, index, locals, expr.span)?
            }
            ExprKind::Call { callee, args } => {
                self.lower_call(callee, args, locals, expected, expr.span)?
            }
            ExprKind::Unary { op, expr } => {
                let expr = self.lower_expr(expr, locals)?;
                let ty = match (*op, &expr.ty) {
                    (UnaryOp::Negate, Type::Int) => Type::Int,
                    (UnaryOp::Not, Type::Bool) => Type::Bool,
                    _ => {
                        return Err(IrError::new(
                            "semantic analysis accepted an invalid unary expression",
                            expr.span,
                        ));
                    }
                };
                (
                    IrExprKind::Unary {
                        op: *op,
                        expr: Box::new(expr),
                    },
                    ty,
                )
            }
            ExprKind::Binary { op, left, right } => {
                let left = self.lower_expr(left, locals)?;
                let right = self.lower_expr(right, locals)?;
                let ty = match op {
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => Type::Int,
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::LogicalAnd
                    | BinaryOp::LogicalOr
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Type::Bool,
                };
                (
                    IrExprKind::Binary {
                        op: *op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    ty,
                )
            }
        };

        Ok(IrExpr {
            kind,
            ty,
            span: expr.span,
        })
    }

    fn lower_borrow_expr_with_expected(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
    ) -> Result<IrExpr, IrError> {
        let lowered = self.lower_borrow_expr(expr, locals)?;
        if let Some(expected) = expected {
            if &lowered.ty != expected {
                return Err(IrError::new(
                    "semantic analysis accepted mismatched borrow argument type",
                    expr.span,
                ));
            }
        }
        Ok(lowered)
    }

    fn lower_borrow_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<IrExpr, IrError> {
        let (kind, ty) = match &expr.kind {
            ExprKind::Var(name) => {
                let Some(ty) = locals.get(name).cloned() else {
                    return Err(IrError::new(
                        format!("unknown variable `{name}` during IR lowering"),
                        expr.span,
                    ));
                };
                (IrExprKind::Var(name.clone()), ty)
            }
            ExprKind::FieldAccess { base, field } => {
                let base = self.lower_borrow_expr(base, locals)?;
                let Type::Struct(type_name) = &base.ty else {
                    return Err(IrError::new(
                        "semantic analysis accepted field borrow on non-struct value",
                        expr.span,
                    ));
                };
                let sig = self.struct_sig(type_name, expr.span)?;
                let Some(field_sig) = sig.fields.iter().find(|candidate| candidate.name == *field)
                else {
                    return Err(IrError::new(
                        "semantic analysis accepted unknown struct field borrow",
                        expr.span,
                    ));
                };
                (
                    IrExprKind::FieldAccess {
                        base: Box::new(base),
                        field: field.clone(),
                    },
                    field_sig.ty.clone(),
                )
            }
            ExprKind::Index { base, index } => {
                let base = self.lower_borrow_expr(base, locals)?;
                let index = self.lower_expr(index, locals)?;
                if index.ty != Type::Int {
                    return Err(IrError::new(
                        "semantic analysis accepted array element borrow with non-int index",
                        index.span,
                    ));
                }
                let Type::Array { element, .. } = &base.ty else {
                    return Err(IrError::new(
                        "semantic analysis accepted array element borrow on non-array value",
                        expr.span,
                    ));
                };
                let element_ty = element.as_ref().clone();
                (
                    IrExprKind::Index {
                        base: Box::new(base),
                        index: Box::new(index),
                    },
                    element_ty,
                )
            }
            _ => {
                return Err(IrError::new(
                    "semantic analysis accepted invalid borrow argument expression",
                    expr.span,
                ));
            }
        };

        Ok(IrExpr {
            kind,
            ty,
            span: expr.span,
        })
    }

    fn lower_if_expr(
        &self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let condition = self.lower_expr(condition, locals)?;
        if condition.ty != Type::Bool {
            return Err(IrError::new(
                "semantic analysis accepted a non-bool if condition",
                condition.span,
            ));
        }

        let (then_branch, else_branch) =
            self.lower_pair_branches(then_branch, else_branch, locals, expected)?;
        if then_branch.ty != else_branch.ty {
            return Err(IrError::new(
                "semantic analysis accepted mismatched if branch types",
                span,
            ));
        }

        let ty = then_branch.ty.clone();
        Ok((
            IrExprKind::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            ty,
        ))
    }

    fn lower_pair_branches(
        &self,
        then_branch: &Expr,
        else_branch: &Expr,
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
    ) -> Result<(IrExpr, IrExpr), IrError> {
        if let Some(expected) = expected {
            return Ok((
                self.lower_expr_with_expected(then_branch, locals, Some(expected))?,
                self.lower_expr_with_expected(else_branch, locals, Some(expected))?,
            ));
        }

        match self.lower_expr(then_branch, locals) {
            Ok(then_expr) => {
                let else_expr =
                    self.lower_expr_with_expected(else_branch, locals, Some(&then_expr.ty))?;
                Ok((then_expr, else_expr))
            }
            Err(then_error) => {
                let else_expr = self.lower_expr(else_branch, locals)?;
                let then_expr = self
                    .lower_expr_with_expected(then_branch, locals, Some(&else_expr.ty))
                    .map_err(|_| then_error)?;
                Ok((then_expr, else_expr))
            }
        }
    }

    fn lower_match_expr(
        &self,
        scrutinee: &Expr,
        arms: &[MatchArm],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let scrutinee = self.lower_expr(scrutinee, locals)?;
        let prepared_arms = self.prepare_match_arms(&scrutinee.ty, arms, span)?;
        let arms = self.lower_match_arms(&prepared_arms, locals, expected)?;
        let ty = arms[0].expr.ty.clone();

        for arm in &arms[1..] {
            if arm.expr.ty != ty {
                return Err(IrError::new(
                    "semantic analysis accepted mismatched match arm types",
                    span,
                ));
            }
        }

        Ok((
            IrExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            ty,
        ))
    }

    fn lower_struct_literal(
        &self,
        type_name: &str,
        fields: &[crate::ast::FieldInit],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let ty = Type::Struct(type_name.to_string());
        if let Some(expected) = expected {
            if expected != &ty {
                return Err(IrError::new(
                    "semantic analysis accepted mismatched struct literal type",
                    span,
                ));
            }
        }

        let sig = self.struct_sig(type_name, span)?;
        let mut lowered = Vec::new();
        for field_sig in &sig.fields {
            let Some(field) = fields.iter().find(|field| field.name == field_sig.name) else {
                return Err(IrError::new(
                    "semantic analysis accepted missing struct literal field",
                    span,
                ));
            };
            lowered.push(IrFieldValue {
                name: field.name.clone(),
                expr: self.lower_expr_with_expected(&field.expr, locals, Some(&field_sig.ty))?,
                span: field.span,
            });
        }

        Ok((
            IrExprKind::StructLiteral {
                type_name: type_name.to_string(),
                fields: lowered,
            },
            ty,
        ))
    }

    fn lower_array_literal(
        &self,
        ty_ref: &crate::ast::TypeRef,
        elements: &[Expr],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let ty = self.type_from_ref(ty_ref)?;
        if let Some(expected) = expected {
            if expected != &ty {
                return Err(IrError::new(
                    "semantic analysis accepted mismatched array literal type",
                    span,
                ));
            }
        }

        let Type::Array { len, element } = &ty else {
            return Err(IrError::new(
                "semantic analysis accepted array literal without array type",
                ty_ref.span,
            ));
        };

        if elements.len() != *len {
            return Err(IrError::new(
                "semantic analysis accepted array literal length mismatch",
                span,
            ));
        }

        let mut lowered = Vec::new();
        for element_expr in elements {
            let expr = self.lower_expr_with_expected(element_expr, locals, Some(element))?;
            if expr.ty != **element {
                return Err(IrError::new(
                    "semantic analysis accepted array literal element type mismatch",
                    element_expr.span,
                ));
            }
            lowered.push(expr);
        }

        Ok((IrExprKind::ArrayLiteral { elements: lowered }, ty))
    }

    fn lower_field_access(
        &self,
        base: &Expr,
        field: &str,
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let base = self.lower_expr(base, locals)?;
        let Type::Struct(type_name) = &base.ty else {
            return Err(IrError::new(
                "semantic analysis accepted field access on non-struct value",
                span,
            ));
        };
        let sig = self.struct_sig(type_name, span)?;
        let Some(field_sig) = sig.fields.iter().find(|candidate| candidate.name == field) else {
            return Err(IrError::new(
                "semantic analysis accepted unknown struct field access",
                span,
            ));
        };
        let ty = field_sig.ty.clone();

        Ok((
            IrExprKind::FieldAccess {
                base: Box::new(base),
                field: field.to_string(),
            },
            ty,
        ))
    }

    fn lower_index_access(
        &self,
        base: &Expr,
        index: &Expr,
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let base = self.lower_expr(base, locals)?;
        let index = self.lower_expr(index, locals)?;
        if index.ty != Type::Int {
            return Err(IrError::new(
                "semantic analysis accepted array index with non-int type",
                index.span,
            ));
        }
        let Type::Array { element, .. } = &base.ty else {
            return Err(IrError::new(
                "semantic analysis accepted indexing on non-array value",
                span,
            ));
        };
        let ty = (**element).clone();
        if !ty.is_copy() {
            return Err(IrError::new(
                "semantic analysis accepted indexing a non-copy array element",
                span,
            ));
        }

        Ok((
            IrExprKind::Index {
                base: Box::new(base),
                index: Box::new(index),
            },
            ty,
        ))
    }

    fn prepare_match_arms<'b>(
        &self,
        scrutinee_ty: &Type,
        arms: &'b [MatchArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchArm<'b>>, IrError> {
        match scrutinee_ty {
            Type::Option(inner) => self.prepare_option_match_arms(inner, arms, span),
            Type::Result(ok, err) => self.prepare_result_match_arms(ok, err, arms, span),
            _ => Err(IrError::new(
                "semantic analysis accepted match on non-ADT value",
                span,
            )),
        }
    }

    fn prepare_match_block_arms<'b>(
        &self,
        scrutinee_ty: &Type,
        arms: &'b [MatchBlockArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchBlockArm<'b>>, IrError> {
        match scrutinee_ty {
            Type::Option(inner) => self.prepare_option_match_block_arms(inner, arms, span),
            Type::Result(ok, err) => self.prepare_result_match_block_arms(ok, err, arms, span),
            _ => Err(IrError::new(
                "semantic analysis accepted match on non-ADT value",
                span,
            )),
        }
    }

    fn prepare_option_match_arms<'b>(
        &self,
        inner: &Type,
        arms: &'b [MatchArm],
        span: Span,
    ) -> Result<Vec<PreparedMatchArm<'b>>, IrError> {
        let mut prepared = Vec::new();
        let mut seen_some = false;
        let mut seen_none = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Some(binding) => {
                    if seen_some {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate Some arm",
                            arm.span,
                        ));
                    }
                    seen_some = true;
                    prepared.push(PreparedMatchArm {
                        pattern: IrMatchPattern::Some(binding.clone()),
                        expr: &arm.expr,
                        binding: Some((binding.clone(), inner.clone())),
                    });
                }
                MatchPattern::None => {
                    if seen_none {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate None arm",
                            arm.span,
                        ));
                    }
                    seen_none = true;
                    prepared.push(PreparedMatchArm {
                        pattern: IrMatchPattern::None,
                        expr: &arm.expr,
                        binding: None,
                    });
                }
                MatchPattern::Ok(_) | MatchPattern::Err(_) => {
                    return Err(IrError::new(
                        "semantic analysis accepted invalid Option match pattern",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_some || !seen_none {
            return Err(IrError::new(
                "semantic analysis accepted non-exhaustive Option match",
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
    ) -> Result<Vec<PreparedMatchArm<'b>>, IrError> {
        let mut prepared = Vec::new();
        let mut seen_ok = false;
        let mut seen_err = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Ok(binding) => {
                    if seen_ok {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate Ok arm",
                            arm.span,
                        ));
                    }
                    seen_ok = true;
                    prepared.push(PreparedMatchArm {
                        pattern: IrMatchPattern::Ok(binding.clone()),
                        expr: &arm.expr,
                        binding: Some((binding.clone(), ok.clone())),
                    });
                }
                MatchPattern::Err(binding) => {
                    if seen_err {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate Err arm",
                            arm.span,
                        ));
                    }
                    seen_err = true;
                    prepared.push(PreparedMatchArm {
                        pattern: IrMatchPattern::Err(binding.clone()),
                        expr: &arm.expr,
                        binding: Some((binding.clone(), err.clone())),
                    });
                }
                MatchPattern::Some(_) | MatchPattern::None => {
                    return Err(IrError::new(
                        "semantic analysis accepted invalid Result match pattern",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_ok || !seen_err {
            return Err(IrError::new(
                "semantic analysis accepted non-exhaustive Result match",
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
    ) -> Result<Vec<PreparedMatchBlockArm<'b>>, IrError> {
        let mut prepared = Vec::new();
        let mut seen_some = false;
        let mut seen_none = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Some(binding) => {
                    if seen_some {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate Some arm",
                            arm.span,
                        ));
                    }
                    seen_some = true;
                    prepared.push(PreparedMatchBlockArm {
                        pattern: IrMatchPattern::Some(binding.clone()),
                        block: &arm.block,
                        binding: Some((binding.clone(), inner.clone())),
                    });
                }
                MatchPattern::None => {
                    if seen_none {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate None arm",
                            arm.span,
                        ));
                    }
                    seen_none = true;
                    prepared.push(PreparedMatchBlockArm {
                        pattern: IrMatchPattern::None,
                        block: &arm.block,
                        binding: None,
                    });
                }
                MatchPattern::Ok(_) | MatchPattern::Err(_) => {
                    return Err(IrError::new(
                        "semantic analysis accepted invalid Option match pattern",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_some || !seen_none {
            return Err(IrError::new(
                "semantic analysis accepted non-exhaustive Option match",
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
    ) -> Result<Vec<PreparedMatchBlockArm<'b>>, IrError> {
        let mut prepared = Vec::new();
        let mut seen_ok = false;
        let mut seen_err = false;

        for arm in arms {
            match &arm.pattern {
                MatchPattern::Ok(binding) => {
                    if seen_ok {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate Ok arm",
                            arm.span,
                        ));
                    }
                    seen_ok = true;
                    prepared.push(PreparedMatchBlockArm {
                        pattern: IrMatchPattern::Ok(binding.clone()),
                        block: &arm.block,
                        binding: Some((binding.clone(), ok.clone())),
                    });
                }
                MatchPattern::Err(binding) => {
                    if seen_err {
                        return Err(IrError::new(
                            "semantic analysis accepted duplicate Err arm",
                            arm.span,
                        ));
                    }
                    seen_err = true;
                    prepared.push(PreparedMatchBlockArm {
                        pattern: IrMatchPattern::Err(binding.clone()),
                        block: &arm.block,
                        binding: Some((binding.clone(), err.clone())),
                    });
                }
                MatchPattern::Some(_) | MatchPattern::None => {
                    return Err(IrError::new(
                        "semantic analysis accepted invalid Result match pattern",
                        arm.span,
                    ));
                }
            }
        }

        if !seen_ok || !seen_err {
            return Err(IrError::new(
                "semantic analysis accepted non-exhaustive Result match",
                span,
            ));
        }

        Ok(prepared)
    }

    fn lower_match_arms(
        &self,
        arms: &[PreparedMatchArm<'_>],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
    ) -> Result<Vec<IrMatchArm>, IrError> {
        if let Some(expected) = expected {
            return arms
                .iter()
                .map(|arm| self.lower_prepared_match_arm(arm, locals, Some(expected)))
                .collect();
        }

        let mut first_error = None;
        for arm in arms {
            match self.lower_prepared_match_arm(arm, locals, None) {
                Ok(first_arm) => {
                    let expected_ty = first_arm.expr.ty.clone();
                    let mut lowered = Vec::new();
                    for retry_arm in arms {
                        lowered.push(self.lower_prepared_match_arm(
                            retry_arm,
                            locals,
                            Some(&expected_ty),
                        )?);
                    }
                    return Ok(lowered);
                }
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }

        Err(first_error.expect("match arms are non-empty"))
    }

    fn lower_prepared_match_arm(
        &self,
        arm: &PreparedMatchArm<'_>,
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
    ) -> Result<IrMatchArm, IrError> {
        let mut arm_locals = locals.clone();
        if let Some((name, ty)) = &arm.binding {
            arm_locals.insert(name.clone(), ty.clone());
        }

        Ok(IrMatchArm {
            pattern: arm.pattern.clone(),
            expr: self.lower_expr_with_expected(arm.expr, &arm_locals, expected)?,
            span: arm.expr.span,
        })
    }

    fn lower_match_block_arms(
        &self,
        arms: &[PreparedMatchBlockArm<'_>],
        locals: &HashMap<String, Type>,
        return_type: &Type,
    ) -> Result<Vec<IrMatchBlockArm>, IrError> {
        arms.iter()
            .map(|arm| self.lower_prepared_match_block_arm(arm, locals, return_type))
            .collect()
    }

    fn lower_prepared_match_block_arm(
        &self,
        arm: &PreparedMatchBlockArm<'_>,
        locals: &HashMap<String, Type>,
        return_type: &Type,
    ) -> Result<IrMatchBlockArm, IrError> {
        let mut arm_locals = locals.clone();
        if let Some((name, ty)) = &arm.binding {
            arm_locals.insert(name.clone(), ty.clone());
        }

        Ok(IrMatchBlockArm {
            pattern: arm.pattern.clone(),
            body: self.lower_block_statements(arm.block, &mut arm_locals, return_type)?,
            span: arm.block.span,
        })
    }

    fn lower_none_constructor(
        &self,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let Some(expected @ Type::Option(_)) = expected else {
            return Err(IrError::new(
                "semantic analysis accepted `None` without Option context",
                span,
            ));
        };

        Ok((
            IrExprKind::AdtConstructor {
                constructor: IrAdtConstructor::None,
                payload: None,
            },
            expected.clone(),
        ))
    }

    fn lower_some_constructor(
        &self,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let arg = expect_constructor_arg("Some", args, span)?;
        let expected_payload = match expected {
            Some(Type::Option(inner)) => Some(inner.as_ref()),
            _ => None,
        };
        let payload = self.lower_expr_with_expected(&arg.expr, locals, expected_payload)?;
        let ty = Type::Option(Box::new(payload.ty.clone()));

        Ok((
            IrExprKind::AdtConstructor {
                constructor: IrAdtConstructor::Some,
                payload: Some(Box::new(payload)),
            },
            ty,
        ))
    }

    fn lower_ok_constructor(
        &self,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let arg = expect_constructor_arg("Ok", args, span)?;
        let Some(Type::Result(expected_ok, expected_err)) = expected else {
            return Err(IrError::new(
                "semantic analysis accepted `Ok` without Result context",
                span,
            ));
        };
        let payload = self.lower_expr_with_expected(&arg.expr, locals, Some(expected_ok))?;
        let ty = Type::Result(
            Box::new(payload.ty.clone()),
            Box::new(expected_err.as_ref().clone()),
        );

        Ok((
            IrExprKind::AdtConstructor {
                constructor: IrAdtConstructor::Ok,
                payload: Some(Box::new(payload)),
            },
            ty,
        ))
    }

    fn lower_err_constructor(
        &self,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let arg = expect_constructor_arg("Err", args, span)?;
        let Some(Type::Result(expected_ok, expected_err)) = expected else {
            return Err(IrError::new(
                "semantic analysis accepted `Err` without Result context",
                span,
            ));
        };
        let payload = self.lower_expr_with_expected(&arg.expr, locals, Some(expected_err))?;
        let ty = Type::Result(
            Box::new(expected_ok.as_ref().clone()),
            Box::new(payload.ty.clone()),
        );

        Ok((
            IrExprKind::AdtConstructor {
                constructor: IrAdtConstructor::Err,
                payload: Some(Box::new(payload)),
            },
            ty,
        ))
    }

    fn lower_call(
        &self,
        callee: &Expr,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        if let ExprKind::FieldAccess { base, field } = &callee.kind {
            return self.lower_method_call(base, field, args, locals, span);
        }

        let ExprKind::Var(name) = &callee.kind else {
            return Err(IrError::new(
                "only direct function and method calls should reach IR lowering",
                callee.span,
            ));
        };

        match name.as_str() {
            "Some" => return self.lower_some_constructor(args, locals, expected, span),
            "Ok" => return self.lower_ok_constructor(args, locals, expected, span),
            "Err" => return self.lower_err_constructor(args, locals, expected, span),
            _ => {}
        }

        if name == "len" {
            return self.lower_len_builtin(args, locals, span);
        }

        if name == "print" {
            let mut lowered_args = Vec::new();
            for arg in args {
                lowered_args.push(IrArg {
                    mode: arg.mode,
                    expr: self.lower_expr(&arg.expr, locals)?,
                    span: arg.span,
                });
            }
            return Ok((
                IrExprKind::Call {
                    callee: name.clone(),
                    args: lowered_args,
                },
                Type::Unit,
            ));
        }

        let sig = self.function_sig(name, span)?;
        let mut lowered_args = Vec::new();
        for (arg, param) in args.iter().zip(sig.params.iter()) {
            lowered_args.push(IrArg {
                mode: arg.mode,
                expr: self.lower_call_arg_expr(arg, locals, Some(&param.ty))?,
                span: arg.span,
            });
        }
        Ok((
            IrExprKind::Call {
                callee: name.clone(),
                args: lowered_args,
            },
            sig.return_type.clone(),
        ))
    }

    fn lower_len_builtin(
        &self,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        if args.len() != 1 || args[0].mode != ArgMode::Owned {
            return Err(IrError::new(
                "semantic analysis accepted invalid `len` arguments",
                span,
            ));
        }
        let array = self.lower_expr(&args[0].expr, locals)?;
        if !matches!(array.ty, Type::Array { .. }) {
            return Err(IrError::new(
                "semantic analysis accepted `len` on non-array value",
                span,
            ));
        }

        Ok((
            IrExprKind::ArrayLen {
                array: Box::new(array),
            },
            Type::Int,
        ))
    }

    fn lower_method_call(
        &self,
        base: &Expr,
        method_name: &str,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let receiver_probe = self.lower_expr(base, locals)?;
        let key = MethodKey {
            receiver: receiver_probe.ty.clone(),
            name: method_name.to_string(),
        };
        let sig = self.method_sig(&key, span)?;

        let receiver = self.lower_expr_with_expected(base, locals, Some(&sig.receiver.ty))?;
        let mut lowered_args = vec![IrArg {
            mode: arg_mode_for_param(sig.receiver.mode),
            expr: receiver,
            span: base.span,
        }];
        for (arg, param) in args.iter().zip(sig.function.params.iter()) {
            lowered_args.push(IrArg {
                mode: arg.mode,
                expr: self.lower_call_arg_expr(arg, locals, Some(&param.ty))?,
                span: arg.span,
            });
        }

        Ok((
            IrExprKind::Call {
                callee: method_ir_name(&sig.receiver.ty, method_name),
                args: lowered_args,
            },
            sig.function.return_type.clone(),
        ))
    }

    fn lower_call_arg_expr(
        &self,
        arg: &Arg,
        locals: &HashMap<String, Type>,
        expected: Option<&Type>,
    ) -> Result<IrExpr, IrError> {
        match arg.mode {
            ArgMode::Owned => self.lower_expr_with_expected(&arg.expr, locals, expected),
            ArgMode::Con | ArgMode::Mut => {
                self.lower_borrow_expr_with_expected(&arg.expr, locals, expected)
            }
        }
    }

    fn function_sig(&self, name: &str, span: Span) -> Result<&FunctionSig, IrError> {
        self.checked
            .signatures
            .get(name)
            .ok_or_else(|| IrError::new(format!("unknown function `{name}`"), span))
    }

    fn method_sig(&self, key: &MethodKey, span: Span) -> Result<&MethodSig, IrError> {
        self.checked.methods.get(key).ok_or_else(|| {
            IrError::new(
                format!(
                    "unknown method `{}` on `{}`",
                    key.name,
                    key.receiver.source_name()
                ),
                span,
            )
        })
    }

    fn callable_sig(
        &self,
        function: &Function,
    ) -> Result<(Option<&ParamSig>, &FunctionSig, String), IrError> {
        if let Some(receiver) = &function.receiver {
            let receiver_ty = self.type_from_ref(&receiver.ty)?;
            let key = MethodKey {
                receiver: receiver_ty,
                name: function.name.clone(),
            };
            let method = self.method_sig(&key, function.span)?;
            Ok((
                Some(&method.receiver),
                &method.function,
                method_ir_name(&method.receiver.ty, &function.name),
            ))
        } else {
            Ok((
                None,
                self.function_sig(&function.name, function.span)?,
                function.name.clone(),
            ))
        }
    }

    fn type_from_ref(&self, ty: &crate::ast::TypeRef) -> Result<Type, IrError> {
        if let Some(len) = ty.array_len {
            if ty.name != "Array" || ty.args.len() != 1 {
                return Err(IrError::new(
                    "semantic analysis accepted malformed fixed-size array type reference",
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
            "Option" if ty.args.len() == 1 => {
                Ok(Type::Option(Box::new(self.type_from_ref(&ty.args[0])?)))
            }
            "Result" if ty.args.len() == 2 => Ok(Type::Result(
                Box::new(self.type_from_ref(&ty.args[0])?),
                Box::new(self.type_from_ref(&ty.args[1])?),
            )),
            name if ty.args.is_empty() && self.checked.structs.contains_key(name) => {
                Ok(Type::Struct(name.to_string()))
            }
            _ => Err(IrError::new(
                "semantic analysis accepted unknown type reference",
                ty.span,
            )),
        }
    }

    fn struct_sig(&self, name: &str, span: Span) -> Result<&StructSig, IrError> {
        self.checked
            .structs
            .get(name)
            .ok_or_else(|| IrError::new(format!("unknown struct `{name}`"), span))
    }
}

fn lower_param(param: &ParamSig) -> IrParam {
    IrParam {
        name: param.name.clone(),
        mode: param.mode,
        ty: param.ty.clone(),
    }
}

fn arg_mode_for_param(mode: ParamMode) -> ArgMode {
    match mode {
        ParamMode::Owned => ArgMode::Owned,
        ParamMode::Con => ArgMode::Con,
        ParamMode::Mut => ArgMode::Mut,
    }
}

fn method_ir_name(receiver: &Type, method: &str) -> String {
    format!("{}.{}", receiver.source_name(), method)
}

struct PreparedMatchArm<'a> {
    pattern: IrMatchPattern,
    expr: &'a Expr,
    binding: Option<(String, Type)>,
}

struct PreparedMatchBlockArm<'a> {
    pattern: IrMatchPattern,
    block: &'a Block,
    binding: Option<(String, Type)>,
}

struct IrRangeForParts<'a> {
    index_name: &'a str,
    value_name: &'a str,
    source: &'a Expr,
    body: &'a Block,
    span: Span,
}

fn expect_constructor_arg<'a>(
    constructor: &str,
    args: &'a [Arg],
    span: Span,
) -> Result<&'a Arg, IrError> {
    if args.len() != 1 {
        return Err(IrError::new(
            format!("`{constructor}` expects exactly one argument"),
            span,
        ));
    }
    let arg = &args[0];
    if !matches!(arg.mode, ArgMode::Owned) {
        return Err(IrError::new(
            format!("`{constructor}` expects an owned argument"),
            arg.span,
        ));
    }
    Ok(arg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{check, parse};

    #[test]
    fn ir_lowers_first_target_program_with_types() {
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

        assert_eq!(ir.functions.len(), 2);
        assert_eq!(ir.functions[0].name, "main");
        assert_eq!(ir.functions[1].return_type, Type::Int);
        assert_eq!(ir.functions[1].params.len(), 2);

        let IrStmtKind::Let { ty, .. } = &ir.functions[0].body[0].kind else {
            panic!("expected typed let");
        };
        assert_eq!(*ty, Type::Int);
    }

    #[test]
    fn ir_lowers_if_expression_with_branch_type() {
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

        let IrStmtKind::Let { ty, expr, .. } = &ir.functions[0].body[0].kind else {
            panic!("expected typed let");
        };
        assert_eq!(*ty, Type::String);
        assert!(matches!(expr.kind, IrExprKind::If { .. }));
    }

    #[test]
    fn ir_lowers_if_statement() {
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

        let IrStmtKind::If {
            condition,
            then_body,
            else_body,
        } = &ir.functions[0].body[0].kind
        else {
            panic!("expected if statement");
        };
        assert_eq!(condition.ty, Type::Bool);
        assert_eq!(then_body.len(), 1);
        assert_eq!(else_body.len(), 1);
    }

    #[test]
    fn ir_lowers_for_statement() {
        let program = parse(
            r#"
func main() {
    mut count := 0
    for count < 3 {
        count = count + 1
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::For {
            init,
            condition,
            post,
            body,
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        assert!(post.is_none());
        let condition = condition.as_deref().expect("expected for condition");
        assert_eq!(condition.ty, Type::Bool);
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn ir_lowers_for_statement_without_condition() {
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

        let IrStmtKind::For {
            init,
            condition,
            post,
            body,
        } = &ir.functions[0].body[0].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        assert!(condition.is_none());
        assert!(post.is_none());
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn ir_lowers_for_clause_statement() {
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

        let IrStmtKind::For {
            init,
            condition,
            post,
            body,
        } = &ir.functions[0].body[0].kind
        else {
            panic!("expected for statement");
        };
        assert!(matches!(
            init.as_deref(),
            Some(IrForInit::Let {
                mutable: true,
                name,
                ty: Type::Int,
                ..
            }) if name == "i"
        ));
        let condition = condition.as_deref().expect("expected for condition");
        assert_eq!(condition.ty, Type::Bool);
        assert!(matches!(post.as_deref(), Some(IrForPost::Assign { .. })));
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn ir_lowers_initless_for_clause_statement() {
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

        let IrStmtKind::For {
            init,
            condition,
            post,
            body,
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        let condition = condition.as_deref().expect("expected for condition");
        assert_eq!(condition.ty, Type::Bool);
        assert!(matches!(post.as_deref(), Some(IrForPost::Assign { .. })));
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn ir_lowers_for_clause_without_condition() {
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

        let IrStmtKind::For {
            init,
            condition,
            post,
            body,
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected for statement");
        };
        assert!(init.is_none());
        assert!(condition.is_none());
        assert!(matches!(post.as_deref(), Some(IrForPost::Assign { .. })));
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn ir_lowers_loop_control_statements() {
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

        let IrStmtKind::For { body, .. } = &ir.functions[0].body[0].kind else {
            panic!("expected for statement");
        };
        assert!(matches!(body[0].kind, IrStmtKind::Continue));
        assert!(matches!(body[1].kind, IrStmtKind::Break));
    }

    #[test]
    fn ir_lowers_adt_constructors() {
        let program = parse(
            r#"
func find() Option[int] {
    return None
}

func main() {}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Return { expr } = &ir.functions[0].body[0].kind else {
            panic!("expected return");
        };
        assert!(matches!(
            expr.kind,
            IrExprKind::AdtConstructor {
                constructor: IrAdtConstructor::None,
                ..
            }
        ));
        assert_eq!(expr.ty, Type::Option(Box::new(Type::Int)));
    }

    #[test]
    fn ir_lowers_match_expression() {
        let program = parse(
            r#"
func unwrap(value Option[int]) int {
    return match value {
        case Some(inner) { inner }
        case None { 0 }
    }
}

func main() {}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Return { expr } = &ir.functions[0].body[0].kind else {
            panic!("expected return");
        };
        let IrExprKind::Match { arms, .. } = &expr.kind else {
            panic!("expected match expression");
        };
        assert_eq!(expr.ty, Type::Int);
        assert_eq!(arms.len(), 2);
    }

    #[test]
    fn ir_lowers_match_statement() {
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
    return if flag { Some(1) } else { None }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Match { scrutinee, arms } = &ir.functions[0].body[0].kind else {
            panic!("expected match statement");
        };
        assert_eq!(scrutinee.ty, Type::Option(Box::new(Type::Int)));
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].body.len(), 1);
    }

    #[test]
    fn ir_lowers_struct_literal_and_field_access() {
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

        assert_eq!(ir.structs.len(), 1);
        assert_eq!(ir.structs[0].name, "User");
        let IrStmtKind::Let { ty, expr, .. } = &ir.functions[0].body[0].kind else {
            panic!("expected typed let");
        };
        assert_eq!(*ty, Type::Struct("User".to_string()));
        assert!(matches!(expr.kind, IrExprKind::StructLiteral { .. }));

        let IrStmtKind::Expr { expr } = &ir.functions[0].body[1].kind else {
            panic!("expected print expression");
        };
        let IrExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected print call");
        };
        assert_eq!(args[0].expr.ty, Type::Int);
        assert!(matches!(args[0].expr.kind, IrExprKind::FieldAccess { .. }));
    }

    #[test]
    fn ir_lowers_method_declarations_and_calls() {
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

        assert_eq!(ir.functions[0].name, "User.age");
        assert_eq!(ir.functions[0].params.len(), 1);
        assert_eq!(ir.functions[0].params[0].mode, ParamMode::Con);

        let IrStmtKind::Expr { expr } = &ir.functions[1].body[1].kind else {
            panic!("expected print expression");
        };
        let IrExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected print call");
        };
        let IrExprKind::Call { callee, args } = &args[0].expr.kind else {
            panic!("expected method call");
        };
        assert_eq!(callee, "User.age");
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].mode, ArgMode::Con);
    }

    #[test]
    fn ir_lowers_field_assignment() {
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

        let IrStmtKind::FieldAssign { base, field, expr } = &ir.functions[0].body[1].kind else {
            panic!("expected field assignment");
        };
        assert_eq!(base.ty, Type::Struct("User".to_string()));
        assert_eq!(field, "age");
        assert_eq!(expr.ty, Type::Int);
    }

    #[test]
    fn ir_lowers_fixed_size_array_literals() {
        let program = parse(
            r#"
func main() {
    values := [2]int{1, 2}
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let { ty, expr, .. } = &ir.functions[0].body[0].kind else {
            panic!("expected let statement");
        };
        let Type::Array { len, element } = ty else {
            panic!("expected array type");
        };
        assert_eq!(*len, 2);
        assert_eq!(**element, Type::Int);

        let IrExprKind::ArrayLiteral { elements } = &expr.kind else {
            panic!("expected array literal");
        };
        assert_eq!(elements.len(), 2);
        assert_eq!(expr.ty, ty.clone());
    }

    #[test]
    fn ir_lowers_array_range_loops() {
        let program = parse(
            r#"
func main() {
    values := [2]int{1, 2}
    for i, value := range values {
        print(i)
        print(value)
    }
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::RangeFor {
            index_name,
            value_name,
            source,
            element_ty,
            body,
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "i");
        assert_eq!(value_name, "value");
        assert!(matches!(source.ty, Type::Array { .. }));
        assert_eq!(*element_ty, Type::Int);
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn ir_lowers_fixed_size_array_indexing_and_len() {
        let program = parse(
            r#"
func main() {
    values := [2]int{1, 2}
    first := values[1]
    count := len(values)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let { expr, ty, .. } = &ir.functions[0].body[1].kind else {
            panic!("expected let statement");
        };
        assert_eq!(*ty, Type::Int);
        let IrExprKind::Index { base, index } = &expr.kind else {
            panic!("expected index expression");
        };
        assert!(matches!(base.ty, Type::Array { .. }));
        assert_eq!(index.ty, Type::Int);

        let IrStmtKind::Let { expr, ty, .. } = &ir.functions[0].body[2].kind else {
            panic!("expected let statement");
        };
        assert_eq!(*ty, Type::Int);
        let IrExprKind::ArrayLen { array } = &expr.kind else {
            panic!("expected array len expression");
        };
        assert!(matches!(array.ty, Type::Array { .. }));
    }

    #[test]
    fn ir_lowers_array_element_borrow_arguments() {
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
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Expr { expr } = &ir.functions[0].body[1].kind else {
            panic!("expected call statement");
        };
        let IrExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected call expression");
        };
        assert_eq!(args[0].mode, ArgMode::Con);
        assert_eq!(args[0].expr.ty, Type::String);
        assert!(matches!(args[0].expr.kind, IrExprKind::FieldAccess { .. }));

        let IrStmtKind::Expr { expr } = &ir.functions[0].body[2].kind else {
            panic!("expected call statement");
        };
        let IrExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected call expression");
        };
        assert_eq!(args[0].mode, ArgMode::Mut);
        assert_eq!(args[0].expr.ty, Type::String);
        assert!(matches!(args[0].expr.kind, IrExprKind::FieldAccess { .. }));
    }

    #[test]
    fn ir_lowers_fixed_size_array_element_assignment() {
        let program = parse(
            r#"
func main() {
    mut values := [2]int{1, 2}
    index := 1
    values[index] = 5
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::IndexAssign { base, index, expr } = &ir.functions[0].body[2].kind else {
            panic!("expected index assignment");
        };
        assert!(matches!(base.ty, Type::Array { .. }));
        assert_eq!(index.ty, Type::Int);
        assert_eq!(expr.ty, Type::Int);
    }

    #[test]
    fn ir_lowers_fixed_size_array_element_assignment_in_for_post() {
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
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::For { post, .. } = &ir.functions[0].body[3].kind else {
            panic!("expected for statement");
        };
        let Some(IrForPost::Assign { target, expr }) = post.as_deref() else {
            panic!("expected for post assignment");
        };
        let IrExprKind::Index { base, index } = &target.kind else {
            panic!("expected index assignment target");
        };
        assert!(matches!(&base.kind, IrExprKind::Var(name) if name == "values"));
        assert!(matches!(&index.kind, IrExprKind::Var(name) if name == "slot"));
        assert_eq!(index.ty, Type::Int);
        assert_eq!(expr.ty, Type::Int);
    }
}
