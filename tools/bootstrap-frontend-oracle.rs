use std::env;
use std::fs;
use std::process::ExitCode;

use mallang::ast::{
    Block, EnumDecl, FieldDecl, Function, FunctionTypeParam, ImportDecl, PackageDecl, Param,
    Program, SourceUnit, StructDecl, TestDecl, TypeParam, TypeRef, Visibility,
};
use mallang::{lex, parse_with_diagnostics, Keyword, LexError, Span, Token, TokenKind};

fn main() -> ExitCode {
    let mut args = env::args();
    let _program = args.next();
    let Some(first) = args.next() else {
        eprintln!("usage: bootstrap-frontend-oracle [parse] <source>");
        return ExitCode::from(2);
    };
    let (mode, path) = if first == "parse" {
        let Some(path) = args.next() else {
            eprintln!("usage: bootstrap-frontend-oracle [parse] <source>");
            return ExitCode::from(2);
        };
        ("parse", path)
    } else {
        ("lex", first)
    };
    if args.next().is_some() {
        eprintln!("usage: bootstrap-frontend-oracle [parse] <source>");
        return ExitCode::from(2);
    }

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => {
            eprintln!("bootstrap frontend oracle could not read source");
            return ExitCode::from(2);
        }
    };

    if mode == "parse" {
        match parse_with_diagnostics(&source) {
            Ok(program) => println!("{}", normalize_program(&program).normalize(0)),
            Err(errors) => {
                for error in errors {
                    println!(
                        "PERR|{}|{}|{}|{}",
                        error.span.source.index(),
                        error.span.start,
                        error.span.end,
                        encode_bytes(&error.message)
                    );
                }
            }
        }
    } else {
        match lex(&source) {
            Ok(tokens) => {
                for token in tokens {
                    println!("{}", normalize_token(&token));
                }
            }
            Err(error) => println!("{}", normalize_error(&error)),
        }
    }

    ExitCode::SUCCESS
}

struct NormalizedNode {
    kind: String,
    value: String,
    span: Span,
    children: Vec<NormalizedNode>,
}

impl NormalizedNode {
    fn new(
        kind: impl Into<String>,
        value: impl Into<String>,
        span: Span,
        children: Vec<Self>,
    ) -> Self {
        Self {
            kind: kind.into(),
            value: value.into(),
            span,
            children,
        }
    }

    fn normalize(&self, depth: usize) -> String {
        let mut lines = vec![format!(
            "N|{depth}|{}|{}|{}|{}|{}|{}",
            self.kind,
            self.span.source.index(),
            self.span.start,
            self.span.end,
            encode_bytes(&self.value),
            self.children.len()
        )];
        lines.extend(self.children.iter().map(|child| child.normalize(depth + 1)));
        lines.join("\n")
    }
}

fn normalize_program(program: &Program) -> NormalizedNode {
    let mut children = program
        .source_units
        .iter()
        .map(normalize_source_unit)
        .collect::<Vec<_>>();
    children.extend(program.structs.iter().map(normalize_struct));
    children.extend(program.enums.iter().map(normalize_enum));
    children.extend(program.functions.iter().map(normalize_function));
    children.extend(program.tests.iter().map(normalize_test));
    NormalizedNode::new("Program", "", program.span, children)
}

fn normalize_source_unit(unit: &SourceUnit) -> NormalizedNode {
    let mut children = unit
        .package
        .iter()
        .map(normalize_package)
        .collect::<Vec<_>>();
    children.extend(unit.imports.iter().map(normalize_import));
    NormalizedNode::new("SourceUnit", "", unit.span, children)
}

fn normalize_package(package: &PackageDecl) -> NormalizedNode {
    NormalizedNode::new("PackageDecl", &package.name, package.span, Vec::new())
}

fn normalize_import(import: &ImportDecl) -> NormalizedNode {
    NormalizedNode::new("ImportDecl", &import.path, import.span, Vec::new())
}

