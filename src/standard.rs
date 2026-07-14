use std::collections::BTreeMap;

use crate::{
    ast::{
        Block, EnumDecl, EnumVariant, FieldDecl, Function, FunctionTypeParam, FunctionTypeRef,
        Param, ParamMode, Program, StructDecl, TypeParam, TypeRef, Visibility,
    },
    linker::internal_symbol,
    package::{Package, PackageDeclaration, PackageDeclarationKind, PackageGraph},
    token::Span,
};

pub const STANDARD_PREFIX: &str = "std/";

pub(crate) fn is_error_kind_type_name(name: &str) -> bool {
    name == internal_symbol("std/errors", "Kind")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardType {
    ErrorKind,
    Error,
    Map,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StandardIntrinsic {
    FsReadText,
    FsWriteText,
    IoReadStdin,
    IoWriteStdout,
    IoWriteStderr,
    OsArgs,
    OsEnv,
    OsExit,
    StringsByteLen,
    StringsScalarCount,
    StringsContains,
    StringsFind,
    StringsSplit,
    StringsJoin,
    StringsFromInt,
    StringsParseInt,
    StringsFromBool,
    StringsParseBool,
    CollectionsNewMap,
    CollectionsCount,
    CollectionsInsert,
    CollectionsWith,
    CollectionsUpdate,
    CollectionsRemove,
}

pub const STANDARD_INTRINSICS: &[StandardIntrinsic] = &[
    StandardIntrinsic::FsReadText,
    StandardIntrinsic::FsWriteText,
    StandardIntrinsic::IoReadStdin,
    StandardIntrinsic::IoWriteStdout,
    StandardIntrinsic::IoWriteStderr,
    StandardIntrinsic::OsArgs,
    StandardIntrinsic::OsEnv,
    StandardIntrinsic::OsExit,
    StandardIntrinsic::StringsByteLen,
    StandardIntrinsic::StringsScalarCount,
    StandardIntrinsic::StringsContains,
    StandardIntrinsic::StringsFind,
    StandardIntrinsic::StringsSplit,
    StandardIntrinsic::StringsJoin,
    StandardIntrinsic::StringsFromInt,
    StandardIntrinsic::StringsParseInt,
    StandardIntrinsic::StringsFromBool,
    StandardIntrinsic::StringsParseBool,
    StandardIntrinsic::CollectionsNewMap,
    StandardIntrinsic::CollectionsCount,
    StandardIntrinsic::CollectionsInsert,
    StandardIntrinsic::CollectionsWith,
    StandardIntrinsic::CollectionsUpdate,
    StandardIntrinsic::CollectionsRemove,
];

impl StandardIntrinsic {
    pub const fn package_path(self) -> &'static str {
        match self {
            Self::FsReadText | Self::FsWriteText => "std/fs",
            Self::IoReadStdin | Self::IoWriteStdout | Self::IoWriteStderr => "std/io",
            Self::OsArgs | Self::OsEnv | Self::OsExit => "std/os",
            Self::StringsByteLen
            | Self::StringsScalarCount
            | Self::StringsContains
            | Self::StringsFind
            | Self::StringsSplit
            | Self::StringsJoin
            | Self::StringsFromInt
            | Self::StringsParseInt
            | Self::StringsFromBool
            | Self::StringsParseBool => "std/strings",
            Self::CollectionsNewMap
            | Self::CollectionsCount
            | Self::CollectionsInsert
            | Self::CollectionsWith
            | Self::CollectionsUpdate
            | Self::CollectionsRemove => "std/collections",
        }
    }

    pub const fn function_name(self) -> &'static str {
        match self {
            Self::FsReadText => "readText",
            Self::FsWriteText => "writeText",
            Self::IoReadStdin => "readStdin",
            Self::IoWriteStdout => "writeStdout",
            Self::IoWriteStderr => "writeStderr",
            Self::OsArgs => "args",
            Self::OsEnv => "env",
            Self::OsExit => "exit",
            Self::StringsByteLen => "byteLen",
            Self::StringsScalarCount => "scalarCount",
            Self::StringsContains => "contains",
            Self::StringsFind => "find",
            Self::StringsSplit => "split",
            Self::StringsJoin => "join",
            Self::StringsFromInt => "fromInt",
            Self::StringsParseInt => "parseInt",
            Self::StringsFromBool => "fromBool",
            Self::StringsParseBool => "parseBool",
            Self::CollectionsNewMap => "newMap",
            Self::CollectionsCount => "count",
            Self::CollectionsInsert => "insert",
            Self::CollectionsWith => "with",
            Self::CollectionsUpdate => "update",
            Self::CollectionsRemove => "remove",
        }
    }

    pub fn source_name(self) -> String {
        format!("{}.{}", self.package_path(), self.function_name())
    }

    pub(crate) fn internal_name(self) -> String {
        internal_symbol(self.package_path(), self.function_name())
    }
}

