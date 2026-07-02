use std::{collections::HashMap, fmt};

use crate::{
    ast::{Arg, ArgMode, BinaryOp, Expr, ExprKind, ParamMode, Stmt, StmtKind, UnaryOp},
    semantic::{CheckedProgram, FunctionSig, ParamSig, Type},
    token::Span,
};

pub fn lower(checked: &CheckedProgram<'_>) -> Result<IrProgram, IrError> {
    Lowerer::new(checked).lower_program()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrProgram {
    pub functions: Vec<IrFunction>,
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
    Return {
        expr: IrExpr,
    },
    Expr {
        expr: IrExpr,
    },
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
        let mut functions = Vec::new();
        for function in &self.checked.program.functions {
            let sig = self.function_sig(&function.name, function.span)?;
            let mut locals = HashMap::new();
            for param in &sig.params {
                locals.insert(param.name.clone(), param.ty);
            }
            let mut body = Vec::new();
            for stmt in &function.body.statements {
                body.push(self.lower_stmt(stmt, &mut locals)?);
            }
            functions.push(IrFunction {
                name: function.name.clone(),
                params: sig.params.iter().map(lower_param).collect(),
                return_type: sig.return_type,
                body,
            });
        }

        Ok(IrProgram { functions })
    }

    fn lower_stmt(
        &self,
        stmt: &Stmt,
        locals: &mut HashMap<String, Type>,
    ) -> Result<IrStmt, IrError> {
        let kind = match &stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                expr,
            } => {
                let expr = self.lower_expr(expr, locals)?;
                locals.insert(name.clone(), expr.ty);
                IrStmtKind::Let {
                    mutable: *mutable,
                    name: name.clone(),
                    ty: expr.ty,
                    expr,
                }
            }
            StmtKind::Assign { name, expr } => {
                let expr = self.lower_expr(expr, locals)?;
                IrStmtKind::Assign {
                    name: name.clone(),
                    expr,
                }
            }
            StmtKind::Return { expr } => IrStmtKind::Return {
                expr: self.lower_expr(expr, locals)?,
            },
            StmtKind::Expr { expr } => IrStmtKind::Expr {
                expr: self.lower_expr(expr, locals)?,
            },
        };

        Ok(IrStmt {
            kind,
            span: stmt.span,
        })
    }

    fn lower_expr(&self, expr: &Expr, locals: &HashMap<String, Type>) -> Result<IrExpr, IrError> {
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
                let Some(ty) = locals.get(name).copied() else {
                    return Err(IrError::new(
                        format!("unknown variable `{name}` during IR lowering"),
                        expr.span,
                    ));
                };
                (IrExprKind::Var(name.clone()), ty)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.lower_expr(condition, locals)?;
                if condition.ty != Type::Bool {
                    return Err(IrError::new(
                        "semantic analysis accepted a non-bool if condition",
                        condition.span,
                    ));
                }
                let then_branch = self.lower_expr(then_branch, locals)?;
                let else_branch = self.lower_expr(else_branch, locals)?;
                if then_branch.ty != else_branch.ty {
                    return Err(IrError::new(
                        "semantic analysis accepted mismatched if branch types",
                        expr.span,
                    ));
                }
                let ty = then_branch.ty;
                (
                    IrExprKind::If {
                        condition: Box::new(condition),
                        then_branch: Box::new(then_branch),
                        else_branch: Box::new(else_branch),
                    },
                    ty,
                )
            }
            ExprKind::Call { callee, args } => self.lower_call(callee, args, locals, expr.span)?,
            ExprKind::Unary { op, expr } => {
                let expr = self.lower_expr(expr, locals)?;
                let ty = match (op, expr.ty) {
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

    fn lower_call(
        &self,
        callee: &Expr,
        args: &[Arg],
        locals: &HashMap<String, Type>,
        span: Span,
    ) -> Result<(IrExprKind, Type), IrError> {
        let ExprKind::Var(name) = &callee.kind else {
            return Err(IrError::new(
                "only direct calls should reach IR lowering",
                callee.span,
            ));
        };

        let mut lowered_args = Vec::new();
        for arg in args {
            lowered_args.push(IrArg {
                mode: arg.mode,
                expr: self.lower_expr(&arg.expr, locals)?,
                span: arg.span,
            });
        }

        if name == "print" {
            return Ok((
                IrExprKind::Call {
                    callee: name.clone(),
                    args: lowered_args,
                },
                Type::Unit,
            ));
        }

        let sig = self.function_sig(name, span)?;
        Ok((
            IrExprKind::Call {
                callee: name.clone(),
                args: lowered_args,
            },
            sig.return_type,
        ))
    }

    fn function_sig(&self, name: &str, span: Span) -> Result<&FunctionSig, IrError> {
        self.checked
            .signatures
            .get(name)
            .ok_or_else(|| IrError::new(format!("unknown function `{name}`"), span))
    }
}

fn lower_param(param: &ParamSig) -> IrParam {
    IrParam {
        name: param.name.clone(),
        mode: param.mode,
        ty: param.ty,
    }
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

        let IrStmtKind::Let { ty, .. } = ir.functions[0].body[0].kind else {
            panic!("expected typed let");
        };
        assert_eq!(ty, Type::Int);
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
}
