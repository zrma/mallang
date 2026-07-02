use std::{
    collections::{HashMap, HashSet},
    fmt,
};

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
        cleanup: Vec<IrStmt>,
    },
    RangeFor {
        index_name: String,
        value_name: String,
        source: IrExpr,
        element_ty: Type,
        body: Vec<IrStmt>,
    },
    Drop {
        expr: IrExpr,
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
        then_cleanup: Vec<IrStmt>,
        else_branch: Box<IrExpr>,
        else_cleanup: Vec<IrStmt>,
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
    SliceFieldTake {
        source: Box<IrExpr>,
    },
    Index {
        base: Box<IrExpr>,
        index: Box<IrExpr>,
    },
    ArrayLen {
        array: Box<IrExpr>,
    },
    SliceAppend {
        slice: Box<IrExpr>,
        item: Box<IrExpr>,
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
    pub cleanup: Vec<IrStmt>,
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
            let body = insert_straight_line_cleanup_drops(body, &params, function.body.span);
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
                let base = self.lower_assignment_target_expr(base, locals)?;
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
                let base = self.lower_assignment_target_expr(base, locals)?;
                let element = match &base.ty {
                    Type::Array { element, .. } | Type::Slice(element) => element,
                    _ => {
                        return Err(IrError::new(
                            "semantic analysis accepted indexed assignment on non-array non-slice value",
                            stmt.span,
                        ));
                    }
                };
                let index = self.lower_expr(index, locals)?;
                if index.ty != Type::Int {
                    return Err(IrError::new(
                        "semantic analysis accepted indexed assignment with non-int index",
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
                    cleanup: Vec::new(),
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
        let source = self.lower_read_source_expr(parts.source, locals)?;
        let element = match &source.ty {
            Type::Array { element, .. } | Type::Slice(element) => element,
            _ => {
                return Err(IrError::new(
                    "semantic analysis accepted range over non-array non-slice source",
                    parts.span,
                ));
            }
        };
        if !is_blank_identifier(parts.value_name) && !element.is_copy() {
            return Err(IrError::new(
                "semantic analysis accepted range over non-Copy element type",
                parts.span,
            ));
        }

        let element_ty = element.as_ref().clone();
        let mut body_locals = locals.clone();
        if !is_blank_identifier(parts.index_name) {
            body_locals.insert(parts.index_name.to_string(), Type::Int);
        }
        if !is_blank_identifier(parts.value_name) {
            body_locals.insert(parts.value_name.to_string(), element_ty.clone());
        }
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
                let target = self.lower_assignment_target_expr(target, locals)?;
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
                        "semantic analysis accepted element borrow with non-int index",
                        index.span,
                    ));
                }
                let element = match &base.ty {
                    Type::Array { element, .. } | Type::Slice(element) => element,
                    _ => {
                        return Err(IrError::new(
                            "semantic analysis accepted element borrow on non-array non-slice value",
                            expr.span,
                        ));
                    }
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

    fn lower_assignment_target_expr(
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
                let base = self.lower_assignment_target_expr(base, locals)?;
                let Type::Struct(type_name) = &base.ty else {
                    return Err(IrError::new(
                        "semantic analysis accepted field assignment target on non-struct value",
                        expr.span,
                    ));
                };
                let sig = self.struct_sig(type_name, expr.span)?;
                let Some(field_sig) = sig.fields.iter().find(|candidate| candidate.name == *field)
                else {
                    return Err(IrError::new(
                        "semantic analysis accepted unknown field assignment target",
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
                let base = self.lower_assignment_target_expr(base, locals)?;
                let index = self.lower_expr(index, locals)?;
                if index.ty != Type::Int {
                    return Err(IrError::new(
                        "semantic analysis accepted indexed assignment with non-int index",
                        index.span,
                    ));
                }
                let element = match &base.ty {
                    Type::Array { element, .. } | Type::Slice(element) => element,
                    _ => {
                        return Err(IrError::new(
                            "semantic analysis accepted indexed assignment target on non-array non-slice value",
                            expr.span,
                        ));
                    }
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
                    "semantic analysis accepted invalid assignment target expression",
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
                then_cleanup: Vec::new(),
                else_branch: Box::new(else_branch),
                else_cleanup: Vec::new(),
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

        let element = match &ty {
            Type::Array { len, element } => {
                if elements.len() != *len {
                    return Err(IrError::new(
                        "semantic analysis accepted array literal length mismatch",
                        span,
                    ));
                }
                element.as_ref()
            }
            Type::Slice(element) => element.as_ref(),
            _ => {
                return Err(IrError::new(
                    "semantic analysis accepted array literal without array or slice type",
                    ty_ref.span,
                ));
            }
        };

        let mut lowered = Vec::new();
        for element_expr in elements {
            let expr = self.lower_expr_with_expected(element_expr, locals, Some(element))?;
            if expr.ty != *element {
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
        if is_direct_borrow_expr(base) {
            let source = self.lower_borrow_field_access(base, field, locals, span)?;
            if matches!(source.ty, Type::Slice(_)) {
                let ty = source.ty.clone();
                return Ok((
                    IrExprKind::SliceFieldTake {
                        source: Box::new(source),
                    },
                    ty,
                ));
            }
        }

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

    fn lower_borrow_field_access(
        &self,
        base: &Expr,
        field: &str,
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<IrExpr, IrError> {
        let base = self.lower_borrow_expr(base, locals)?;
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

        Ok(IrExpr {
            kind: IrExprKind::FieldAccess {
                base: Box::new(base),
                field: field.to_string(),
            },
            ty: field_sig.ty.clone(),
            span,
        })
    }

    fn lower_index_access(
        &self,
        base: &Expr,
        index: &Expr,
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let base = self.lower_read_source_expr(base, locals)?;
        let index = self.lower_expr(index, locals)?;
        if index.ty != Type::Int {
            return Err(IrError::new(
                "semantic analysis accepted array index with non-int type",
                index.span,
            ));
        }
        let ty = match &base.ty {
            Type::Array { element, .. } | Type::Slice(element) => (**element).clone(),
            _ => {
                return Err(IrError::new(
                    "semantic analysis accepted indexing on non-array non-slice value",
                    span,
                ));
            }
        };
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
            cleanup: Vec::new(),
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
        if name == "append" {
            return self.lower_append_builtin(args, locals, span);
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
        let array = self.lower_read_source_expr(&args[0].expr, locals)?;
        if !matches!(array.ty, Type::Array { .. } | Type::Slice(_)) {
            return Err(IrError::new(
                "semantic analysis accepted `len` on non-array non-slice value",
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

    fn lower_read_source_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<IrExpr, IrError> {
        if is_direct_borrow_expr(expr) {
            self.lower_borrow_expr(expr, locals)
        } else {
            self.lower_expr(expr, locals)
        }
    }

    fn lower_append_builtin(
        &self,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        if args.len() != 2 || args[0].mode != ArgMode::Owned || args[1].mode != ArgMode::Owned {
            return Err(IrError::new(
                "semantic analysis accepted invalid `append` arguments",
                span,
            ));
        }
        let slice = if is_field_place_expr(&args[0].expr) {
            self.lower_borrow_expr(&args[0].expr, locals)?
        } else {
            self.lower_expr(&args[0].expr, locals)?
        };
        let Type::Slice(element_ty) = &slice.ty else {
            return Err(IrError::new(
                "semantic analysis accepted `append` on non-slice value",
                args[0].span,
            ));
        };
        let item = self.lower_expr_with_expected(&args[1].expr, locals, Some(element_ty))?;
        if item.ty != **element_ty {
            return Err(IrError::new(
                "semantic analysis accepted `append` item type mismatch",
                args[1].span,
            ));
        }
        let ty = slice.ty.clone();

        Ok((
            IrExprKind::SliceAppend {
                slice: Box::new(slice),
                item: Box::new(item),
            },
            ty,
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
        let receiver_probe = self.lower_receiver_probe_expr(base, locals)?;
        let key = MethodKey {
            receiver: receiver_probe.ty.clone(),
            name: method_name.to_string(),
        };
        let sig = self.method_sig(&key, span)?;

        let receiver_mode = arg_mode_for_param(sig.receiver.mode);
        let receiver = match receiver_mode {
            ArgMode::Owned => {
                self.lower_expr_with_expected(base, locals, Some(&sig.receiver.ty))?
            }
            ArgMode::Con | ArgMode::Mut => {
                self.lower_borrow_expr_with_expected(base, locals, Some(&sig.receiver.ty))?
            }
        };
        let mut lowered_args = vec![IrArg {
            mode: receiver_mode,
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

    fn lower_receiver_probe_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<IrExpr, IrError> {
        if is_direct_borrow_expr(expr) {
            return self.lower_borrow_expr(expr, locals);
        }
        self.lower_expr(expr, locals)
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
        if ty.slice {
            if ty.name != "Slice" || ty.args.len() != 1 || ty.array_len.is_some() {
                return Err(IrError::new(
                    "semantic analysis accepted malformed slice type reference",
                    ty.span,
                ));
            }
            return Ok(Type::Slice(Box::new(self.type_from_ref(&ty.args[0])?)));
        }

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupBinding {
    name: String,
    ty: Type,
    span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupInsertion {
    body: Vec<IrStmt>,
    moved_roots: HashSet<String>,
    continues: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExprCleanupInsertion {
    expr: IrExpr,
    moved_roots: HashSet<String>,
}

fn insert_straight_line_cleanup_drops(
    body: Vec<IrStmt>,
    params: &[IrParam],
    fallback_span: Span,
) -> Vec<IrStmt> {
    let active = params
        .iter()
        .filter(|param| param.mode == ParamMode::Owned && param.ty.needs_cleanup())
        .map(|param| CleanupBinding {
            name: param.name.clone(),
            ty: param.ty.clone(),
            span: fallback_span,
        })
        .collect::<Vec<_>>();

    insert_cleanup_drops(
        body,
        active,
        fallback_span,
        &HashSet::new(),
        &HashSet::new(),
        &HashSet::new(),
    )
    .body
}

fn insert_cleanup_drops(
    body: Vec<IrStmt>,
    initial_active: Vec<CleanupBinding>,
    fallback_span: Span,
    tail_excluded_roots: &HashSet<String>,
    break_excluded_roots: &HashSet<String>,
    continue_excluded_roots: &HashSet<String>,
) -> CleanupInsertion {
    let mut output = Vec::new();
    let mut active = initial_active;
    let mut moved_in_body = HashSet::new();

    let mut statements = body.into_iter();
    while let Some(stmt) = statements.next() {
        let (stmt, branch_moved_roots) = insert_branch_cleanup_drops(
            stmt,
            &active,
            break_excluded_roots,
            continue_excluded_roots,
        );
        let (stmt, stmt_moved_roots) = insert_cleanup_drops_in_stmt_exprs(stmt, &active);
        if let IrStmtKind::Return { expr: _ } = &stmt.kind {
            let returned_roots = stmt_moved_roots;
            moved_in_body.extend(returned_roots.iter().cloned());
            push_cleanup_drops(&mut output, &active, &returned_roots, stmt.span);
            output.push(stmt);
            return CleanupInsertion {
                body: output,
                moved_roots: moved_in_body,
                continues: false,
            };
        }
        if matches!(stmt.kind, IrStmtKind::Break) {
            push_cleanup_drops(&mut output, &active, break_excluded_roots, stmt.span);
            output.push(stmt);
            output.extend(statements);
            return CleanupInsertion {
                body: output,
                moved_roots: moved_in_body,
                continues: false,
            };
        }
        if matches!(stmt.kind, IrStmtKind::Continue) {
            push_cleanup_drops(&mut output, &active, continue_excluded_roots, stmt.span);
            output.push(stmt);
            output.extend(statements);
            return CleanupInsertion {
                body: output,
                moved_roots: moved_in_body,
                continues: false,
            };
        }

        let moved_roots = stmt_moved_roots
            .into_iter()
            .chain(branch_moved_roots)
            .collect::<HashSet<_>>();
        let new_binding = cleanup_binding_from_stmt(&stmt);
        let assigned_binding = cleanup_assigned_binding_from_stmt(&stmt);
        let reassigned_binding =
            cleanup_reassigned_active_binding(&stmt, &active, &moved_roots).cloned();
        let overwritten_place = cleanup_overwritten_place_from_stmt(&stmt);
        let (stmt, rhs_temp) = if reassigned_binding.is_some() || overwritten_place.is_some() {
            prepare_cleanup_assignment_rhs(stmt)
        } else {
            (stmt, None)
        };

        if let Some(temp) = rhs_temp {
            output.push(temp);
        }
        if let Some(binding) = &reassigned_binding {
            push_cleanup_drop(&mut output, binding, stmt.span);
        }
        if let Some(overwritten_place) = overwritten_place {
            push_cleanup_drop_expr(&mut output, overwritten_place, stmt.span);
        }
        output.push(stmt);
        moved_in_body.extend(moved_roots.iter().cloned());
        active.retain(|binding| !moved_roots.contains(&binding.name));
        if let Some(binding) = new_binding {
            active.push(binding);
        }
        if let Some(binding) = assigned_binding {
            if !active.iter().any(|active| active.name == binding.name) {
                active.push(binding);
            }
        }
    }

    push_cleanup_drops(&mut output, &active, tail_excluded_roots, fallback_span);
    CleanupInsertion {
        body: output,
        moved_roots: moved_in_body,
        continues: true,
    }
}

fn insert_cleanup_drops_in_stmt_exprs(
    mut stmt: IrStmt,
    active: &[CleanupBinding],
) -> (IrStmt, HashSet<String>) {
    let moved_roots = match stmt.kind {
        IrStmtKind::Let {
            mutable,
            name,
            ty,
            expr,
        } => {
            let insertion = insert_expr_cleanup_drops(expr, active);
            let moved_roots = insertion.moved_roots;
            stmt.kind = IrStmtKind::Let {
                mutable,
                name,
                ty,
                expr: insertion.expr,
            };
            moved_roots
        }
        IrStmtKind::Assign { name, expr } => {
            let insertion = insert_expr_cleanup_drops(expr, active);
            let moved_roots = insertion.moved_roots;
            stmt.kind = IrStmtKind::Assign {
                name,
                expr: insertion.expr,
            };
            moved_roots
        }
        IrStmtKind::FieldAssign { base, field, expr } => {
            let insertion = insert_expr_cleanup_drops(expr, active);
            let moved_roots = insertion.moved_roots;
            stmt.kind = IrStmtKind::FieldAssign {
                base,
                field,
                expr: insertion.expr,
            };
            moved_roots
        }
        IrStmtKind::IndexAssign { base, index, expr } => {
            let insertion = insert_expr_cleanup_drops(expr, active);
            let moved_roots = insertion.moved_roots;
            stmt.kind = IrStmtKind::IndexAssign {
                base,
                index,
                expr: insertion.expr,
            };
            moved_roots
        }
        IrStmtKind::Return { expr } => {
            let insertion = insert_expr_cleanup_drops(expr, active);
            let moved_roots = insertion.moved_roots;
            stmt.kind = IrStmtKind::Return {
                expr: insertion.expr,
            };
            moved_roots
        }
        IrStmtKind::Drop { expr } => {
            let insertion = insert_expr_cleanup_drops(expr, active);
            let moved_roots = insertion.moved_roots;
            stmt.kind = IrStmtKind::Drop {
                expr: insertion.expr,
            };
            moved_roots
        }
        IrStmtKind::Expr { expr } => {
            let insertion = insert_expr_cleanup_drops(expr, active);
            let moved_roots = insertion.moved_roots;
            stmt.kind = IrStmtKind::Expr {
                expr: insertion.expr,
            };
            moved_roots
        }
        IrStmtKind::If { .. }
        | IrStmtKind::For { .. }
        | IrStmtKind::RangeFor { .. }
        | IrStmtKind::Break
        | IrStmtKind::Continue
        | IrStmtKind::Match { .. } => HashSet::new(),
    };
    (stmt, moved_roots)
}

fn insert_branch_cleanup_drops(
    mut stmt: IrStmt,
    active: &[CleanupBinding],
    break_excluded_roots: &HashSet<String>,
    continue_excluded_roots: &HashSet<String>,
) -> (IrStmt, HashSet<String>) {
    let mut moved_roots = HashSet::new();
    let span = stmt.span;

    stmt.kind = match stmt.kind {
        IrStmtKind::If {
            condition,
            then_body,
            else_body,
        } => {
            let condition_insertion = insert_expr_cleanup_drops(condition, active);
            let condition_moved_roots = condition_insertion.moved_roots;
            let branch_active = cleanup_bindings_after_moved_roots(active, &condition_moved_roots);
            let tail_excluded_roots = cleanup_binding_names(&branch_active);

            let mut then_insertion = insert_cleanup_drops(
                then_body,
                branch_active.clone(),
                span,
                &tail_excluded_roots,
                break_excluded_roots,
                continue_excluded_roots,
            );
            let mut else_insertion = insert_cleanup_drops(
                else_body,
                branch_active.clone(),
                span,
                &tail_excluded_roots,
                break_excluded_roots,
                continue_excluded_roots,
            );
            let branch_moved_roots =
                merged_cleanup_roots(&branch_active, [&then_insertion, &else_insertion]);

            push_branch_merge_cleanup_drops(
                &mut then_insertion.body,
                &branch_active,
                &branch_moved_roots,
                &then_insertion.moved_roots,
                span,
                then_insertion.continues,
            );
            push_branch_merge_cleanup_drops(
                &mut else_insertion.body,
                &branch_active,
                &branch_moved_roots,
                &else_insertion.moved_roots,
                span,
                else_insertion.continues,
            );

            moved_roots.extend(condition_moved_roots);
            moved_roots.extend(branch_moved_roots);

            IrStmtKind::If {
                condition: condition_insertion.expr,
                then_body: then_insertion.body,
                else_body: else_insertion.body,
            }
        }
        IrStmtKind::For {
            init,
            condition,
            post,
            body,
            cleanup: existing_cleanup,
        } => {
            let init_moved_roots = init
                .as_deref()
                .map(cleanup_moved_roots_in_for_init)
                .unwrap_or_default();
            let mut loop_active = cleanup_bindings_after_moved_roots(active, &init_moved_roots);
            let pre_loop_roots = cleanup_binding_names(&loop_active);
            if let Some(binding) = init.as_deref().and_then(cleanup_binding_from_for_init) {
                loop_active.push(binding);
            }
            let loop_persistent_roots = cleanup_binding_names(&loop_active);
            let insertion = insert_cleanup_drops(
                body,
                loop_active.clone(),
                span,
                &loop_persistent_roots,
                &loop_persistent_roots,
                &loop_persistent_roots,
            );
            let mut cleanup = existing_cleanup;
            push_loop_cleanup_drops(
                &mut cleanup,
                &loop_active,
                &pre_loop_roots,
                &insertion.moved_roots,
                span,
            );
            moved_roots.extend(init_moved_roots);

            IrStmtKind::For {
                init,
                condition,
                post,
                body: insertion.body,
                cleanup,
            }
        }
        IrStmtKind::RangeFor {
            index_name,
            value_name,
            source,
            element_ty,
            body,
        } => {
            let loop_active = active.to_vec();
            let loop_excluded_roots = cleanup_binding_names(&loop_active);
            let insertion = insert_cleanup_drops(
                body,
                loop_active,
                span,
                &loop_excluded_roots,
                &loop_excluded_roots,
                &loop_excluded_roots,
            );

            IrStmtKind::RangeFor {
                index_name,
                value_name,
                source,
                element_ty,
                body: insertion.body,
            }
        }
        IrStmtKind::Match { scrutinee, arms } => {
            let scrutinee_insertion = insert_expr_cleanup_drops(scrutinee, active);
            let scrutinee_moved_roots = scrutinee_insertion.moved_roots;
            let arm_active = cleanup_bindings_after_moved_roots(active, &scrutinee_moved_roots);
            let tail_excluded_roots = cleanup_binding_names(&arm_active);
            let mut arms = arms
                .into_iter()
                .map(|arm| {
                    let insertion = insert_cleanup_drops(
                        arm.body,
                        arm_active.clone(),
                        arm.span,
                        &tail_excluded_roots,
                        break_excluded_roots,
                        continue_excluded_roots,
                    );
                    (arm.pattern, insertion, arm.span)
                })
                .collect::<Vec<_>>();
            let arm_moved_roots =
                merged_cleanup_roots(&arm_active, arms.iter().map(|(_, insertion, _)| insertion));

            for (_, insertion, span) in &mut arms {
                push_branch_merge_cleanup_drops(
                    &mut insertion.body,
                    &arm_active,
                    &arm_moved_roots,
                    &insertion.moved_roots,
                    *span,
                    insertion.continues,
                );
            }

            moved_roots.extend(scrutinee_moved_roots);
            moved_roots.extend(arm_moved_roots);

            IrStmtKind::Match {
                scrutinee: scrutinee_insertion.expr,
                arms: arms
                    .into_iter()
                    .map(|(pattern, insertion, span)| IrMatchBlockArm {
                        pattern,
                        body: insertion.body,
                        span,
                    })
                    .collect(),
            }
        }
        kind => kind,
    };
    (stmt, moved_roots)
}

fn cleanup_bindings_after_moved_roots(
    active: &[CleanupBinding],
    moved_roots: &HashSet<String>,
) -> Vec<CleanupBinding> {
    active
        .iter()
        .filter(|binding| !moved_roots.contains(&binding.name))
        .cloned()
        .collect()
}

fn cleanup_binding_names(active: &[CleanupBinding]) -> HashSet<String> {
    active.iter().map(|binding| binding.name.clone()).collect()
}

fn merged_cleanup_roots<'a>(
    active: &[CleanupBinding],
    insertions: impl IntoIterator<Item = &'a CleanupInsertion>,
) -> HashSet<String> {
    let active_names = cleanup_binding_names(active);
    let mut moved_roots = HashSet::new();
    for insertion in insertions {
        moved_roots.extend(
            insertion
                .moved_roots
                .iter()
                .filter(|root| active_names.contains(*root))
                .cloned(),
        );
    }
    moved_roots
}

fn merged_cleanup_expr_roots<'a>(
    active: &[CleanupBinding],
    insertions: impl IntoIterator<Item = &'a ExprCleanupInsertion>,
) -> HashSet<String> {
    let active_names = cleanup_binding_names(active);
    let mut moved_roots = HashSet::new();
    for insertion in insertions {
        moved_roots.extend(
            insertion
                .moved_roots
                .iter()
                .filter(|root| active_names.contains(*root))
                .cloned(),
        );
    }
    moved_roots
}

fn push_branch_merge_cleanup_drops(
    body: &mut Vec<IrStmt>,
    active: &[CleanupBinding],
    merged_moved_roots: &HashSet<String>,
    branch_moved_roots: &HashSet<String>,
    span: Span,
    branch_continues: bool,
) {
    if !branch_continues {
        return;
    }
    for binding in active.iter().rev() {
        if merged_moved_roots.contains(&binding.name) && !branch_moved_roots.contains(&binding.name)
        {
            push_cleanup_drop(body, binding, span);
        }
    }
}

fn push_expr_branch_cleanup_drops(
    cleanup: &mut Vec<IrStmt>,
    active: &[CleanupBinding],
    merged_moved_roots: &HashSet<String>,
    branch_moved_roots: &HashSet<String>,
    span: Span,
) {
    for binding in active.iter().rev() {
        if merged_moved_roots.contains(&binding.name) && !branch_moved_roots.contains(&binding.name)
        {
            push_cleanup_drop(cleanup, binding, span);
        }
    }
}

fn insert_expr_cleanup_drops(expr: IrExpr, active: &[CleanupBinding]) -> ExprCleanupInsertion {
    let ty = expr.ty;
    let span = expr.span;
    let mut moved_roots = HashSet::new();

    let kind = match expr.kind {
        IrExprKind::Var(name) => {
            if ty.needs_cleanup() {
                moved_roots.insert(name.clone());
            }
            IrExprKind::Var(name)
        }
        IrExprKind::If {
            condition,
            then_branch,
            mut then_cleanup,
            else_branch,
            mut else_cleanup,
        } => {
            let condition = insert_expr_cleanup_drops(*condition, active);
            let branch_active = cleanup_bindings_after_moved_roots(active, &condition.moved_roots);
            let then_branch = insert_expr_cleanup_drops(*then_branch, &branch_active);
            let else_branch = insert_expr_cleanup_drops(*else_branch, &branch_active);
            let branch_moved_roots =
                merged_cleanup_expr_roots(&branch_active, [&then_branch, &else_branch]);

            push_expr_branch_cleanup_drops(
                &mut then_cleanup,
                &branch_active,
                &branch_moved_roots,
                &then_branch.moved_roots,
                span,
            );
            push_expr_branch_cleanup_drops(
                &mut else_cleanup,
                &branch_active,
                &branch_moved_roots,
                &else_branch.moved_roots,
                span,
            );

            moved_roots.extend(condition.moved_roots);
            moved_roots.extend(branch_moved_roots);

            IrExprKind::If {
                condition: Box::new(condition.expr),
                then_branch: Box::new(then_branch.expr),
                then_cleanup,
                else_branch: Box::new(else_branch.expr),
                else_cleanup,
            }
        }
        IrExprKind::AdtConstructor {
            constructor,
            payload,
        } => {
            let payload = payload.map(|payload| {
                let insertion = insert_expr_cleanup_drops(*payload, active);
                moved_roots.extend(insertion.moved_roots);
                Box::new(insertion.expr)
            });
            IrExprKind::AdtConstructor {
                constructor,
                payload,
            }
        }
        IrExprKind::Match { scrutinee, arms } => {
            let scrutinee = insert_expr_cleanup_drops(*scrutinee, active);
            let arm_active = cleanup_bindings_after_moved_roots(active, &scrutinee.moved_roots);
            let mut arms = arms
                .into_iter()
                .map(|arm| {
                    let expr = insert_expr_cleanup_drops(arm.expr, &arm_active);
                    (arm.pattern, expr, arm.cleanup, arm.span)
                })
                .collect::<Vec<_>>();
            let arm_moved_roots =
                merged_cleanup_expr_roots(&arm_active, arms.iter().map(|(_, expr, _, _)| expr));

            for (_, expr, cleanup, span) in &mut arms {
                push_expr_branch_cleanup_drops(
                    cleanup,
                    &arm_active,
                    &arm_moved_roots,
                    &expr.moved_roots,
                    *span,
                );
            }

            moved_roots.extend(scrutinee.moved_roots);
            moved_roots.extend(arm_moved_roots);

            IrExprKind::Match {
                scrutinee: Box::new(scrutinee.expr),
                arms: arms
                    .into_iter()
                    .map(|(pattern, expr, cleanup, span)| IrMatchArm {
                        pattern,
                        expr: expr.expr,
                        cleanup,
                        span,
                    })
                    .collect(),
            }
        }
        IrExprKind::StructLiteral { type_name, fields } => {
            let mut active = active.to_vec();
            let fields = fields
                .into_iter()
                .map(|field| {
                    let insertion = insert_expr_cleanup_drops(field.expr, &active);
                    moved_roots.extend(insertion.moved_roots.iter().cloned());
                    active = cleanup_bindings_after_moved_roots(&active, &insertion.moved_roots);
                    IrFieldValue {
                        name: field.name,
                        expr: insertion.expr,
                        span: field.span,
                    }
                })
                .collect();
            IrExprKind::StructLiteral { type_name, fields }
        }
        IrExprKind::ArrayLiteral { elements } => {
            let mut active = active.to_vec();
            let elements = elements
                .into_iter()
                .map(|element| {
                    let insertion = insert_expr_cleanup_drops(element, &active);
                    moved_roots.extend(insertion.moved_roots.iter().cloned());
                    active = cleanup_bindings_after_moved_roots(&active, &insertion.moved_roots);
                    insertion.expr
                })
                .collect();
            IrExprKind::ArrayLiteral { elements }
        }
        IrExprKind::FieldAccess { base, field } => {
            let base = insert_place_expr_cleanup_drops(*base, active);
            moved_roots.extend(base.moved_roots);
            IrExprKind::FieldAccess {
                base: Box::new(base.expr),
                field,
            }
        }
        IrExprKind::SliceFieldTake { source } => {
            let source = insert_place_expr_cleanup_drops(*source, active);
            moved_roots.extend(source.moved_roots);
            IrExprKind::SliceFieldTake {
                source: Box::new(source.expr),
            }
        }
        IrExprKind::Index { base, index } => {
            let base = insert_place_expr_cleanup_drops(*base, active);
            let active_after_base = cleanup_bindings_after_moved_roots(active, &base.moved_roots);
            let index = insert_expr_cleanup_drops(*index, &active_after_base);
            moved_roots.extend(base.moved_roots);
            moved_roots.extend(index.moved_roots);
            IrExprKind::Index {
                base: Box::new(base.expr),
                index: Box::new(index.expr),
            }
        }
        IrExprKind::ArrayLen { array } => {
            let array = insert_place_expr_cleanup_drops(*array, active);
            moved_roots.extend(array.moved_roots);
            IrExprKind::ArrayLen {
                array: Box::new(array.expr),
            }
        }
        IrExprKind::SliceAppend { slice, item } => {
            let slice = insert_expr_cleanup_drops(*slice, active);
            let active_after_slice = cleanup_bindings_after_moved_roots(active, &slice.moved_roots);
            let item = insert_expr_cleanup_drops(*item, &active_after_slice);
            moved_roots.extend(slice.moved_roots);
            moved_roots.extend(item.moved_roots);
            IrExprKind::SliceAppend {
                slice: Box::new(slice.expr),
                item: Box::new(item.expr),
            }
        }
        IrExprKind::Call { callee, args } => {
            let mut active = active.to_vec();
            let args = args
                .into_iter()
                .map(|arg| {
                    let insertion = if arg.mode == ArgMode::Owned {
                        insert_expr_cleanup_drops(arg.expr, &active)
                    } else {
                        insert_place_expr_cleanup_drops(arg.expr, &active)
                    };
                    moved_roots.extend(insertion.moved_roots.iter().cloned());
                    active = cleanup_bindings_after_moved_roots(&active, &insertion.moved_roots);
                    IrArg {
                        mode: arg.mode,
                        expr: insertion.expr,
                        span: arg.span,
                    }
                })
                .collect();
            IrExprKind::Call { callee, args }
        }
        IrExprKind::Unary { op, expr } => {
            let expr = insert_expr_cleanup_drops(*expr, active);
            moved_roots.extend(expr.moved_roots);
            IrExprKind::Unary {
                op,
                expr: Box::new(expr.expr),
            }
        }
        IrExprKind::Binary { op, left, right } => {
            let left = insert_expr_cleanup_drops(*left, active);
            let active_after_left = cleanup_bindings_after_moved_roots(active, &left.moved_roots);
            let right = insert_expr_cleanup_drops(*right, &active_after_left);
            moved_roots.extend(left.moved_roots);
            moved_roots.extend(right.moved_roots);
            IrExprKind::Binary {
                op,
                left: Box::new(left.expr),
                right: Box::new(right.expr),
            }
        }
        IrExprKind::Int(value) => IrExprKind::Int(value),
        IrExprKind::String(value) => IrExprKind::String(value),
        IrExprKind::Bool(value) => IrExprKind::Bool(value),
    };

    ExprCleanupInsertion {
        expr: IrExpr { kind, ty, span },
        moved_roots,
    }
}

fn insert_place_expr_cleanup_drops(
    expr: IrExpr,
    active: &[CleanupBinding],
) -> ExprCleanupInsertion {
    let ty = expr.ty;
    let span = expr.span;
    let mut moved_roots = HashSet::new();

    let kind = match expr.kind {
        IrExprKind::Var(name) => IrExprKind::Var(name),
        IrExprKind::FieldAccess { base, field } => {
            let base = insert_place_expr_cleanup_drops(*base, active);
            moved_roots.extend(base.moved_roots);
            IrExprKind::FieldAccess {
                base: Box::new(base.expr),
                field,
            }
        }
        IrExprKind::SliceFieldTake { source } => {
            let source = insert_place_expr_cleanup_drops(*source, active);
            moved_roots.extend(source.moved_roots);
            IrExprKind::SliceFieldTake {
                source: Box::new(source.expr),
            }
        }
        IrExprKind::Index { base, index } => {
            let base = insert_place_expr_cleanup_drops(*base, active);
            let active_after_base = cleanup_bindings_after_moved_roots(active, &base.moved_roots);
            let index = insert_expr_cleanup_drops(*index, &active_after_base);
            moved_roots.extend(base.moved_roots);
            moved_roots.extend(index.moved_roots);
            IrExprKind::Index {
                base: Box::new(base.expr),
                index: Box::new(index.expr),
            }
        }
        kind => {
            let insertion = insert_expr_cleanup_drops(IrExpr { kind, ty, span }, active);
            return insertion;
        }
    };

    ExprCleanupInsertion {
        expr: IrExpr { kind, ty, span },
        moved_roots,
    }
}

fn cleanup_binding_from_stmt(stmt: &IrStmt) -> Option<CleanupBinding> {
    match &stmt.kind {
        IrStmtKind::Let { name, ty, .. } if ty.needs_cleanup() => Some(CleanupBinding {
            name: name.clone(),
            ty: ty.clone(),
            span: stmt.span,
        }),
        _ => None,
    }
}

fn cleanup_assigned_binding_from_stmt(stmt: &IrStmt) -> Option<CleanupBinding> {
    match &stmt.kind {
        IrStmtKind::Assign { name, expr } if expr.ty.needs_cleanup() => Some(CleanupBinding {
            name: name.clone(),
            ty: expr.ty.clone(),
            span: stmt.span,
        }),
        _ => None,
    }
}

fn cleanup_binding_from_for_init(init: &IrForInit) -> Option<CleanupBinding> {
    match init {
        IrForInit::Let { name, ty, expr, .. } if ty.needs_cleanup() => Some(CleanupBinding {
            name: name.clone(),
            ty: ty.clone(),
            span: expr.span,
        }),
        _ => None,
    }
}

fn cleanup_moved_roots_in_for_init(init: &IrForInit) -> HashSet<String> {
    match init {
        IrForInit::Let { expr, .. } => cleanup_moved_roots_in_expr(expr),
    }
}

fn push_loop_cleanup_drops(
    output: &mut Vec<IrStmt>,
    active: &[CleanupBinding],
    pre_loop_roots: &HashSet<String>,
    moved_roots: &HashSet<String>,
    span: Span,
) {
    for binding in active.iter().rev() {
        if pre_loop_roots.contains(&binding.name) || moved_roots.contains(&binding.name) {
            continue;
        }
        push_cleanup_drop(output, binding, span);
    }
}

fn cleanup_reassigned_active_binding<'a>(
    stmt: &IrStmt,
    active: &'a [CleanupBinding],
    moved_roots: &HashSet<String>,
) -> Option<&'a CleanupBinding> {
    let IrStmtKind::Assign { name, expr } = &stmt.kind else {
        return None;
    };
    if !expr.ty.needs_cleanup() || moved_roots.contains(name) {
        return None;
    }
    active.iter().find(|binding| binding.name == *name)
}

fn cleanup_overwritten_place_from_stmt(stmt: &IrStmt) -> Option<IrExpr> {
    match &stmt.kind {
        IrStmtKind::FieldAssign { base, field, expr }
            if expr.ty.needs_cleanup()
                && !field_assignment_consumes_overwritten_place(base, field, expr) =>
        {
            Some(IrExpr {
                kind: IrExprKind::FieldAccess {
                    base: Box::new(base.clone()),
                    field: field.clone(),
                },
                ty: expr.ty.clone(),
                span: stmt.span,
            })
        }
        IrStmtKind::IndexAssign { base, index, expr } if expr.ty.needs_cleanup() => Some(IrExpr {
            kind: IrExprKind::Index {
                base: Box::new(base.clone()),
                index: Box::new(index.clone()),
            },
            ty: expr.ty.clone(),
            span: stmt.span,
        }),
        _ => None,
    }
}

fn field_assignment_consumes_overwritten_place(base: &IrExpr, field: &str, expr: &IrExpr) -> bool {
    let IrExprKind::SliceAppend { slice, .. } = &expr.kind else {
        return false;
    };
    is_same_ir_field_target(base, field, slice)
}

fn is_same_ir_field_target(base: &IrExpr, field: &str, expr: &IrExpr) -> bool {
    let IrExprKind::FieldAccess {
        base: expr_base,
        field: expr_field,
    } = &expr.kind
    else {
        return false;
    };
    expr_field == field && is_same_direct_ir_field_path(base, expr_base)
}

fn is_same_direct_ir_field_path(left: &IrExpr, right: &IrExpr) -> bool {
    match (&left.kind, &right.kind) {
        (IrExprKind::Var(left), IrExprKind::Var(right)) => left == right,
        (
            IrExprKind::FieldAccess {
                base: left_base,
                field: left_field,
            },
            IrExprKind::FieldAccess {
                base: right_base,
                field: right_field,
            },
        ) => left_field == right_field && is_same_direct_ir_field_path(left_base, right_base),
        (
            IrExprKind::Index {
                base: left_base,
                index: left_index,
            },
            IrExprKind::Index {
                base: right_base,
                index: right_index,
            },
        ) => {
            is_same_direct_ir_field_path(left_base, right_base)
                && is_same_ir_expr_ignoring_span(left_index, right_index)
        }
        _ => false,
    }
}

fn is_same_ir_expr_ignoring_span(left: &IrExpr, right: &IrExpr) -> bool {
    match (&left.kind, &right.kind) {
        (IrExprKind::Int(left), IrExprKind::Int(right)) => left == right,
        (IrExprKind::String(left), IrExprKind::String(right)) => left == right,
        (IrExprKind::Bool(left), IrExprKind::Bool(right)) => left == right,
        (IrExprKind::Var(left), IrExprKind::Var(right)) => left == right,
        (
            IrExprKind::Unary {
                op: left_op,
                expr: left_expr,
            },
            IrExprKind::Unary {
                op: right_op,
                expr: right_expr,
            },
        ) => left_op == right_op && is_same_ir_expr_ignoring_span(left_expr, right_expr),
        (
            IrExprKind::Binary {
                op: left_op,
                left: left_left,
                right: left_right,
            },
            IrExprKind::Binary {
                op: right_op,
                left: right_left,
                right: right_right,
            },
        ) => {
            left_op == right_op
                && is_same_ir_expr_ignoring_span(left_left, right_left)
                && is_same_ir_expr_ignoring_span(left_right, right_right)
        }
        (
            IrExprKind::FieldAccess {
                base: left_base,
                field: left_field,
            },
            IrExprKind::FieldAccess {
                base: right_base,
                field: right_field,
            },
        ) => left_field == right_field && is_same_ir_expr_ignoring_span(left_base, right_base),
        (
            IrExprKind::Index {
                base: left_base,
                index: left_index,
            },
            IrExprKind::Index {
                base: right_base,
                index: right_index,
            },
        ) => {
            is_same_ir_expr_ignoring_span(left_base, right_base)
                && is_same_ir_expr_ignoring_span(left_index, right_index)
        }
        _ => false,
    }
}

fn prepare_cleanup_assignment_rhs(stmt: IrStmt) -> (IrStmt, Option<IrStmt>) {
    let span = stmt.span;
    match stmt.kind {
        IrStmtKind::Assign { name, expr } if expr.ty.needs_cleanup() => {
            let (_, temp_stmt, temp_expr) = cleanup_assignment_rhs_temp(expr, span);
            let stmt = IrStmt {
                kind: IrStmtKind::Assign {
                    name,
                    expr: temp_expr,
                },
                span,
            };
            (stmt, Some(temp_stmt))
        }
        IrStmtKind::FieldAssign { base, field, expr } if expr.ty.needs_cleanup() => {
            let (_, temp_stmt, temp_expr) = cleanup_assignment_rhs_temp(expr, span);
            let stmt = IrStmt {
                kind: IrStmtKind::FieldAssign {
                    base,
                    field,
                    expr: temp_expr,
                },
                span,
            };
            (stmt, Some(temp_stmt))
        }
        IrStmtKind::IndexAssign { base, index, expr } if expr.ty.needs_cleanup() => {
            let (_, temp_stmt, temp_expr) = cleanup_assignment_rhs_temp(expr, span);
            let stmt = IrStmt {
                kind: IrStmtKind::IndexAssign {
                    base,
                    index,
                    expr: temp_expr,
                },
                span,
            };
            (stmt, Some(temp_stmt))
        }
        kind => (IrStmt { kind, span }, None),
    }
}

fn cleanup_assignment_rhs_temp(expr: IrExpr, stmt_span: Span) -> (String, IrStmt, IrExpr) {
    let temp_name = cleanup_assignment_rhs_temp_name(stmt_span);
    let temp_ty = expr.ty.clone();
    let temp_expr = IrExpr {
        kind: IrExprKind::Var(temp_name.clone()),
        ty: temp_ty.clone(),
        span: expr.span,
    };
    let temp_stmt = IrStmt {
        kind: IrStmtKind::Let {
            mutable: false,
            name: temp_name.clone(),
            ty: temp_ty,
            expr,
        },
        span: stmt_span,
    };
    (temp_name, temp_stmt, temp_expr)
}

fn cleanup_assignment_rhs_temp_name(span: Span) -> String {
    format!("mallang_cleanup_assign_rhs_{}_{}", span.start, span.end)
}

fn cleanup_moved_roots_in_stmt(stmt: &IrStmt) -> HashSet<String> {
    match &stmt.kind {
        IrStmtKind::Let { expr, .. }
        | IrStmtKind::Assign { expr, .. }
        | IrStmtKind::Return { expr }
        | IrStmtKind::Expr { expr }
        | IrStmtKind::Drop { expr } => cleanup_moved_roots_in_expr(expr),
        IrStmtKind::FieldAssign { expr, .. } | IrStmtKind::IndexAssign { expr, .. } => {
            cleanup_moved_roots_in_expr(expr)
        }
        IrStmtKind::If { .. }
        | IrStmtKind::For { .. }
        | IrStmtKind::RangeFor { .. }
        | IrStmtKind::Break
        | IrStmtKind::Continue
        | IrStmtKind::Match { .. } => HashSet::new(),
    }
}

fn cleanup_moved_roots_in_expr(expr: &IrExpr) -> HashSet<String> {
    let mut roots = HashSet::new();
    collect_cleanup_moved_roots(expr, &mut roots);
    roots
}

fn collect_cleanup_moved_roots(expr: &IrExpr, roots: &mut HashSet<String>) {
    match &expr.kind {
        IrExprKind::Var(name) if expr.ty.needs_cleanup() => {
            roots.insert(name.clone());
        }
        IrExprKind::AdtConstructor { payload, .. } => {
            if let Some(payload) = payload {
                collect_cleanup_moved_roots(payload, roots);
            }
        }
        IrExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_cleanup_moved_roots(&field.expr, roots);
            }
        }
        IrExprKind::ArrayLiteral { elements } => {
            for element in elements {
                collect_cleanup_moved_roots(element, roots);
            }
        }
        IrExprKind::SliceAppend { slice, item } => {
            collect_cleanup_moved_roots(slice, roots);
            collect_cleanup_moved_roots(item, roots);
        }
        IrExprKind::SliceFieldTake { source } => {
            collect_cleanup_moved_roots(source, roots);
        }
        IrExprKind::Call { args, .. } => {
            for arg in args {
                if arg.mode == ArgMode::Owned {
                    collect_cleanup_moved_roots(&arg.expr, roots);
                }
            }
        }
        IrExprKind::If {
            condition,
            then_branch,
            then_cleanup,
            else_branch,
            else_cleanup,
        } => {
            collect_cleanup_moved_roots(condition, roots);
            collect_cleanup_moved_roots(then_branch, roots);
            for stmt in then_cleanup {
                roots.extend(cleanup_moved_roots_in_stmt(stmt));
            }
            collect_cleanup_moved_roots(else_branch, roots);
            for stmt in else_cleanup {
                roots.extend(cleanup_moved_roots_in_stmt(stmt));
            }
        }
        IrExprKind::Match { scrutinee, arms } => {
            collect_cleanup_moved_roots(scrutinee, roots);
            for arm in arms {
                collect_cleanup_moved_roots(&arm.expr, roots);
                for stmt in &arm.cleanup {
                    roots.extend(cleanup_moved_roots_in_stmt(stmt));
                }
            }
        }
        IrExprKind::Unary { expr, .. } => collect_cleanup_moved_roots(expr, roots),
        IrExprKind::Binary { left, right, .. } => {
            collect_cleanup_moved_roots(left, roots);
            collect_cleanup_moved_roots(right, roots);
        }
        IrExprKind::FieldAccess { .. }
        | IrExprKind::Index { .. }
        | IrExprKind::ArrayLen { .. }
        | IrExprKind::Int(_)
        | IrExprKind::String(_)
        | IrExprKind::Bool(_)
        | IrExprKind::Var(_) => {}
    }
}

fn push_cleanup_drops(
    output: &mut Vec<IrStmt>,
    active: &[CleanupBinding],
    excluded_roots: &HashSet<String>,
    span: Span,
) {
    for binding in active.iter().rev() {
        if excluded_roots.contains(&binding.name) {
            continue;
        }
        push_cleanup_drop(output, binding, span);
    }
}

fn push_cleanup_drop(output: &mut Vec<IrStmt>, binding: &CleanupBinding, span: Span) {
    push_cleanup_drop_expr(
        output,
        IrExpr {
            kind: IrExprKind::Var(binding.name.clone()),
            ty: binding.ty.clone(),
            span: binding.span,
        },
        span,
    );
}

fn push_cleanup_drop_expr(output: &mut Vec<IrStmt>, expr: IrExpr, span: Span) {
    output.push(IrStmt {
        kind: IrStmtKind::Drop { expr },
        span,
    });
}

fn arg_mode_for_param(mode: ParamMode) -> ArgMode {
    match mode {
        ParamMode::Owned => ArgMode::Owned,
        ParamMode::Con => ArgMode::Con,
        ParamMode::Mut => ArgMode::Mut,
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

fn is_blank_identifier(name: &str) -> bool {
    name == "_"
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

    fn test_span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn test_slice_ty() -> Type {
        Type::Slice(Box::new(Type::Int))
    }

    fn test_var(name: &str, ty: Type) -> IrExpr {
        IrExpr {
            kind: IrExprKind::Var(name.to_string()),
            ty,
            span: test_span(),
        }
    }

    fn test_bool(value: bool) -> IrExpr {
        IrExpr {
            kind: IrExprKind::Bool(value),
            ty: Type::Bool,
            span: test_span(),
        }
    }

    fn test_owned_call(callee: &str, args: Vec<IrExpr>) -> IrExpr {
        IrExpr {
            kind: IrExprKind::Call {
                callee: callee.to_string(),
                args: args
                    .into_iter()
                    .map(|expr| IrArg {
                        mode: ArgMode::Owned,
                        expr,
                        span: test_span(),
                    })
                    .collect(),
            },
            ty: Type::Unit,
            span: test_span(),
        }
    }

    fn assert_drop_of(stmt: &IrStmt, expected_name: &str) {
        let IrStmtKind::Drop { expr } = &stmt.kind else {
            panic!("expected drop statement");
        };
        let IrExprKind::Var(name) = &expr.kind else {
            panic!("expected drop target variable");
        };
        assert_eq!(name, expected_name);
    }

    fn assert_drop_field(stmt: &IrStmt, expected_base: &str, expected_field: &str) {
        let IrStmtKind::Drop { expr } = &stmt.kind else {
            panic!("expected drop statement");
        };
        let IrExprKind::FieldAccess { base, field } = &expr.kind else {
            panic!("expected drop target field");
        };
        let IrExprKind::Var(name) = &base.kind else {
            panic!("expected field base variable");
        };
        assert_eq!(name, expected_base);
        assert_eq!(field, expected_field);
    }

    fn assert_drop_index(stmt: &IrStmt, expected_base: &str) {
        let IrStmtKind::Drop { expr } = &stmt.kind else {
            panic!("expected drop statement");
        };
        let IrExprKind::Index { base, .. } = &expr.kind else {
            panic!("expected drop target index");
        };
        let IrExprKind::Var(name) = &base.kind else {
            panic!("expected index base variable");
        };
        assert_eq!(name, expected_base);
    }

    fn assert_cleanup_rhs_temp(stmt: &IrStmt, expected_moved_root: &str) -> String {
        let IrStmtKind::Let { name, expr, .. } = &stmt.kind else {
            panic!("expected cleanup rhs temp let");
        };
        assert!(name.starts_with("mallang_cleanup_assign_rhs_"));
        let IrExprKind::Var(root) = &expr.kind else {
            panic!("expected cleanup rhs temp to move a root variable");
        };
        assert_eq!(root, expected_moved_root);
        name.clone()
    }

    fn assert_assign_from_temp(stmt: &IrStmt, expected_name: &str, expected_temp: &str) {
        let IrStmtKind::Assign { name, expr } = &stmt.kind else {
            panic!("expected assignment");
        };
        assert_eq!(name, expected_name);
        let IrExprKind::Var(temp) = &expr.kind else {
            panic!("expected assignment rhs temp");
        };
        assert_eq!(temp, expected_temp);
    }

    fn assert_field_assign_from_temp(stmt: &IrStmt, expected_field: &str, expected_temp: &str) {
        let IrStmtKind::FieldAssign { field, expr, .. } = &stmt.kind else {
            panic!("expected field assignment");
        };
        assert_eq!(field, expected_field);
        let IrExprKind::Var(temp) = &expr.kind else {
            panic!("expected field assignment rhs temp");
        };
        assert_eq!(temp, expected_temp);
    }

    fn assert_index_assign_from_temp(stmt: &IrStmt, expected_temp: &str) {
        let IrStmtKind::IndexAssign { expr, .. } = &stmt.kind else {
            panic!("expected index assignment");
        };
        let IrExprKind::Var(temp) = &expr.kind else {
            panic!("expected index assignment rhs temp");
        };
        assert_eq!(temp, expected_temp);
    }

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
    fn inserts_tail_drop_for_owned_cleanup_param() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty,
        }];

        let body = insert_straight_line_cleanup_drops(Vec::new(), &params, test_span());

        assert_eq!(body.len(), 1);
        assert_drop_of(&body[0], "values");
    }

    #[test]
    fn inserts_drop_before_straight_line_return() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty,
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::Return {
                expr: IrExpr {
                    kind: IrExprKind::Int(1),
                    ty: Type::Int,
                    span: test_span(),
                },
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 2);
        assert_drop_of(&body[0], "values");
        assert!(matches!(body[1].kind, IrStmtKind::Return { .. }));
    }

    #[test]
    fn skips_drop_for_cleanup_root_returned_by_value() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty.clone(),
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::Return {
                expr: test_var("values", slice_ty),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 1);
        assert!(matches!(body[0].kind, IrStmtKind::Return { .. }));
    }

    #[test]
    fn tracks_cleanup_root_moved_into_local() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "seed".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty.clone(),
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::Let {
                mutable: false,
                name: "values".to_string(),
                ty: slice_ty.clone(),
                expr: test_var("seed", slice_ty),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 2);
        assert!(matches!(body[0].kind, IrStmtKind::Let { .. }));
        assert_drop_of(&body[1], "values");
    }

    #[test]
    fn normalizes_cleanup_if_expression_branch_moves() {
        let slice_ty = test_slice_ty();
        let params = vec![
            IrParam {
                name: "values".to_string(),
                mode: ParamMode::Owned,
                ty: slice_ty.clone(),
            },
            IrParam {
                name: "replacement".to_string(),
                mode: ParamMode::Owned,
                ty: slice_ty.clone(),
            },
        ];
        let body = vec![IrStmt {
            kind: IrStmtKind::Let {
                mutable: false,
                name: "result".to_string(),
                ty: slice_ty.clone(),
                expr: IrExpr {
                    kind: IrExprKind::If {
                        condition: Box::new(test_bool(true)),
                        then_branch: Box::new(test_var("values", slice_ty.clone())),
                        then_cleanup: Vec::new(),
                        else_branch: Box::new(test_var("replacement", slice_ty)),
                        else_cleanup: Vec::new(),
                    },
                    ty: test_slice_ty(),
                    span: test_span(),
                },
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 2);
        let IrStmtKind::Let { expr, .. } = &body[0].kind else {
            panic!("expected result let");
        };
        let IrExprKind::If {
            then_cleanup,
            else_cleanup,
            ..
        } = &expr.kind
        else {
            panic!("expected if expression");
        };
        assert_eq!(then_cleanup.len(), 1);
        assert_drop_of(&then_cleanup[0], "replacement");
        assert_eq!(else_cleanup.len(), 1);
        assert_drop_of(&else_cleanup[0], "values");
        assert_drop_of(&body[1], "result");
    }

    #[test]
    fn normalizes_cleanup_match_expression_arm_moves() {
        let slice_ty = test_slice_ty();
        let option_slice_ty = Type::Option(Box::new(slice_ty.clone()));
        let params = vec![
            IrParam {
                name: "source".to_string(),
                mode: ParamMode::Owned,
                ty: option_slice_ty.clone(),
            },
            IrParam {
                name: "fallback".to_string(),
                mode: ParamMode::Owned,
                ty: slice_ty.clone(),
            },
        ];
        let body = vec![IrStmt {
            kind: IrStmtKind::Let {
                mutable: false,
                name: "result".to_string(),
                ty: slice_ty.clone(),
                expr: IrExpr {
                    kind: IrExprKind::Match {
                        scrutinee: Box::new(test_var("source", option_slice_ty)),
                        arms: vec![
                            IrMatchArm {
                                pattern: IrMatchPattern::Some("value".to_string()),
                                expr: test_var("value", slice_ty.clone()),
                                cleanup: Vec::new(),
                                span: test_span(),
                            },
                            IrMatchArm {
                                pattern: IrMatchPattern::None,
                                expr: test_var("fallback", slice_ty),
                                cleanup: Vec::new(),
                                span: test_span(),
                            },
                        ],
                    },
                    ty: test_slice_ty(),
                    span: test_span(),
                },
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 2);
        let IrStmtKind::Let { expr, .. } = &body[0].kind else {
            panic!("expected result let");
        };
        let IrExprKind::Match { arms, .. } = &expr.kind else {
            panic!("expected match expression");
        };
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].cleanup.len(), 1);
        assert_drop_of(&arms[0].cleanup[0], "fallback");
        assert!(arms[1].cleanup.is_empty());
        assert_drop_of(&body[1], "result");
    }

    #[test]
    fn evaluates_cleanup_rhs_before_root_reassignment_drop() {
        let slice_ty = test_slice_ty();
        let params = vec![
            IrParam {
                name: "values".to_string(),
                mode: ParamMode::Owned,
                ty: slice_ty.clone(),
            },
            IrParam {
                name: "replacement".to_string(),
                mode: ParamMode::Owned,
                ty: slice_ty.clone(),
            },
        ];
        let body = vec![IrStmt {
            kind: IrStmtKind::Assign {
                name: "values".to_string(),
                expr: test_var("replacement", slice_ty),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 4);
        let temp = assert_cleanup_rhs_temp(&body[0], "replacement");
        assert_drop_of(&body[1], "values");
        assert_assign_from_temp(&body[2], "values", &temp);
        assert_drop_of(&body[3], "values");
    }

    #[test]
    fn evaluates_cleanup_field_rhs_before_assignment_drop() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::FieldAssign {
                base: test_var("holder", Type::Struct("Holder".to_string())),
                field: "values".to_string(),
                expr: test_var("replacement", slice_ty.clone()),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        assert_eq!(body.len(), 3);
        let temp = assert_cleanup_rhs_temp(&body[0], "replacement");
        assert_drop_field(&body[1], "holder", "values");
        assert_field_assign_from_temp(&body[2], "values", &temp);
    }

    #[test]
    fn evaluates_cleanup_array_element_rhs_before_assignment_drop() {
        let slice_ty = test_slice_ty();
        let array_ty = Type::Array {
            len: 2,
            element: Box::new(slice_ty.clone()),
        };
        let body = vec![IrStmt {
            kind: IrStmtKind::IndexAssign {
                base: test_var("values", array_ty),
                index: IrExpr {
                    kind: IrExprKind::Int(0),
                    ty: Type::Int,
                    span: test_span(),
                },
                expr: test_var("replacement", slice_ty),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        assert_eq!(body.len(), 3);
        let temp = assert_cleanup_rhs_temp(&body[0], "replacement");
        assert_drop_index(&body[1], "values");
        assert_index_assign_from_temp(&body[2], &temp);
    }

    #[test]
    fn evaluates_cleanup_slice_element_rhs_before_assignment_drop() {
        let slice_ty = test_slice_ty();
        let outer_slice_ty = Type::Slice(Box::new(slice_ty.clone()));
        let body = vec![IrStmt {
            kind: IrStmtKind::IndexAssign {
                base: test_var("values", outer_slice_ty),
                index: IrExpr {
                    kind: IrExprKind::Int(0),
                    ty: Type::Int,
                    span: test_span(),
                },
                expr: test_var("replacement", slice_ty),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        assert_eq!(body.len(), 3);
        let temp = assert_cleanup_rhs_temp(&body[0], "replacement");
        assert_drop_index(&body[1], "values");
        assert_index_assign_from_temp(&body[2], &temp);
    }

    #[test]
    fn inserts_cleanup_drops_for_if_branch_local_roots() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::If {
                condition: test_bool(true),
                then_body: vec![IrStmt {
                    kind: IrStmtKind::Let {
                        mutable: false,
                        name: "left".to_string(),
                        ty: slice_ty.clone(),
                        expr: test_var("seed_left", slice_ty.clone()),
                    },
                    span: test_span(),
                }],
                else_body: vec![IrStmt {
                    kind: IrStmtKind::Let {
                        mutable: false,
                        name: "right".to_string(),
                        ty: slice_ty.clone(),
                        expr: test_var("seed_right", slice_ty),
                    },
                    span: test_span(),
                }],
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::If {
            then_body,
            else_body,
            ..
        } = &body[0].kind
        else {
            panic!("expected if statement");
        };
        assert_eq!(then_body.len(), 2);
        assert_eq!(else_body.len(), 2);
        assert_drop_of(&then_body[1], "left");
        assert_drop_of(&else_body[1], "right");
    }

    #[test]
    fn inserts_cleanup_drops_for_match_arm_local_roots() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::Match {
                scrutinee: IrExpr {
                    kind: IrExprKind::Var("maybe".to_string()),
                    ty: Type::Option(Box::new(Type::Int)),
                    span: test_span(),
                },
                arms: vec![
                    IrMatchBlockArm {
                        pattern: IrMatchPattern::Some("value".to_string()),
                        body: vec![IrStmt {
                            kind: IrStmtKind::Let {
                                mutable: false,
                                name: "some_values".to_string(),
                                ty: slice_ty.clone(),
                                expr: test_var("seed_some", slice_ty.clone()),
                            },
                            span: test_span(),
                        }],
                        span: test_span(),
                    },
                    IrMatchBlockArm {
                        pattern: IrMatchPattern::None,
                        body: vec![IrStmt {
                            kind: IrStmtKind::Let {
                                mutable: false,
                                name: "none_values".to_string(),
                                ty: slice_ty.clone(),
                                expr: test_var("seed_none", slice_ty),
                            },
                            span: test_span(),
                        }],
                        span: test_span(),
                    },
                ],
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::Match { arms, .. } = &body[0].kind else {
            panic!("expected match statement");
        };
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].body.len(), 2);
        assert_eq!(arms[1].body.len(), 2);
        assert_drop_of(&arms[0].body[1], "some_values");
        assert_drop_of(&arms[1].body[1], "none_values");
    }

    #[test]
    fn inserts_merge_drop_for_if_outer_cleanup_root_moved_in_one_branch() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty.clone(),
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::If {
                condition: test_bool(true),
                then_body: vec![IrStmt {
                    kind: IrStmtKind::Expr {
                        expr: test_owned_call("consume", vec![test_var("values", slice_ty)]),
                    },
                    span: test_span(),
                }],
                else_body: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 1);
        let IrStmtKind::If {
            then_body,
            else_body,
            ..
        } = &body[0].kind
        else {
            panic!("expected if statement");
        };
        assert_eq!(then_body.len(), 1);
        assert_eq!(else_body.len(), 1);
        assert_drop_of(&else_body[0], "values");
    }

    #[test]
    fn inserts_merge_drop_for_match_outer_cleanup_root_moved_in_one_arm() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty.clone(),
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::Match {
                scrutinee: IrExpr {
                    kind: IrExprKind::Var("maybe".to_string()),
                    ty: Type::Option(Box::new(Type::Int)),
                    span: test_span(),
                },
                arms: vec![
                    IrMatchBlockArm {
                        pattern: IrMatchPattern::Some("value".to_string()),
                        body: vec![IrStmt {
                            kind: IrStmtKind::Expr {
                                expr: test_owned_call(
                                    "consume",
                                    vec![test_var("values", slice_ty)],
                                ),
                            },
                            span: test_span(),
                        }],
                        span: test_span(),
                    },
                    IrMatchBlockArm {
                        pattern: IrMatchPattern::None,
                        body: Vec::new(),
                        span: test_span(),
                    },
                ],
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 1);
        let IrStmtKind::Match { arms, .. } = &body[0].kind else {
            panic!("expected match statement");
        };
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].body.len(), 1);
        assert_eq!(arms[1].body.len(), 1);
        assert_drop_of(&arms[1].body[0], "values");
    }

    #[test]
    fn inserts_outer_cleanup_drop_before_branch_local_return() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty,
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::If {
                condition: test_bool(true),
                then_body: vec![IrStmt {
                    kind: IrStmtKind::Return {
                        expr: IrExpr {
                            kind: IrExprKind::Int(1),
                            ty: Type::Int,
                            span: test_span(),
                        },
                    },
                    span: test_span(),
                }],
                else_body: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 2);
        let IrStmtKind::If {
            then_body,
            else_body,
            ..
        } = &body[0].kind
        else {
            panic!("expected if statement");
        };
        assert_eq!(then_body.len(), 2);
        assert_drop_of(&then_body[0], "values");
        assert!(matches!(then_body[1].kind, IrStmtKind::Return { .. }));
        assert!(else_body.is_empty());
        assert_drop_of(&body[1], "values");
    }

    #[test]
    fn inserts_merge_drop_when_branch_returns_outer_cleanup_root() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty.clone(),
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::If {
                condition: test_bool(true),
                then_body: vec![IrStmt {
                    kind: IrStmtKind::Return {
                        expr: test_var("values", slice_ty),
                    },
                    span: test_span(),
                }],
                else_body: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 1);
        let IrStmtKind::If {
            then_body,
            else_body,
            ..
        } = &body[0].kind
        else {
            panic!("expected if statement");
        };
        assert_eq!(then_body.len(), 1);
        assert!(matches!(then_body[0].kind, IrStmtKind::Return { .. }));
        assert_eq!(else_body.len(), 1);
        assert_drop_of(&else_body[0], "values");
    }

    #[test]
    fn inserts_loop_body_local_cleanup_drop_at_tail() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: None,
                condition: None,
                post: None,
                body: vec![IrStmt {
                    kind: IrStmtKind::Let {
                        mutable: false,
                        name: "values".to_string(),
                        ty: slice_ty.clone(),
                        expr: test_var("seed", slice_ty),
                    },
                    span: test_span(),
                }],
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::For { body, .. } = &body[0].kind else {
            panic!("expected for statement");
        };
        assert_eq!(body.len(), 2);
        assert!(matches!(body[0].kind, IrStmtKind::Let { .. }));
        assert_drop_of(&body[1], "values");
    }

    #[test]
    fn inserts_loop_body_local_cleanup_drop_before_continue() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: None,
                condition: None,
                post: None,
                body: vec![
                    IrStmt {
                        kind: IrStmtKind::Let {
                            mutable: false,
                            name: "values".to_string(),
                            ty: slice_ty.clone(),
                            expr: test_var("seed", slice_ty),
                        },
                        span: test_span(),
                    },
                    IrStmt {
                        kind: IrStmtKind::Continue,
                        span: test_span(),
                    },
                ],
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::For { body, .. } = &body[0].kind else {
            panic!("expected for statement");
        };
        assert_eq!(body.len(), 3);
        assert!(matches!(body[0].kind, IrStmtKind::Let { .. }));
        assert_drop_of(&body[1], "values");
        assert!(matches!(body[2].kind, IrStmtKind::Continue));
    }

    #[test]
    fn inserts_loop_body_local_cleanup_drop_before_break() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: None,
                condition: None,
                post: None,
                body: vec![
                    IrStmt {
                        kind: IrStmtKind::Let {
                            mutable: false,
                            name: "values".to_string(),
                            ty: slice_ty.clone(),
                            expr: test_var("seed", slice_ty),
                        },
                        span: test_span(),
                    },
                    IrStmt {
                        kind: IrStmtKind::Break,
                        span: test_span(),
                    },
                ],
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::For { body, .. } = &body[0].kind else {
            panic!("expected for statement");
        };
        assert_eq!(body.len(), 3);
        assert!(matches!(body[0].kind, IrStmtKind::Let { .. }));
        assert_drop_of(&body[1], "values");
        assert!(matches!(body[2].kind, IrStmtKind::Break));
    }

    #[test]
    fn inserts_outer_cleanup_drop_before_return_inside_loop() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty,
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: None,
                condition: None,
                post: None,
                body: vec![IrStmt {
                    kind: IrStmtKind::Return {
                        expr: IrExpr {
                            kind: IrExprKind::Int(1),
                            ty: Type::Int,
                            span: test_span(),
                        },
                    },
                    span: test_span(),
                }],
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 2);
        let IrStmtKind::For {
            body: loop_body, ..
        } = &body[0].kind
        else {
            panic!("expected for statement");
        };
        assert_eq!(loop_body.len(), 2);
        assert_drop_of(&loop_body[0], "values");
        assert!(matches!(loop_body[1].kind, IrStmtKind::Return { .. }));
        assert_drop_of(&body[1], "values");
    }

    #[test]
    fn does_not_drop_outer_cleanup_root_before_loop_continue() {
        let slice_ty = test_slice_ty();
        let params = vec![IrParam {
            name: "values".to_string(),
            mode: ParamMode::Owned,
            ty: slice_ty,
        }];
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: None,
                condition: None,
                post: None,
                body: vec![IrStmt {
                    kind: IrStmtKind::Continue,
                    span: test_span(),
                }],
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &params, test_span());

        assert_eq!(body.len(), 2);
        let IrStmtKind::For {
            body: loop_body, ..
        } = &body[0].kind
        else {
            panic!("expected for statement");
        };
        assert_eq!(loop_body.len(), 1);
        assert!(matches!(loop_body[0].kind, IrStmtKind::Continue));
        assert_drop_of(&body[1], "values");
    }

    #[test]
    fn inserts_range_loop_body_local_cleanup_drop_at_tail() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::RangeFor {
                index_name: "i".to_string(),
                value_name: "_".to_string(),
                source: IrExpr {
                    kind: IrExprKind::Var("items".to_string()),
                    ty: Type::Array {
                        len: 2,
                        element: Box::new(Type::Int),
                    },
                    span: test_span(),
                },
                element_ty: Type::Int,
                body: vec![IrStmt {
                    kind: IrStmtKind::Let {
                        mutable: false,
                        name: "values".to_string(),
                        ty: slice_ty.clone(),
                        expr: test_var("seed", slice_ty),
                    },
                    span: test_span(),
                }],
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::RangeFor { body, .. } = &body[0].kind else {
            panic!("expected range loop");
        };
        assert_eq!(body.len(), 2);
        assert!(matches!(body[0].kind, IrStmtKind::Let { .. }));
        assert_drop_of(&body[1], "values");
    }

    #[test]
    fn inserts_for_init_cleanup_after_loop() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: Some(Box::new(IrForInit::Let {
                    mutable: false,
                    name: "loop_values".to_string(),
                    ty: slice_ty.clone(),
                    expr: test_var("seed", slice_ty),
                })),
                condition: None,
                post: None,
                body: Vec::new(),
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::For { body, cleanup, .. } = &body[0].kind else {
            panic!("expected for statement");
        };
        assert!(body.is_empty());
        assert_eq!(cleanup.len(), 1);
        assert_drop_of(&cleanup[0], "loop_values");
    }

    #[test]
    fn preserves_for_init_cleanup_across_continue() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: Some(Box::new(IrForInit::Let {
                    mutable: false,
                    name: "loop_values".to_string(),
                    ty: slice_ty.clone(),
                    expr: test_var("seed", slice_ty),
                })),
                condition: None,
                post: None,
                body: vec![IrStmt {
                    kind: IrStmtKind::Continue,
                    span: test_span(),
                }],
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::For { body, cleanup, .. } = &body[0].kind else {
            panic!("expected for statement");
        };
        assert_eq!(body.len(), 1);
        assert!(matches!(body[0].kind, IrStmtKind::Continue));
        assert_eq!(cleanup.len(), 1);
        assert_drop_of(&cleanup[0], "loop_values");
    }

    #[test]
    fn inserts_for_init_cleanup_before_return_inside_loop() {
        let slice_ty = test_slice_ty();
        let body = vec![IrStmt {
            kind: IrStmtKind::For {
                init: Some(Box::new(IrForInit::Let {
                    mutable: false,
                    name: "loop_values".to_string(),
                    ty: slice_ty.clone(),
                    expr: test_var("seed", slice_ty),
                })),
                condition: None,
                post: None,
                body: vec![IrStmt {
                    kind: IrStmtKind::Return {
                        expr: IrExpr {
                            kind: IrExprKind::Int(1),
                            ty: Type::Int,
                            span: test_span(),
                        },
                    },
                    span: test_span(),
                }],
                cleanup: Vec::new(),
            },
            span: test_span(),
        }];

        let body = insert_straight_line_cleanup_drops(body, &[], test_span());

        let IrStmtKind::For { body, cleanup, .. } = &body[0].kind else {
            panic!("expected for statement");
        };
        assert_eq!(body.len(), 2);
        assert_drop_of(&body[0], "loop_values");
        assert!(matches!(body[1].kind, IrStmtKind::Return { .. }));
        assert_eq!(cleanup.len(), 1);
        assert_drop_of(&cleanup[0], "loop_values");
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
    fn ir_lowers_bool_unary_not() {
        let program = parse(
            r#"
func main() {
    flag := !false
    print(flag)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let { ty, expr, .. } = &ir.functions[0].body[0].kind else {
            panic!("expected typed let");
        };
        assert_eq!(*ty, Type::Bool);
        assert!(matches!(
            expr.kind,
            IrExprKind::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));
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
            ..
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
            ..
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
            ..
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
            ..
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
            ..
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
    fn ir_lowers_array_element_method_receivers() {
        let program = parse(
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
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let main = ir
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function");
        let IrStmtKind::Expr { expr } = &main.body[1].kind else {
            panic!("expected method call expression");
        };
        let IrExprKind::Call { callee, args } = &expr.kind else {
            panic!("expected method call");
        };
        assert_eq!(callee, "Counter.inc");
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].mode, ArgMode::Mut);
        assert_eq!(args[0].expr.ty, Type::Struct("Counter".to_string()));
        assert!(matches!(args[0].expr.kind, IrExprKind::Index { .. }));
    }

    #[test]
    fn ir_lowers_slice_element_borrow_arguments() {
        let program = parse(
            r#"
type User struct {
    name string
    age int
}

func show(con user User) {
    print(user.age)
}

func main() {
    users := []User{User{name: "kim", age: 30}}
    show(con users[0])
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let main = ir
            .functions
            .iter()
            .find(|function| function.name == "main")
            .expect("main function");
        let IrStmtKind::Expr { expr } = &main.body[1].kind else {
            panic!("expected call expression");
        };
        let IrExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected call");
        };
        assert_eq!(args[0].mode, ArgMode::Con);
        assert_eq!(args[0].expr.ty, Type::Struct("User".to_string()));
        let IrExprKind::Index { base, index } = &args[0].expr.kind else {
            panic!("expected slice element borrow expression");
        };
        assert!(matches!(base.ty, Type::Slice(_)));
        assert_eq!(index.ty, Type::Int);
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
    fn ir_lowers_indexed_field_assignment() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    mut users := []User{User{age: 20}}
    users[0].age = 21
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::FieldAssign { base, field, expr } = &ir.functions[0].body[1].kind else {
            panic!("expected field assignment");
        };
        let IrExprKind::Index { base: root, index } = &base.kind else {
            panic!("expected indexed field assignment base");
        };
        assert!(matches!(root.ty, Type::Slice(_)));
        assert_eq!(index.ty, Type::Int);
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
    fn ir_lowers_slice_range_loops() {
        let program = parse(
            r#"
func main() {
    values := []int{1, 2}
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
        assert!(matches!(source.ty, Type::Slice(_)));
        assert_eq!(*element_ty, Type::Int);
        assert_eq!(body.len(), 2);

        assert_drop_of(&ir.functions[0].body[2], "values");
    }

    #[test]
    fn ir_lowers_array_range_blank_identifiers() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    values := [2]int{1, 2}
    for _, value := range values {
        print(value)
    }

    users := [1]User{User{age: 1}}
    for i, _ := range users {
        print(i)
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
            element_ty,
            ..
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "_");
        assert_eq!(value_name, "value");
        assert_eq!(*element_ty, Type::Int);

        let IrStmtKind::RangeFor {
            index_name,
            value_name,
            element_ty,
            ..
        } = &ir.functions[0].body[3].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "i");
        assert_eq!(value_name, "_");
        assert_eq!(*element_ty, Type::Struct("User".to_string()));
    }

    #[test]
    fn ir_lowers_one_variable_array_range() {
        let program = parse(
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
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::RangeFor {
            index_name,
            value_name,
            element_ty,
            ..
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "i");
        assert_eq!(value_name, "_");
        assert_eq!(*element_ty, Type::Struct("User".to_string()));

        let IrStmtKind::RangeFor {
            index_name,
            value_name,
            element_ty,
            ..
        } = &ir.functions[0].body[2].kind
        else {
            panic!("expected range loop");
        };
        assert_eq!(index_name, "_");
        assert_eq!(value_name, "_");
        assert_eq!(*element_ty, Type::Struct("User".to_string()));
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
    fn ir_lowers_slice_literal_indexing_and_len() {
        let program = parse(
            r#"
func main() {
    values := []int{1, 2, 3}
    first := values[1]
    count := len(values)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let { expr, ty, .. } = &ir.functions[0].body[0].kind else {
            panic!("expected slice let statement");
        };
        assert_eq!(*ty, Type::Slice(Box::new(Type::Int)));
        let IrExprKind::ArrayLiteral { elements } = &expr.kind else {
            panic!("expected slice literal expression");
        };
        assert_eq!(elements.len(), 3);

        let IrStmtKind::Let { expr, ty, .. } = &ir.functions[0].body[1].kind else {
            panic!("expected index let statement");
        };
        assert_eq!(*ty, Type::Int);
        let IrExprKind::Index { base, index } = &expr.kind else {
            panic!("expected index expression");
        };
        assert!(matches!(base.ty, Type::Slice(_)));
        assert_eq!(index.ty, Type::Int);

        let IrStmtKind::Let { expr, ty, .. } = &ir.functions[0].body[2].kind else {
            panic!("expected len let statement");
        };
        assert_eq!(*ty, Type::Int);
        let IrExprKind::ArrayLen { array } = &expr.kind else {
            panic!("expected slice len expression");
        };
        assert!(matches!(array.ty, Type::Slice(_)));

        assert_drop_of(&ir.functions[0].body[3], "values");
    }

    #[test]
    fn ir_lowers_slice_append_and_reactivates_assignment_cleanup() {
        let program = parse(
            r#"
func main() {
    mut values := []int{1}
    values = append(values, 2)
    total := values[1] + len(values)
    print(total)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Assign { name, expr } = &ir.functions[0].body[1].kind else {
            panic!("expected slice assignment");
        };
        assert_eq!(name, "values");
        assert_eq!(expr.ty, Type::Slice(Box::new(Type::Int)));
        let IrExprKind::SliceAppend { slice, item } = &expr.kind else {
            panic!("expected slice append expression");
        };
        assert!(matches!(slice.ty, Type::Slice(_)));
        assert_eq!(item.ty, Type::Int);

        assert_drop_of(&ir.functions[0].body[4], "values");
    }

    #[test]
    fn ir_lowers_slice_field_append_without_overwrite_drop() {
        let program = parse(
            r#"
type Bag struct {
    values []int
}

func main() {
    mut bag := Bag{values: []int{1}}
    bag.values = append(bag.values, 2)
    print(len(bag.values))
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        assert_eq!(ir.functions[0].body.len(), 4);
        let IrStmtKind::FieldAssign { base, field, expr } = &ir.functions[0].body[1].kind else {
            panic!("expected field assignment");
        };
        assert!(matches!(base.kind, IrExprKind::Var(ref name) if name == "bag"));
        assert_eq!(field, "values");
        let IrExprKind::SliceAppend { slice, item } = &expr.kind else {
            panic!("expected slice append expression");
        };
        let IrExprKind::FieldAccess { base, field } = &slice.kind else {
            panic!("expected field append source");
        };
        assert!(matches!(base.kind, IrExprKind::Var(ref name) if name == "bag"));
        assert_eq!(field, "values");
        assert_eq!(item.ty, Type::Int);
        assert_drop_of(&ir.functions[0].body[3], "bag");
    }

    #[test]
    fn ir_lowers_slice_field_append_take_source() {
        let program = parse(
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
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let { name, expr, .. } = &ir.functions[0].body[1].kind else {
            panic!("expected grown let");
        };
        assert_eq!(name, "grown");
        let IrExprKind::SliceAppend { slice, item } = &expr.kind else {
            panic!("expected slice append expression");
        };
        assert!(matches!(slice.kind, IrExprKind::FieldAccess { .. }));
        assert_eq!(item.ty, Type::Int);
        assert_drop_of(&ir.functions[0].body[4], "grown");
        assert_drop_of(&ir.functions[0].body[5], "bag");
    }

    #[test]
    fn ir_lowers_owned_slice_field_take_expression() {
        let program = parse(
            r#"
type Bag struct {
    values []int
}

func main() {
    bag := Bag{values: []int{1, 2}}
    taken := bag.values
    print(len(bag.values))
    consume(bag.values)
}

func consume(values []int) {
    print(len(values))
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let { name, expr, .. } = &ir.functions[0].body[1].kind else {
            panic!("expected taken let");
        };
        assert_eq!(name, "taken");
        let IrExprKind::SliceFieldTake { source } = &expr.kind else {
            panic!("expected slice field take expression");
        };
        assert!(matches!(source.kind, IrExprKind::FieldAccess { .. }));

        let IrStmtKind::Expr { expr } = &ir.functions[0].body[2].kind else {
            panic!("expected print expression");
        };
        let IrExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected print call");
        };
        let IrExprKind::ArrayLen { array } = &args[0].expr.kind else {
            panic!("expected len argument");
        };
        assert!(matches!(array.kind, IrExprKind::FieldAccess { .. }));

        let IrStmtKind::Expr { expr } = &ir.functions[0].body[3].kind else {
            panic!("expected consume expression");
        };
        let IrExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected consume call");
        };
        assert!(matches!(
            args[0].expr.kind,
            IrExprKind::SliceFieldTake { .. }
        ));
        assert_drop_of(&ir.functions[0].body[4], "taken");
        assert_drop_of(&ir.functions[0].body[5], "bag");
    }

    #[test]
    fn ir_lowers_indexed_slice_field_append_without_overwrite_drop() {
        let program = parse(
            r#"
type Bag struct {
    values []int
}

type Store struct {
    bags []Bag
}

func main() {
    mut store := Store{bags: []Bag{Bag{values: []int{1}}, Bag{values: []int{2}}}}
    i := 1
    store.bags[i].values = append(store.bags[i].values, 3)
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        assert_eq!(ir.functions[0].body.len(), 4);
        let IrStmtKind::FieldAssign { base, field, expr } = &ir.functions[0].body[2].kind else {
            panic!("expected indexed field assignment");
        };
        assert_eq!(field, "values");
        let IrExprKind::Index { base: bags, index } = &base.kind else {
            panic!("expected indexed assignment base");
        };
        assert_eq!(index.ty, Type::Int);
        let IrExprKind::FieldAccess { base: store, field } = &bags.kind else {
            panic!("expected store.bags base");
        };
        assert!(matches!(store.kind, IrExprKind::Var(ref name) if name == "store"));
        assert_eq!(field, "bags");

        let IrExprKind::SliceAppend { slice, item } = &expr.kind else {
            panic!("expected slice append expression");
        };
        assert_eq!(item.ty, Type::Int);
        let IrExprKind::FieldAccess {
            base: append_base,
            field,
        } = &slice.kind
        else {
            panic!("expected append field source");
        };
        assert_eq!(field, "values");
        assert!(matches!(append_base.kind, IrExprKind::Index { .. }));
        assert_drop_of(&ir.functions[0].body[3], "store");
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
    fn ir_lowers_non_copy_array_element_assignment() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    mut users := [2]User{User{age: 1}, User{age: 2}}
    users[1] = User{age: 3}
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let {
            name: temp_name,
            ty,
            expr,
            ..
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected cleanup assignment temp");
        };
        assert!(temp_name.starts_with("mallang_cleanup_assign_rhs_"));
        assert_eq!(ty, &Type::Struct("User".to_string()));
        assert_eq!(expr.ty, Type::Struct("User".to_string()));

        let IrStmtKind::Drop { expr } = &ir.functions[0].body[2].kind else {
            panic!("expected indexed cleanup drop");
        };
        assert_eq!(expr.ty, Type::Struct("User".to_string()));
        assert!(matches!(expr.kind, IrExprKind::Index { .. }));

        let IrStmtKind::IndexAssign { base, index, expr } = &ir.functions[0].body[3].kind else {
            panic!("expected index assignment");
        };
        assert!(matches!(base.ty, Type::Array { .. }));
        assert_eq!(index.ty, Type::Int);
        assert_eq!(expr.ty, Type::Struct("User".to_string()));
        assert!(matches!(&expr.kind, IrExprKind::Var(name) if name == temp_name));
    }

    #[test]
    fn ir_lowers_slice_element_assignment() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    mut users := []User{User{age: 1}, User{age: 2}}
    users[1] = User{age: 3}
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::Let {
            name: temp_name,
            ty,
            expr,
            ..
        } = &ir.functions[0].body[1].kind
        else {
            panic!("expected cleanup assignment temp");
        };
        assert!(temp_name.starts_with("mallang_cleanup_assign_rhs_"));
        assert_eq!(ty, &Type::Struct("User".to_string()));
        assert_eq!(expr.ty, Type::Struct("User".to_string()));

        let IrStmtKind::Drop { expr } = &ir.functions[0].body[2].kind else {
            panic!("expected indexed cleanup drop");
        };
        assert_eq!(expr.ty, Type::Struct("User".to_string()));
        assert!(matches!(expr.kind, IrExprKind::Index { .. }));

        let IrStmtKind::IndexAssign { base, index, expr } = &ir.functions[0].body[3].kind else {
            panic!("expected index assignment");
        };
        assert!(matches!(base.ty, Type::Slice(_)));
        assert_eq!(index.ty, Type::Int);
        assert_eq!(expr.ty, Type::Struct("User".to_string()));
        assert!(matches!(&expr.kind, IrExprKind::Var(name) if name == temp_name));
    }

    #[test]
    fn ir_lowers_local_rooted_slice_element_assignment() {
        let program = parse(
            r#"
type Bag struct {
    values []int
}

func main() {
    mut bag := Bag{values: []int{1, 2}}
    bag.values[1] = 5
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::IndexAssign { base, index, expr } = &ir.functions[0].body[1].kind else {
            panic!("expected index assignment");
        };
        let IrExprKind::FieldAccess { base: root, field } = &base.kind else {
            panic!("expected field-rooted slice assignment target");
        };
        assert!(matches!(&root.kind, IrExprKind::Var(name) if name == "bag"));
        assert_eq!(field, "values");
        assert!(matches!(base.ty, Type::Slice(_)));
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

    #[test]
    fn ir_lowers_non_copy_array_element_assignment_in_for_post() {
        let program = parse(
            r#"
type User struct {
    age int
}

func main() {
    mut users := [2]User{User{age: 1}, User{age: 2}}
    mut i := 0
    for ; i < 1; users[i] = makeUser() {
        i = i + 1
    }
}

func makeUser() User {
    return User{age: 3}
}
"#,
        )
        .unwrap();
        let checked = check(&program).unwrap();
        let ir = lower(&checked).unwrap();

        let IrStmtKind::For { post, .. } = &ir.functions[0].body[2].kind else {
            panic!("expected for statement");
        };
        let Some(IrForPost::Assign { target, expr }) = post.as_deref() else {
            panic!("expected for post assignment");
        };
        let IrExprKind::Index { index, .. } = &target.kind else {
            panic!("expected index assignment target");
        };
        assert_eq!(index.ty, Type::Int);
        assert_eq!(target.ty, Type::Struct("User".to_string()));
        assert_eq!(expr.ty, Type::Struct("User".to_string()));
    }
}