pub fn package(path: &str, span: Span) -> Option<Package> {
    let declarations = match path {
        "std/errors" => vec![
            declaration("Kind", PackageDeclarationKind::Enum, &[], span),
            declaration("Error", PackageDeclarationKind::Struct, &[], span),
        ],
        "std/fs" => function_declarations(&["readText", "writeText"], &[], span),
        "std/io" => function_declarations(&["readStdin", "writeStdout", "writeStderr"], &[], span),
        "std/os" => function_declarations(&["args", "env", "exit"], &[], span),
        "std/strings" => function_declarations(
            &[
                "byteLen",
                "scalarCount",
                "contains",
                "find",
                "split",
                "join",
                "fromInt",
                "parseInt",
                "fromBool",
                "parseBool",
            ],
            &[],
            span,
        ),
        "std/collections" => {
            let mut declarations = vec![declaration(
                "Map",
                PackageDeclarationKind::Opaque,
                &["K", "V"],
                span,
            )];
            declarations.extend(function_declarations(
                &["newMap", "count", "insert", "with", "update", "remove"],
                &["K", "V"],
                span,
            ));
            declarations
        }
        _ => return None,
    };

    let name = path
        .rsplit_once('/')
        .map_or(path, |(_, qualifier)| qualifier)
        .to_string();
    Some(Package {
        path: path.to_string(),
        name,
        source_ids: Vec::new(),
        imports: Vec::new(),
        declarations: declarations
            .into_iter()
            .map(|declaration| (declaration.name.clone(), declaration))
            .collect(),
        methods: BTreeMap::new(),
    })
}

pub fn augment_program(program: &mut Program, graph: &PackageGraph) {
    let span = program.span;
    let has_package = |path: &str| graph.package(path).is_some();
    let needs_errors = ["std/errors", "std/fs", "std/io", "std/os", "std/strings"]
        .iter()
        .any(|path| has_package(path));

    if needs_errors {
        add_error_types(program, span);
    }
    if has_package("std/fs") {
        add_fs_functions(program, span);
    }
    if has_package("std/io") {
        add_io_functions(program, span);
    }
    if has_package("std/os") {
        add_os_functions(program, span);
    }
    if has_package("std/strings") {
        add_string_functions(program, span);
    }
    if has_package("std/collections") {
        add_collection_declarations(program, span);
    }
}

fn declaration(
    name: &str,
    kind: PackageDeclarationKind,
    type_params: &[&str],
    span: Span,
) -> PackageDeclaration {
    PackageDeclaration {
        name: name.to_string(),
        kind,
        type_params: type_params
            .iter()
            .map(|param| (*param).to_string())
            .collect(),
        visibility: Visibility::Public,
        span,
    }
}

fn function_declarations(
    names: &[&str],
    type_params: &[&str],
    span: Span,
) -> Vec<PackageDeclaration> {
    names
        .iter()
        .map(|name| declaration(name, PackageDeclarationKind::Function, type_params, span))
        .collect()
}

fn add_error_types(program: &mut Program, span: Span) {
    let kind_name = internal_symbol("std/errors", "Kind");
    program.enums.push(EnumDecl {
        visibility: Visibility::Public,
        name: kind_name.clone(),
        intrinsic: Some(StandardType::ErrorKind),
        specialization_origin: None,
        type_params: Vec::new(),
        variants: [
            "NotFound",
            "PermissionDenied",
            "AlreadyExists",
            "InvalidInput",
            "InvalidData",
            "Interrupted",
            "Other",
        ]
        .into_iter()
        .map(|name| EnumVariant {
            name: name.to_string(),
            payloads: Vec::new(),
            span,
        })
        .collect(),
        span,
    });
    program.structs.push(StructDecl {
        visibility: Visibility::Public,
        name: internal_symbol("std/errors", "Error"),
        intrinsic: Some(StandardType::Error),
        intrinsic_args: Vec::new(),
        type_params: Vec::new(),
        fields: vec![
            FieldDecl {
                name: "kind".to_string(),
                ty: named_type(kind_name, span),
                span,
            },
            FieldDecl {
                name: "message".to_string(),
                ty: named_type("string", span),
                span,
            },
        ],
        span,
    });
}

fn add_fs_functions(program: &mut Program, span: Span) {
    add_function(
        program,
        StandardIntrinsic::FsReadText,
        &[],
        vec![param(
            "path",
            ParamMode::Con,
            named_type("string", span),
            span,
        )],
        Some(result_type(
            named_type("string", span),
            error_type(span),
            span,
        )),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::FsWriteText,
        &[],
        vec![
            param("path", ParamMode::Con, named_type("string", span), span),
            param("text", ParamMode::Con, named_type("string", span), span),
        ],
        Some(result_type(
            named_type("unit", span),
            error_type(span),
            span,
        )),
        span,
    );
}

