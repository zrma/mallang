use crate::token::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub structs: Vec<StructDecl>,
    pub functions: Vec<Function>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub receiver: Option<Param>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub mode: ParamMode,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    Owned,
    In,
    Mut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub name: String,
    pub args: Vec<TypeRef>,
    pub array_len: Option<usize>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: String,
        expr: Expr,
    },
    Assign {
        name: String,
        expr: Expr,
    },
    FieldAssign {
        base: Expr,
        field: String,
        expr: Expr,
    },
    Return {
        expr: Expr,
    },
    If {
        condition: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },
    For {
        init: Option<ForInit>,
        condition: Option<Expr>,
        post: Option<ForPost>,
        body: Block,
    },
    Break,
    Continue,
    Match {
        scrutinee: Expr,
        arms: Vec<MatchBlockArm>,
    },
    Expr {
        expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForInit {
    Let {
        mutable: bool,
        name: String,
        expr: Expr,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForPost {
    Assign { target: Expr, expr: Expr },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Int(i64),
    String(String),
    Bool(bool),
    Nil,
    Var(String),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    StructLiteral {
        type_name: String,
        fields: Vec<FieldInit>,
    },
    ArrayLiteral {
        ty: Box<TypeRef>,
        elements: Vec<Expr>,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInit {
    pub name: String,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchBlockArm {
    pub pattern: MatchPattern,
    pub block: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchPattern {
    Some(String),
    None,
    Ok(String),
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    pub mode: ArgMode,
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgMode {
    Owned,
    In,
    Mut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    LogicalAnd,
    LogicalOr,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