fn normalize_struct(declaration: &StructDecl) -> NormalizedNode {
    let mut children = declaration
        .type_params
        .iter()
        .map(normalize_type_param)
        .collect::<Vec<_>>();
    children.extend(declaration.fields.iter().map(normalize_field));
    NormalizedNode::new(
        format!("StructDecl.{}", visibility_name(declaration.visibility)),
        &declaration.name,
        declaration.span,
        children,
    )
}

fn normalize_enum(declaration: &EnumDecl) -> NormalizedNode {
    let mut children = declaration
        .type_params
        .iter()
        .map(normalize_type_param)
        .collect::<Vec<_>>();
    children.extend(declaration.variants.iter().map(|variant| {
        NormalizedNode::new(
            "EnumVariant",
            &variant.name,
            variant.span,
            variant.payloads.iter().map(normalize_type_ref).collect(),
        )
    }));
    NormalizedNode::new(
        format!("EnumDecl.{}", visibility_name(declaration.visibility)),
        &declaration.name,
        declaration.span,
        children,
    )
}

fn normalize_type_param(param: &TypeParam) -> NormalizedNode {
    NormalizedNode::new("TypeParam", &param.name, param.span, Vec::new())
}

fn normalize_field(field: &FieldDecl) -> NormalizedNode {
    NormalizedNode::new(
        "FieldDecl",
        &field.name,
        field.span,
        vec![normalize_type_ref(&field.ty)],
    )
}

fn normalize_function(function: &Function) -> NormalizedNode {
    let mut children = function
        .type_params
        .iter()
        .map(normalize_type_param)
        .collect::<Vec<_>>();
    if let Some(receiver) = &function.receiver {
        children.push(NormalizedNode::new(
            "Receiver",
            "",
            receiver.span,
            vec![normalize_param(receiver, "ReceiverParam")],
        ));
    }
    children.extend(
        function
            .params
            .iter()
            .map(|param| normalize_param(param, "Param")),
    );
    if let Some(return_type) = &function.return_type {
        children.push(NormalizedNode::new(
            "ReturnType",
            "",
            return_type.span,
            vec![normalize_type_ref(return_type)],
        ));
    }
    children.push(normalize_block(&function.body));
    NormalizedNode::new(
        format!("FunctionDecl.{}", visibility_name(function.visibility)),
        &function.name,
        function.span,
        children,
    )
}

fn normalize_test(test: &TestDecl) -> NormalizedNode {
    NormalizedNode::new(
        "TestDecl",
        &test.name,
        test.span,
        vec![normalize_block(&test.body)],
    )
}

fn normalize_param(param: &Param, prefix: &str) -> NormalizedNode {
    NormalizedNode::new(
        format!("{prefix}.{:?}", param.mode),
        &param.name,
        param.span,
        vec![normalize_type_ref(&param.ty)],
    )
}

fn normalize_type_ref(ty: &TypeRef) -> NormalizedNode {
    if let Some(function) = &ty.function {
        let mut children = function
            .params
            .iter()
            .map(normalize_function_type_param)
            .collect::<Vec<_>>();
        children.push(NormalizedNode::new(
            "FunctionReturnType",
            "",
            function.return_type.span,
            vec![normalize_type_ref(&function.return_type)],
        ));
        return NormalizedNode::new(
            if function.mutable {
                "Type.Function.Mutable"
            } else {
                "Type.Function.Immutable"
            },
            "",
            ty.span,
            children,
        );
    }
    if ty.slice {
        return NormalizedNode::new(
            "Type.Slice",
            "",
            ty.span,
            ty.args.iter().map(normalize_type_ref).collect(),
        );
    }
    if let Some(length) = ty.array_len {
        return NormalizedNode::new(
            "Type.Array",
            length.to_string(),
            ty.span,
            ty.args.iter().map(normalize_type_ref).collect(),
        );
    }
    NormalizedNode::new(
        "Type.Named",
        &ty.name,
        ty.span,
        ty.args.iter().map(normalize_type_ref).collect(),
    )
}