fn add_io_functions(program: &mut Program, span: Span) {
    add_function(
        program,
        StandardIntrinsic::IoReadStdin,
        &[],
        Vec::new(),
        Some(result_type(
            named_type("string", span),
            error_type(span),
            span,
        )),
        span,
    );
    for intrinsic in [
        StandardIntrinsic::IoWriteStdout,
        StandardIntrinsic::IoWriteStderr,
    ] {
        add_function(
            program,
            intrinsic,
            &[],
            vec![param(
                "text",
                ParamMode::Con,
                named_type("string", span),
                span,
            )],
            Some(result_type(
                named_type("unit", span),
                error_type(span),
                span,
            )),
            span,
        );
    }
}

fn add_os_functions(program: &mut Program, span: Span) {
    add_function(
        program,
        StandardIntrinsic::OsArgs,
        &[],
        Vec::new(),
        Some(result_type(
            slice_type(named_type("string", span), span),
            error_type(span),
            span,
        )),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::OsEnv,
        &[],
        vec![param(
            "name",
            ParamMode::Con,
            named_type("string", span),
            span,
        )],
        Some(result_type(
            option_type(named_type("string", span), span),
            error_type(span),
            span,
        )),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::OsExit,
        &[],
        vec![param(
            "code",
            ParamMode::Owned,
            named_type("int", span),
            span,
        )],
        None,
        span,
    );
}

fn add_string_functions(program: &mut Program, span: Span) {
    for intrinsic in [
        StandardIntrinsic::StringsByteLen,
        StandardIntrinsic::StringsScalarCount,
    ] {
        add_function(
            program,
            intrinsic,
            &[],
            vec![param(
                "text",
                ParamMode::Con,
                named_type("string", span),
                span,
            )],
            Some(named_type("int", span)),
            span,
        );
    }
    add_function(
        program,
        StandardIntrinsic::StringsContains,
        &[],
        text_pair_params(span),
        Some(named_type("bool", span)),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::StringsFind,
        &[],
        text_pair_params(span),
        Some(option_type(named_type("int", span), span)),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::StringsSplit,
        &[],
        vec![
            param("text", ParamMode::Con, named_type("string", span), span),
            param(
                "separator",
                ParamMode::Con,
                named_type("string", span),
                span,
            ),
        ],
        Some(slice_type(named_type("string", span), span)),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::StringsJoin,
        &[],
        vec![
            param(
                "parts",
                ParamMode::Con,
                slice_type(named_type("string", span), span),
                span,
            ),
            param(
                "separator",
                ParamMode::Con,
                named_type("string", span),
                span,
            ),
        ],
        Some(named_type("string", span)),
        span,
    );
    for (intrinsic, input, output) in [
        (
            StandardIntrinsic::StringsFromInt,
            named_type("int", span),
            named_type("string", span),
        ),
        (
            StandardIntrinsic::StringsFromBool,
            named_type("bool", span),
            named_type("string", span),
        ),
    ] {
        add_function(
            program,
            intrinsic,
            &[],
            vec![param("value", ParamMode::Owned, input, span)],
            Some(output),
            span,
        );
    }
    for (intrinsic, output) in [
        (StandardIntrinsic::StringsParseInt, named_type("int", span)),
        (
            StandardIntrinsic::StringsParseBool,
            named_type("bool", span),
        ),
    ] {
        add_function(
            program,
            intrinsic,
            &[],
            vec![param(
                "text",
                ParamMode::Con,
                named_type("string", span),
                span,
            )],
            Some(result_type(output, error_type(span), span)),
            span,
        );
    }
}

