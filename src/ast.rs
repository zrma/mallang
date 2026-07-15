use crate::{
    standard::{StandardIntrinsic, StandardType},
    token::Span,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub source_units: Vec<SourceUnit>,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<Function>,
    pub tests: Vec<TestDecl>,
    pub source_spans: Vec<Span>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnit {
    pub package: Option<PackageDecl>,
    pub imports: Vec<ImportDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDecl {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Package,
    Public,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructDecl {
    pub visibility: Visibility,
    pub name: String,
    pub intrinsic: Option<StandardType>,
    pub intrinsic_args: Vec<TypeRef>,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDecl {
    pub visibility: Visibility,
    pub name: String,
    pub intrinsic: Option<StandardType>,
    pub specialization_origin: Option<String>,
    pub type_params: Vec<TypeParam>,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub payloads: Vec<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    pub name: String,
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
    pub visibility: Visibility,
    pub name: String,
    pub intrinsic: Option<StandardIntrinsic>,
    pub type_params: Vec<TypeParam>,
    pub receiver: Option<Param>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestDecl {
    pub name: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamMode {
    Owned,
    Con,
    Mut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub name: String,
    pub args: Vec<TypeRef>,
    pub array_len: Option<usize>,
    pub slice: bool,
    pub function: Option<FunctionTypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionTypeRef {
    pub mutable: bool,
    pub params: Vec<FunctionTypeParam>,
    pub return_type: Box<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionTypeParam {
    pub mode: ParamMode,
    pub ty: TypeRef,
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
    IndexAssign {
        base: Expr,
        index: Expr,
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
    RangeFor {
        index_name: String,
        value_name: String,
        source: Expr,
        body: Block,
    },
    Break,
    Continue,
    Match {
        scrutinee: Expr,
        arms: Vec<MatchBlockArm>,
    },
    Assert {
        condition: Expr,
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
pub struct FunctionLiteral {
    pub mutable: bool,
    pub params: Vec<Param>,
    pub return_type: Option<TypeRef>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprKind {
    Int(i64),
    String(String),
    Bool(bool),
    Nil,
    Var(String),
    FunctionLiteral(Box<FunctionLiteral>),
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
        type_args: Vec<TypeRef>,
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
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    TypeApply {
        base: Box<Expr>,
        args: Vec<TypeRef>,
    },
    EnumConstructor {
        enum_name: String,
        variant: String,
        args: Option<Vec<Arg>>,
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
    Wildcard,
    Binding(String),
    Variant {
        type_name: String,
        variant: String,
        payloads: Vec<MatchPattern>,
    },
    NestedBuiltin {
        variant: String,
        payload: Box<MatchPattern>,
    },
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
    Con,
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