fn normalize_function_type_param(param: &FunctionTypeParam) -> NormalizedNode {
    NormalizedNode::new(
        format!("FunctionTypeParam.{:?}", param.mode),
        "",
        param.span,
        vec![normalize_type_ref(&param.ty)],
    )
}

fn normalize_block(block: &Block) -> NormalizedNode {
    NormalizedNode::new("Block", "", block.span, Vec::new())
}

fn visibility_name(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Package => "Package",
        Visibility::Public => "Public",
    }
}

fn normalize_token(token: &Token) -> String {
    let (kind, value) = match &token.kind {
        TokenKind::Ident(value) => ("Ident", value.as_str()),
        TokenKind::Int(value) => ("Int", value.as_str()),
        TokenKind::String(value) => ("String", value.as_str()),
        TokenKind::Keyword(keyword) => (keyword_kind(*keyword), ""),
        TokenKind::LeftParen => ("LeftParen", ""),
        TokenKind::RightParen => ("RightParen", ""),
        TokenKind::LeftBrace => ("LeftBrace", ""),
        TokenKind::RightBrace => ("RightBrace", ""),
        TokenKind::LeftBracket => ("LeftBracket", ""),
        TokenKind::RightBracket => ("RightBracket", ""),
        TokenKind::Comma => ("Comma", ""),
        TokenKind::Dot => ("Dot", ""),
        TokenKind::Colon => ("Colon", ""),
        TokenKind::Semicolon => ("Semicolon", ""),
        TokenKind::Plus => ("Plus", ""),
        TokenKind::Minus => ("Minus", ""),
        TokenKind::Star => ("Star", ""),
        TokenKind::Slash => ("Slash", ""),
        TokenKind::Percent => ("Percent", ""),
        TokenKind::Equal => ("Equal", ""),
        TokenKind::EqualEqual => ("EqualEqual", ""),
        TokenKind::Bang => ("Bang", ""),
        TokenKind::BangEqual => ("BangEqual", ""),
        TokenKind::AmpAmp => ("AmpAmp", ""),
        TokenKind::PipePipe => ("PipePipe", ""),
        TokenKind::Less => ("Less", ""),
        TokenKind::LessEqual => ("LessEqual", ""),
        TokenKind::Greater => ("Greater", ""),
        TokenKind::GreaterEqual => ("GreaterEqual", ""),
        TokenKind::ColonEqual => ("ColonEqual", ""),
        TokenKind::Arrow => ("Arrow", ""),
        TokenKind::PipeGreater => ("PipeGreater", ""),
        TokenKind::Eof => ("Eof", ""),
    };

    format!(
        "T|{kind}|{}|{}|{}",
        token.span.start,
        token.span.end,
        encode_bytes(value)
    )
}

fn normalize_error(error: &LexError) -> String {
    format!(
        "E|{}|{}|{}",
        error.span.start,
        error.span.end,
        encode_bytes(&error.message)
    )
}

fn encode_bytes(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn keyword_kind(keyword: Keyword) -> &'static str {
    match keyword {
        Keyword::Package => "Keyword.Package",
        Keyword::Import => "Keyword.Import",
        Keyword::Pub => "Keyword.Pub",
        Keyword::Func => "Keyword.Func",
        Keyword::Return => "Keyword.Return",
        Keyword::If => "Keyword.If",
        Keyword::Else => "Keyword.Else",
        Keyword::For => "Keyword.For",
        Keyword::Break => "Keyword.Break",
        Keyword::Continue => "Keyword.Continue",
        Keyword::Range => "Keyword.Range",
        Keyword::Match => "Keyword.Match",
        Keyword::Case => "Keyword.Case",
        Keyword::Mut => "Keyword.Mut",
        Keyword::Con => "Keyword.Con",
        Keyword::True => "Keyword.True",
        Keyword::False => "Keyword.False",
        Keyword::Struct => "Keyword.Struct",
        Keyword::Enum => "Keyword.Enum",
        Keyword::Type => "Keyword.Type",
        Keyword::Nil => "Keyword.Nil",
    }
}