fn add_collection_declarations(program: &mut Program, span: Span) {
    let type_params = ["K", "V"];
    program.structs.push(StructDecl {
        visibility: Visibility::Public,
        name: internal_symbol("std/collections", "Map"),
        intrinsic: Some(StandardType::Map),
        intrinsic_args: vec![named_type("K", span), named_type("V", span)],
        type_params: type_params
            .iter()
            .map(|name| TypeParam {
                name: (*name).to_string(),
                span,
            })
            .collect(),
        fields: Vec::new(),
        span,
    });

    let key = || named_type("K", span);
    let value = || named_type("V", span);
    let map = || map_type(key(), value(), span);
    add_function(
        program,
        StandardIntrinsic::CollectionsNewMap,
        &type_params,
        Vec::new(),
        Some(map()),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::CollectionsCount,
        &type_params,
        vec![param("map", ParamMode::Con, map(), span)],
        Some(named_type("int", span)),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::CollectionsInsert,
        &type_params,
        vec![
            param("map", ParamMode::Mut, map(), span),
            param("key", ParamMode::Owned, key(), span),
            param("value", ParamMode::Owned, value(), span),
        ],
        Some(option_type(value(), span)),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::CollectionsWith,
        &type_params,
        vec![
            param("map", ParamMode::Con, map(), span),
            param("key", ParamMode::Con, key(), span),
            param(
                "visit",
                ParamMode::Con,
                function_type(ParamMode::Con, value(), span),
                span,
            ),
        ],
        Some(named_type("bool", span)),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::CollectionsUpdate,
        &type_params,
        vec![
            param("map", ParamMode::Mut, map(), span),
            param("key", ParamMode::Con, key(), span),
            param(
                "edit",
                ParamMode::Con,
                function_type(ParamMode::Mut, value(), span),
                span,
            ),
        ],
        Some(named_type("bool", span)),
        span,
    );
    add_function(
        program,
        StandardIntrinsic::CollectionsRemove,
        &type_params,
        vec![
            param("map", ParamMode::Mut, map(), span),
            param("key", ParamMode::Con, key(), span),
        ],
        Some(option_type(value(), span)),
        span,
    );
}

fn add_function(
    program: &mut Program,
    intrinsic: StandardIntrinsic,
    type_params: &[&str],
    params: Vec<Param>,
    return_type: Option<TypeRef>,
    span: Span,
) {
    program.functions.push(Function {
        visibility: Visibility::Public,
        name: intrinsic.internal_name(),
        intrinsic: Some(intrinsic),
        type_params: type_params
            .iter()
            .map(|name| TypeParam {
                name: (*name).to_string(),
                span,
            })
            .collect(),
        receiver: None,
        params,
        return_type,
        body: Block {
            statements: Vec::new(),
            span,
        },
        span,
    });
}

fn text_pair_params(span: Span) -> Vec<Param> {
    vec![
        param("text", ParamMode::Con, named_type("string", span), span),
        param("needle", ParamMode::Con, named_type("string", span), span),
    ]
}

fn param(name: &str, mode: ParamMode, ty: TypeRef, span: Span) -> Param {
    Param {
        name: name.to_string(),
        mode,
        ty,
        span,
    }
}

fn named_type(name: impl Into<String>, span: Span) -> TypeRef {
    TypeRef {
        name: name.into(),
        args: Vec::new(),
        array_len: None,
        slice: false,
        function: None,
        span,
    }
}

fn applied_type(name: &str, args: Vec<TypeRef>, span: Span) -> TypeRef {
    TypeRef {
        name: name.to_string(),
        args,
        array_len: None,
        slice: false,
        function: None,
        span,
    }
}

fn slice_type(element: TypeRef, span: Span) -> TypeRef {
    TypeRef {
        name: "Slice".to_string(),
        args: vec![element],
        array_len: None,
        slice: true,
        function: None,
        span,
    }
}

fn option_type(inner: TypeRef, span: Span) -> TypeRef {
    applied_type("Option", vec![inner], span)
}

fn result_type(ok: TypeRef, error: TypeRef, span: Span) -> TypeRef {
    applied_type("Result", vec![ok, error], span)
}

fn error_type(span: Span) -> TypeRef {
    named_type(internal_symbol("std/errors", "Error"), span)
}

fn map_type(key: TypeRef, value: TypeRef, span: Span) -> TypeRef {
    applied_type(
        &internal_symbol("std/collections", "Map"),
        vec![key, value],
        span,
    )
}

fn function_type(mode: ParamMode, ty: TypeRef, span: Span) -> TypeRef {
    TypeRef {
        name: "func".to_string(),
        args: Vec::new(),
        array_len: None,
        slice: false,
        function: Some(FunctionTypeRef {
            mutable: false,
            params: vec![FunctionTypeParam { mode, ty, span }],
            return_type: Box::new(named_type("unit", span)),
            span,
        }),
        span,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn registry_covers_every_intrinsic_exactly_once() {
        let span = Span::default();
        let mut names = BTreeSet::new();

        for intrinsic in STANDARD_INTRINSICS {
            let package = package(intrinsic.package_path(), span).unwrap();
            let declaration = &package.declarations[intrinsic.function_name()];

            assert_eq!(declaration.kind, PackageDeclarationKind::Function);
            assert_eq!(declaration.visibility, Visibility::Public);
            assert!(names.insert((intrinsic.package_path(), intrinsic.function_name())));
        }

        assert_eq!(names.len(), 24);
    }

    #[test]
    fn map_registry_type_is_public_generic_and_opaque() {
        let package = package("std/collections", Span::default()).unwrap();
        let map = &package.declarations["Map"];

        assert_eq!(map.kind, PackageDeclarationKind::Opaque);
        assert_eq!(map.visibility, Visibility::Public);
        assert_eq!(map.type_params, ["K", "V"]);
    }
}
