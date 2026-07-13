#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(usize);

impl SourceId {
    pub const fn new(index: usize) -> Self {
        Self(index)
    }

    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Span {
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(source: SourceId, start: usize, end: usize) -> Self {
        Self { source, start, end }
    }

    pub fn join(self, other: Self) -> Self {
        assert_eq!(
            self.source, other.source,
            "cannot join spans from different source files"
        );
        Self {
            source: self.source,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Ident(String),
    Int(String),
    String(String),
    Keyword(Keyword),
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    Bang,
    BangEqual,
    AmpAmp,
    PipePipe,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    ColonEqual,
    Arrow,
    PipeGreater,
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    Package,
    Import,
    Pub,
    Func,
    Return,
    If,
    Else,
    For,
    Break,
    Continue,
    Range,
    Match,
    Case,
    Mut,
    Con,
    True,
    False,
    Struct,
    Type,
    Nil,
}

impl Keyword {
    pub fn from_ident(ident: &str) -> Option<Self> {
        match ident {
            "package" => Some(Self::Package),
            "import" => Some(Self::Import),
            "pub" => Some(Self::Pub),
            "func" => Some(Self::Func),
            "return" => Some(Self::Return),
            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "for" => Some(Self::For),
            "break" => Some(Self::Break),
            "continue" => Some(Self::Continue),
            "range" => Some(Self::Range),
            "match" => Some(Self::Match),
            "case" => Some(Self::Case),
            "mut" => Some(Self::Mut),
            "con" => Some(Self::Con),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "struct" => Some(Self::Struct),
            "type" => Some(Self::Type),
            "nil" => Some(Self::Nil),
            _ => None,
        }
    }
}
