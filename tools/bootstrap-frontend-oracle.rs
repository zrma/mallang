use std::env;
use std::fs;
use std::process::ExitCode;

use mallang::ast::{
    Arg, ArgMode, BinaryOp, Block, EnumDecl, Expr, ExprKind, FieldDecl, FieldInit, ForInit,
    ForPost, Function, FunctionTypeParam, ImportDecl, MatchPattern, PackageDecl, Param, ParamMode,
    Program, SourceUnit, Stmt, StmtKind, StructDecl, TestDecl, TypeParam, TypeRef, UnaryOp,
    Visibility,
};
use mallang::ir::{
    IrArg, IrClosureCaptureValue, IrEnumStorage, IrExpr, IrExprKind, IrFieldValue, IrForInit,
    IrForPost, IrMatchArm, IrMatchBlockArm, IrMatchPattern, IrStmt, IrStmtKind,
};
use mallang::standard::augment_program;
use mallang::{
    build_package_graph, check, check_project, discover_project, lex, link_project, lower,
    lower_test, parse_sources_with_diagnostics, parse_with_diagnostics, specialize, CheckedProgram,
    IrProgram, Keyword, LexError, PackageDeclarationKind, SourceMap, Span, Token, TokenKind,
};

fn main() -> ExitCode {
    let mut args = env::args();
    let _program = args.next();
    let Some(first) = args.next() else {
        eprintln!("usage: bootstrap-frontend-oracle [parse|check|ir|ir-test] <source>");
        return ExitCode::from(2);
    };
    if first == "manifest" {
        let Some(path) = args.next() else {
            eprintln!("usage: bootstrap-frontend-oracle manifest <mallang.toml>");
            return ExitCode::from(2);
        };
        if args.next().is_some() {
            eprintln!("usage: bootstrap-frontend-oracle manifest <mallang.toml>");
            return ExitCode::from(2);
        }
        let project = match discover_project(&path) {
            Ok(project) => project,
            Err(error) => {
                println!("MERR|{}", encode_bytes(&error.to_string()));
                return ExitCode::SUCCESS;
            }
        };
        println!(
            "MANIFEST|1|{}|{}",
            encode_bytes(&project.manifest().project.name),
            project.manifest().dependencies.len()
        );
        for (name, dependency) in &project.manifest().dependencies {
            println!(
                "DEPENDENCY|{}|{}",
                encode_bytes(name),
                encode_bytes(&dependency.path)
            );
        }
        return ExitCode::SUCCESS;
    }
    if first == "project-plan" {
        let Some(root_manifest) = args.next() else {
            eprintln!("usage: bootstrap-frontend-oracle project-plan <root-manifest> <snapshot>...");
            return ExitCode::from(2);
        };
        let Some(unit_count) = args.next().and_then(|value| value.parse::<usize>().ok()) else {
            eprintln!("bootstrap frontend oracle received an invalid project snapshot count");
            return ExitCode::from(2);
        };
        for _ in 0..unit_count {
            if args.next().is_none() || args.next().is_none() {
                eprintln!("bootstrap frontend oracle received an incomplete project snapshot unit");
                return ExitCode::from(2);
            }
            let Some(dependency_count) =
                args.next().and_then(|value| value.parse::<usize>().ok())
            else {
                eprintln!("bootstrap frontend oracle received an invalid project dependency count");
                return ExitCode::from(2);
            };
            for _ in 0..dependency_count {
                if args.next().is_none() || args.next().is_none() || args.next().is_none() {
                    eprintln!("bootstrap frontend oracle received an incomplete project snapshot dependency");
                    return ExitCode::from(2);
                }
            }
        }
        if args.next().is_some() {
            eprintln!("bootstrap frontend oracle received trailing project snapshot arguments");
            return ExitCode::from(2);
        }
        let project = match discover_project(&root_manifest) {
            Ok(project) => project,
            Err(error) => {
                println!("GERR|{}", encode_bytes(&error.to_string()));
                return ExitCode::SUCCESS;
            }
        };
        let units = project.compiler_units().collect::<Vec<_>>();
        println!("PROJECT|1|{}", units.len());
        for unit in units {
            let dependencies = unit.direct_dependencies().collect::<Vec<_>>();
            print!(
                "UNIT|{}|{}|{}|{}",
                encode_bytes(unit.name()),
                encode_bytes(&unit.manifest_path().to_string_lossy()),
                encode_bytes(&unit.source_root().to_string_lossy()),
                dependencies.len()
            );
            for dependency in dependencies {
                print!("|{}", encode_bytes(dependency));
            }
            println!();
        }
        return ExitCode::SUCCESS;
    }
    if first == "parse-sources" {
        let paths = args.collect::<Vec<_>>();
        if paths.is_empty() {
            eprintln!("usage: bootstrap-frontend-oracle parse-sources <source>...");
            return ExitCode::from(2);
        }
        let mut sources = SourceMap::new();
        let mut source_ids = Vec::with_capacity(paths.len());
        for path in paths {
            let source = match fs::read_to_string(&path) {
                Ok(source) => source,
                Err(_) => {
                    eprintln!("bootstrap frontend oracle could not read source");
                    return ExitCode::from(2);
                }
            };
            source_ids.push(sources.add_file(path, source));
        }
        match parse_sources_with_diagnostics(&sources, &source_ids) {
            Ok(program) => println!("{}", normalize_program(&program).normalize(0)),
            Err(errors) => {
                for error in errors {
                    if let Some(span) = error.span {
                        println!(
                            "PERR|{}|{}|{}|{}",
                            span.source.index(),
                            span.start,
                            span.end,
                            encode_bytes(&error.message)
                        );
                    } else {
                        println!("PERR|-1|0|0|{}", encode_bytes(&error.message));
                    }
                }
            }
        }
        return ExitCode::SUCCESS;
    }
    if first == "package-layout"
        || first == "package-layout-project"
        || first == "link-project"
        || first == "augment-project"
        || first == "prepare-project"
        || first == "check-project"
        || first == "ir-project"
    {
        let operation = first.clone();
        let (source_root, paths) = if first == "package-layout" {
            let Some(_project_name) = args.next() else {
                eprintln!("usage: bootstrap-frontend-oracle package-layout <project-name> <source-root> <source>...");
                return ExitCode::from(2);
            };
            let Some(source_root) = args.next() else {
                eprintln!("usage: bootstrap-frontend-oracle package-layout <project-name> <source-root> <source>...");
                return ExitCode::from(2);
            };
            (source_root, args.collect::<Vec<_>>())
        } else {
            let Some(unit_count) = args.next().and_then(|value| value.parse::<usize>().ok()) else {
                eprintln!("usage: bootstrap-frontend-oracle package-layout-project <unit-count> (<name> <source-root> <dependency-count> <dependency>...)* <source>...");
                return ExitCode::from(2);
            };
            let mut root_source = None;
            for unit_index in 0..unit_count {
                let Some(_name) = args.next() else {
                    eprintln!("bootstrap frontend oracle received an incomplete project unit");
                    return ExitCode::from(2);
                };
                let Some(source_root) = args.next() else {
                    eprintln!("bootstrap frontend oracle received an incomplete project unit");
                    return ExitCode::from(2);
                };
                if unit_index == 0 {
                    root_source = Some(source_root);
                }
                let Some(dependency_count) =
                    args.next().and_then(|value| value.parse::<usize>().ok())
                else {
                    eprintln!("bootstrap frontend oracle received an invalid dependency count");
                    return ExitCode::from(2);
                };
                for _ in 0..dependency_count {
                    if args.next().is_none() {
                        eprintln!("bootstrap frontend oracle received an incomplete dependency list");
                        return ExitCode::from(2);
                    }
                }
            }
            let Some(source_root) = root_source else {
                eprintln!("bootstrap frontend oracle project graph requires a root unit");
                return ExitCode::from(2);
            };
            (source_root, args.collect::<Vec<_>>())
        };
        if paths.is_empty() {
            eprintln!("bootstrap frontend oracle project layout requires source files");
            return ExitCode::from(2);
        }
        let project = match discover_project(&source_root) {
            Ok(project) => project,
            Err(_) => {
                eprintln!("bootstrap frontend oracle could not discover project");
                return ExitCode::from(2);
            }
        };
        let mut sources = SourceMap::new();
        let mut source_ids = Vec::with_capacity(paths.len());
        for path in paths {
            let canonical_path = match fs::canonicalize(&path) {
                Ok(path) => path,
                Err(_) => {
                    eprintln!("bootstrap frontend oracle could not read source");
                    return ExitCode::from(2);
                }
            };
            let source = match fs::read_to_string(&canonical_path) {
                Ok(source) => source,
                Err(_) => {
                    eprintln!("bootstrap frontend oracle could not read source");
                    return ExitCode::from(2);
                }
            };
            source_ids.push(sources.add_file(canonical_path, source));
        }
        let program = match parse_sources_with_diagnostics(&sources, &source_ids) {
            Ok(program) => program,
            Err(errors) => {
                for error in errors {
                    if let Some(span) = error.span {
                        println!(
                            "PERR|{}|{}|{}|{}",
                            span.source.index(),
                            span.start,
                            span.end,
                            encode_bytes(&error.message)
                        );
                    } else {
                        println!("PERR|-1|0|0|{}", encode_bytes(&error.message));
                    }
                }
                return ExitCode::SUCCESS;
            }
        };
        match build_package_graph(&project, &sources, &program) {
            Ok(graph) => {
                if operation != "package-layout" && operation != "package-layout-project" {
                    let mut linked = match link_project(&project, &graph, &program) {
                        Ok(linked) => linked,
                        Err(error) => {
                            println!(
                            "LERR|{}|{}|{}|{}",
                            error.span.source.index(),
                            error.span.start,
                            error.span.end,
                            encode_bytes(&error.message)
                            );
                            return ExitCode::SUCCESS;
                        }
                    };
                    if operation == "link-project" {
                        println!("{}", normalize_program(&linked).normalize(0));
                        return ExitCode::SUCCESS;
                    }
                    augment_program(&mut linked, &graph);
                    if operation == "augment-project" {
                        println!("{}", normalize_program(&linked).normalize(0));
                        return ExitCode::SUCCESS;
                    }
                    if operation == "prepare-project" {
                        match specialize(&linked) {
                            Ok(prepared) => {
                                println!("{}", normalize_program(&prepared).normalize(0))
                            }
                            Err(error) => println!(
                                "SERR|{}|{}|{}|{}",
                                error.span.source.index(),
                                error.span.start,
                                error.span.end,
                                encode_bytes(&error.message)
                            ),
                        }
                        return ExitCode::SUCCESS;
                    }
                    match check_project(&linked, &graph) {
                        Ok(checked) => {
                            if operation == "check-project" {
                                println!("{}", normalize_checked(&checked));
                            } else {
                                match lower(&checked) {
                                    Ok(ir) => println!("{}", normalize_ir(&ir)),
                                    Err(error) => println!(
                                        "IERR|{}|{}|{}|{}",
                                        error.span.source.index(),
                                        error.span.start,
                                        error.span.end,
                                        encode_bytes(&error.message)
                                    ),
                                }
                            }
                        }
                        Err(error) => println!(
                            "SERR|{}|{}|{}|{}",
                            error.span.source.index(),
                            error.span.start,
                            error.span.end,
                            encode_bytes(&error.message)
                        ),
                    }
                    return ExitCode::SUCCESS;
                }
                println!("LAYOUT|{}|{}", source_ids.len(), graph.packages().len());
                for source_id in &source_ids {
                    let package = graph
                        .package_for_source(*source_id)
                        .expect("package graph contains every parsed source");
                    println!(
                        "SOURCE|{}|{}|{}",
                        source_id.index(),
                        package.path,
                        package.name
                    );
                }
                for package in graph.packages().values() {
                    let method_count = package.methods.values().map(|methods| methods.len()).sum::<usize>();
                    println!(
                        "PACKAGE|{}|{}|{}|{}|{}|{}",
                        package.path,
                        package.name,
                        package.source_ids.len(),
                        package.imports.len(),
                        package.declarations.len(),
                        method_count
                    );
                    for source_id in &package.source_ids {
                        println!("PSOURCE|{}|{}", package.path, source_id.index());
                    }
                    for import in &package.imports {
                        println!(
                            "IMPORT|{}|{}|{}",
                            package.path, import.path, import.qualifier
                        );
                    }
                    for declaration in package.declarations.values() {
                        print!(
                            "DECL|{}|{}|{:?}|{}|{}",
                            package.path,
                            package_declaration_kind(declaration.kind),
                            declaration.visibility,
                            declaration.name,
                            declaration.type_params.len()
                        );
                        for type_param in &declaration.type_params {
                            print!("|{type_param}");
                        }
                        println!();
                    }
                    for (receiver, methods) in &package.methods {
                        for declaration in methods.values() {
                            print!(
                                "METHOD|{}|{}|{}|{:?}|{}|{}",
                                package.path,
                                receiver,
                                package_declaration_kind(declaration.kind),
                                declaration.visibility,
                                declaration.name,
                                declaration.type_params.len()
                            );
                            for type_param in &declaration.type_params {
                                print!("|{type_param}");
                            }
                            println!();
                        }
                    }
                }
                print!("ORDER|{}", graph.build_order().len());
                for package_path in graph.build_order() {
                    print!("|{package_path}");
                }
                println!();
            }
            Err(error) => {
                if let Some(span) = error.span {
                    println!(
                        "KERR|{}|{}|{}|{}",
                        span.source.index(),
                        span.start,
                        span.end,
                        encode_bytes(&error.message)
                    );
                } else {
                    println!("KERR|-1|0|0|{}", encode_bytes(&error.message));
                }
            }
        }
        return ExitCode::SUCCESS;
    }
    let (mode, path) =
        if first == "parse" || first == "check" || first == "ir" || first == "ir-test" {
        let Some(path) = args.next() else {
            eprintln!("usage: bootstrap-frontend-oracle [parse|check|ir|ir-test] <source>");
            return ExitCode::from(2);
        };
        (first.as_str(), path)
    } else {
        ("lex", first)
    };
    if args.next().is_some() {
        eprintln!("usage: bootstrap-frontend-oracle [parse|check|ir|ir-test] <source>");
        return ExitCode::from(2);
    }

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(_) => {
            eprintln!("bootstrap frontend oracle could not read source");
            return ExitCode::from(2);
        }
    };

    if mode == "ir" || mode == "ir-test" {
        match parse_with_diagnostics(&source) {
            Ok(program) => match check(&program) {
                Ok(checked) => match if mode == "ir-test" {
                    lower_test(&checked, 0)
                } else {
                    lower(&checked)
                } {
                    Ok(ir) => println!("{}", normalize_ir(&ir)),
                    Err(error) => println!(
                        "IERR|{}|{}|{}|{}",
                        error.span.source.index(),
                        error.span.start,
                        error.span.end,
                        encode_bytes(&error.message)
                    ),
                },
                Err(error) => println!(
                    "SERR|{}|{}|{}|{}",
                    error.span.source.index(),
                    error.span.start,
                    error.span.end,
                    encode_bytes(&error.message)
                ),
            },
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
    } else if mode == "check" {
        match parse_with_diagnostics(&source) {
            Ok(program) => match check(&program) {
                Ok(checked) => println!("{}", normalize_checked(&checked)),
                Err(error) => println!(
                    "SERR|{}|{}|{}|{}",
                    error.span.source.index(),
                    error.span.start,
                    error.span.end,
                    encode_bytes(&error.message)
                ),
            },
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
    } else if mode == "parse" {
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

fn normalize_checked(checked: &CheckedProgram) -> String {
    let mut lines = vec![format!(
        "CHECKED|{}|{}|{}|{}",
        checked.structs.len(),
        checked.enums.len(),
        checked.signatures.len(),
        checked.methods.len()
    )];

    for declaration in &checked.program.structs {
        let signature = &checked.structs[&declaration.name];
        lines.push(format!(
            "STRUCT|{}|{}",
            declaration.name,
            signature.fields.len()
        ));
        for field in &signature.fields {
            lines.push(format!(
                "FIELD|{}|{}|{}",
                declaration.name,
                field.name,
                field.ty.source_name()
            ));
        }
    }
    for declaration in &checked.program.enums {
        let signature = &checked.enums[&declaration.name];
        lines.push(format!(
            "ENUM|{}|{}",
            declaration.name,
            signature.variants.len()
        ));
        for variant in &signature.variants {
            let mut line = format!(
                "VARIANT|{}|{}|{}",
                declaration.name,
                variant.name,
                variant.payloads.len()
            );
            for payload in &variant.payloads {
                line.push('|');
                line.push_str(&payload.source_name());
            }
            lines.push(line);
        }
    }
    for declaration in &checked.program.functions {
        if declaration.receiver.is_none() {
            let signature = &checked.signatures[&declaration.name];
            lines.push(format!(
                "FUNC|{}|{}|{}",
                declaration.name,
                signature.return_type.source_name(),
                signature.params.len()
            ));
            for param in &signature.params {
                lines.push(format!(
                    "PARAM|{}|{}|{}|{}",
                    declaration.name,
                    normalize_param_mode(param.mode),
                    param.name,
                    param.ty.source_name()
                ));
            }
        }
    }
    for declaration in &checked.program.functions {
        if let Some(receiver) = &declaration.receiver {
            let (_, method) = checked
                .methods
                .iter()
                .find(|(key, signature)| {
                    key.name == declaration.name
                        && key.receiver.source_name() == receiver.ty.name
                        && signature.receiver.name == receiver.name
                })
                .expect("checked source method must have a signature");
            lines.push(format!(
                "METHOD|{}|{}|{}|{}|{}|{}",
                method.receiver.ty.source_name(),
                declaration.name,
                normalize_param_mode(method.receiver.mode),
                method.receiver.name,
                method.function.return_type.source_name(),
                method.function.params.len()
            ));
            for param in &method.function.params {
                lines.push(format!(
                    "MPARAM|{}|{}|{}|{}|{}",
                    method.receiver.ty.source_name(),
                    declaration.name,
                    normalize_param_mode(param.mode),
                    param.name,
                    param.ty.source_name()
                ));
            }
        }
    }

    lines.join("\n")
}

fn normalize_param_mode(mode: ParamMode) -> &'static str {
    match mode {
        ParamMode::Owned => "owned",
        ParamMode::Con => "con",
        ParamMode::Mut => "mut",
    }
}

fn normalize_ir(program: &IrProgram) -> String {
    let mut lines = vec![format!("IR|{}", program.functions.len())];
    for function in &program.functions {
        lines.push(format!(
            "FUNCTION|{}|{}|{}|{}",
            function.name,
            function.return_type.source_name(),
            function.params.len(),
            function.body.len()
        ));
        for param in &function.params {
            lines.push(format!(
                "IPARAM|{}|{}|{}|{}",
                function.name,
                normalize_param_mode(param.mode),
                param.name,
                param.ty.source_name()
            ));
        }
        lines.extend(
            function
                .body
                .iter()
            .map(|statement| normalize_ir_statement(statement, 0)),
        );
    }
    lines.push(format!("CLOSURES|{}", program.closures.len()));
    for closure in &program.closures {
        lines.push(format!(
            "CLOSURE|{}|{}|{}|{}|{}|{}",
            closure.name,
            closure.mutable,
            closure.return_type.source_name(),
            closure.params.len(),
            closure.captures.len(),
            closure.body.len()
        ));
        for capture in &closure.captures {
            lines.push(format!(
                "CCAPTURE|{}|{}|{}|{}",
                closure.name,
                capture.mutable,
                capture.name,
                capture.ty.source_name()
            ));
        }
        for param in &closure.params {
            lines.push(format!(
                "CPARAM|{}|{}|{}|{}",
                closure.name,
                normalize_param_mode(param.mode),
                param.name,
                param.ty.source_name()
            ));
        }
        lines.extend(
            closure
                .body
                .iter()
                .map(|statement| normalize_ir_statement(statement, 0)),
        );
    }
    lines.join("\n")
}

fn normalize_ir_statement(statement: &IrStmt, depth: usize) -> String {
    let (kind, value, ty, children) = match &statement.kind {
        IrStmtKind::Let {
            mutable,
            name,
            ty,
            expr,
        } => (
            if *mutable {
                "Stmt.Let.Mutable"
            } else {
                "Stmt.Let.Immutable"
            },
            name.as_str(),
            ty.source_name(),
            vec![normalize_ir_expression(expr, depth + 1)],
        ),
        IrStmtKind::Assign { name, expr } => (
            "Stmt.Assign",
            name.as_str(),
            "unit".to_string(),
            vec![normalize_ir_expression(expr, depth + 1)],
        ),
        IrStmtKind::Return { expr } => (
            "Stmt.Return",
            "",
            "unit".to_string(),
            vec![normalize_ir_expression(expr, depth + 1)],
        ),
        IrStmtKind::If {
            condition,
            then_body,
            else_body,
        } => (
            "Stmt.If",
            "",
            "unit".to_string(),
            vec![
                normalize_ir_expression(condition, depth + 1),
                normalize_ir_block("Block.Then", then_body, statement.span, depth + 1),
                normalize_ir_block("Block.Else", else_body, statement.span, depth + 1),
            ],
        ),
        IrStmtKind::For {
            init,
            condition,
            post,
            body,
            cleanup,
        } => {
            let mut children = Vec::new();
            children.extend(
                init.iter()
                    .map(|init| normalize_ir_for_init(init, depth + 1)),
            );
            children.extend(condition.iter().map(|condition| {
                normalize_ir_line(
                    depth + 1,
                    "F",
                    "ForCondition",
                    condition.span,
                    "",
                    "bool",
                    &[normalize_ir_expression(condition, depth + 2)],
                )
            }));
            children.extend(
                post.iter()
                    .map(|post| normalize_ir_for_post(post, depth + 1)),
            );
            children.push(normalize_ir_block(
                "Block.For",
                body,
                statement.span,
                depth + 1,
            ));
            children.push(normalize_ir_block(
                "Block.ForCleanup",
                cleanup,
                statement.span,
                depth + 1,
            ));
            ("Stmt.For", "", "unit".to_string(), children)
        }
        IrStmtKind::RangeFor {
            index_name,
            value_name,
            source,
            element_ty,
            body,
            cleanup,
        } => {
            let bindings = format!("{index_name},{value_name}");
            let element_ty = element_ty.source_name();
            (
                "Stmt.RangeFor",
                "",
                "unit".to_string(),
                vec![
                    normalize_ir_line(
                        depth + 1,
                        "F",
                        "RangeBindings",
                        statement.span,
                        &bindings,
                        &element_ty,
                        &[],
                    ),
                    normalize_ir_expression(source, depth + 1),
                    normalize_ir_block("Block.RangeFor", body, statement.span, depth + 1),
                    normalize_ir_block(
                        "Block.RangeCleanup",
                        cleanup,
                        statement.span,
                        depth + 1,
                    ),
                ],
            )
        }
        IrStmtKind::Break => ("Stmt.Break", "", "unit".to_string(), Vec::new()),
        IrStmtKind::Continue => ("Stmt.Continue", "", "unit".to_string(), Vec::new()),
        IrStmtKind::Match { scrutinee, arms } => {
            let mut children = vec![normalize_ir_expression(scrutinee, depth + 1)];
            children.extend(
                arms.iter()
                    .map(|arm| normalize_ir_match_block_arm(arm, depth + 1)),
            );
            ("Stmt.Match", "", "unit".to_string(), children)
        }
        IrStmtKind::Assert { condition, .. } => (
            "Stmt.Assert",
            "",
            "unit".to_string(),
            vec![normalize_ir_expression(condition, depth + 1)],
        ),
        IrStmtKind::Expr { expr } => (
            "Stmt.Expr",
            "",
            "unit".to_string(),
            vec![normalize_ir_expression(expr, depth + 1)],
        ),
        IrStmtKind::FieldAssign { base, field, expr } => (
            "Stmt.FieldAssign",
            field.as_str(),
            "unit".to_string(),
            vec![
                normalize_ir_expression(base, depth + 1),
                normalize_ir_expression(expr, depth + 1),
            ],
        ),
        IrStmtKind::IndexAssign { base, index, expr } => (
            "Stmt.IndexAssign",
            "",
            "unit".to_string(),
            vec![
                normalize_ir_expression(base, depth + 1),
                normalize_ir_expression(index, depth + 1),
                normalize_ir_expression(expr, depth + 1),
            ],
        ),
        IrStmtKind::Overwrite { target, expr } => (
            "Stmt.Overwrite",
            "",
            "unit".to_string(),
            vec![
                normalize_ir_expression(target, depth + 1),
                normalize_ir_expression(expr, depth + 1),
            ],
        ),
        IrStmtKind::Drop { expr } => (
            "Stmt.Drop",
            "",
            "unit".to_string(),
            vec![normalize_ir_expression(expr, depth + 1)],
        ),
    };
    normalize_ir_line(
        depth,
        "S",
        kind,
        statement.span,
        value,
        &ty,
        &children,
    )
}

fn normalize_ir_block(kind: &str, body: &[IrStmt], span: Span, depth: usize) -> String {
    let children = body
        .iter()
        .map(|statement| normalize_ir_statement(statement, depth + 1))
        .collect::<Vec<_>>();
    normalize_ir_line(depth, "B", kind, span, "", "unit", &children)
}

fn normalize_ir_for_init(init: &IrForInit, depth: usize) -> String {
    match init {
        IrForInit::Let {
            mutable,
            name,
            ty,
            expr,
        } => normalize_ir_line(
            depth,
            "F",
            if *mutable {
                "ForInit.Let.Mutable"
            } else {
                "ForInit.Let.Immutable"
            },
            expr.span,
            name,
            &ty.source_name(),
            &[normalize_ir_expression(expr, depth + 1)],
        ),
    }
}

fn normalize_ir_for_post(post: &IrForPost, depth: usize) -> String {
    match post {
        IrForPost::Assign { target, expr } => normalize_ir_line(
            depth,
            "F",
            "ForPost.Assign",
            target.span.join(expr.span),
            "",
            "unit",
            &[
                normalize_ir_expression(target, depth + 1),
                normalize_ir_expression(expr, depth + 1),
            ],
        ),
    }
}

fn normalize_ir_expression(expression: &IrExpr, depth: usize) -> String {
    if let IrExprKind::SliceFieldTake { source } = &expression.kind {
        return normalize_ir_expression(source, depth);
    }
    let (kind, value, children) = match &expression.kind {
        IrExprKind::Int(value) => ("Expr.Int", value.to_string(), Vec::new()),
        IrExprKind::String(value) => ("Expr.String", value.clone(), Vec::new()),
        IrExprKind::Bool(value) => ("Expr.Bool", value.to_string(), Vec::new()),
        IrExprKind::Var(value) => ("Expr.Var", value.clone(), Vec::new()),
        IrExprKind::FullExprTemporary { name, expr } => (
            "Expr.FullExprTemporary",
            name.clone(),
            vec![normalize_ir_expression(expr, depth + 1)],
        ),
        IrExprKind::FunctionValue { function } => {
            ("Expr.FunctionValue", function.clone(), Vec::new())
        }
        IrExprKind::IntrinsicFunctionValue { intrinsic } => (
            "Expr.IntrinsicFunctionValue",
            format!("{intrinsic:?}"),
            Vec::new(),
        ),
        IrExprKind::ClosureValue { closure, captures } => (
            "Expr.ClosureValue",
            closure.clone(),
            captures
                .iter()
                .map(|capture| normalize_ir_capture(capture, depth + 1))
                .collect(),
        ),
        IrExprKind::Call { callee, args } => (
            "Expr.Call",
            callee.clone(),
            args.iter()
                .map(|arg| normalize_ir_argument(arg, depth + 1))
                .collect(),
        ),
        IrExprKind::IntrinsicCall { intrinsic, args } => (
            "Expr.IntrinsicCall",
            format!("{intrinsic:?}"),
            args.iter()
                .map(|arg| normalize_ir_argument(arg, depth + 1))
                .collect(),
        ),
        IrExprKind::IndirectCall { callee, args } => {
            let mut children = vec![normalize_ir_expression(callee, depth + 1)];
            children.extend(
                args.iter()
                    .map(|arg| normalize_ir_argument(arg, depth + 1)),
            );
            ("Expr.IndirectCall", String::new(), children)
        }
        IrExprKind::StructLiteral { type_name, fields } => (
            "Expr.StructLiteral",
            type_name.clone(),
            fields
                .iter()
                .map(|field| normalize_ir_field(field, depth + 1))
                .collect(),
        ),
        IrExprKind::ArrayLiteral { elements } => (
            "Expr.ArrayLiteral",
            String::new(),
            elements
                .iter()
                .map(|element| normalize_ir_expression(element, depth + 1))
                .collect(),
        ),
        IrExprKind::VariantConstructor {
            variant,
            storage,
            payloads,
        } => (
            match storage {
                IrEnumStorage::Inline => "Expr.VariantConstructor.Inline",
                IrEnumStorage::Owned => "Expr.VariantConstructor.Owned",
            },
            variant.clone(),
            payloads
                .iter()
                .map(|payload| normalize_ir_expression(payload, depth + 1))
                .collect(),
        ),
        IrExprKind::Match { scrutinee, arms } => {
            let mut children = vec![normalize_ir_expression(scrutinee, depth + 1)];
            children.extend(
                arms.iter()
                    .map(|arm| normalize_ir_match_arm(arm, depth + 1)),
            );
            ("Expr.Match", String::new(), children)
        }
        IrExprKind::FieldAccess { base, field } => (
            "Expr.FieldAccess",
            field.clone(),
            vec![normalize_ir_expression(base, depth + 1)],
        ),
        IrExprKind::Index { base, index } => (
            "Expr.Index",
            String::new(),
            vec![
                normalize_ir_expression(base, depth + 1),
                normalize_ir_expression(index, depth + 1),
            ],
        ),
        IrExprKind::ArrayLen { array } => (
            "Expr.ArrayLen",
            String::new(),
            vec![normalize_ir_expression(array, depth + 1)],
        ),
        IrExprKind::SliceAppend { slice, item } => (
            "Expr.SliceAppend",
            String::new(),
            vec![
                normalize_ir_expression(slice, depth + 1),
                normalize_ir_expression(item, depth + 1),
            ],
        ),
        IrExprKind::If {
            condition,
            then_branch,
            then_cleanup,
            else_branch,
            else_cleanup,
        } => (
            "Expr.If",
            String::new(),
            vec![
                normalize_ir_expression(condition, depth + 1),
                normalize_ir_expression(then_branch, depth + 1),
                normalize_ir_block(
                    "Block.IfThenCleanup",
                    then_cleanup,
                    expression.span,
                    depth + 1,
                ),
                normalize_ir_expression(else_branch, depth + 1),
                normalize_ir_block(
                    "Block.IfElseCleanup",
                    else_cleanup,
                    expression.span,
                    depth + 1,
                ),
            ],
        ),
        IrExprKind::Unary { op, expr } => (
            match op {
                UnaryOp::Negate => "Expr.Unary.Negate",
                UnaryOp::Not => "Expr.Unary.Not",
            },
            String::new(),
            vec![normalize_ir_expression(expr, depth + 1)],
        ),
        IrExprKind::Binary { op, left, right } => (
            match op {
                BinaryOp::Add => "Expr.Binary.Add",
                BinaryOp::Subtract => "Expr.Binary.Subtract",
                BinaryOp::Multiply => "Expr.Binary.Multiply",
                BinaryOp::Divide => "Expr.Binary.Divide",
                BinaryOp::Remainder => "Expr.Binary.Remainder",
                BinaryOp::Equal => "Expr.Binary.Equal",
                BinaryOp::NotEqual => "Expr.Binary.NotEqual",
                BinaryOp::LogicalAnd => "Expr.Binary.LogicalAnd",
                BinaryOp::LogicalOr => "Expr.Binary.LogicalOr",
                BinaryOp::Less => "Expr.Binary.Less",
                BinaryOp::LessEqual => "Expr.Binary.LessEqual",
                BinaryOp::Greater => "Expr.Binary.Greater",
                BinaryOp::GreaterEqual => "Expr.Binary.GreaterEqual",
            },
            String::new(),
            vec![
                normalize_ir_expression(left, depth + 1),
                normalize_ir_expression(right, depth + 1),
            ],
        ),
        other => panic!("unsupported P176b IR expression in oracle: {other:?}"),
    };
    normalize_ir_line(
        depth,
        "E",
        kind,
        expression.span,
        &value,
        &expression.ty.source_name(),
        &children,
    )
}

fn normalize_ir_field(field: &IrFieldValue, depth: usize) -> String {
    normalize_ir_line(
        depth,
        "F",
        "Field.Value",
        field.span,
        &field.name,
        &field.expr.ty.source_name(),
        &[normalize_ir_expression(&field.expr, depth + 1)],
    )
}

fn normalize_ir_match_arm(arm: &IrMatchArm, depth: usize) -> String {
    let mut children = vec![
        normalize_ir_match_pattern(&arm.pattern, arm.span, depth + 1),
        normalize_ir_expression(&arm.expr, depth + 1),
    ];
    children.extend(
        arm.cleanup
            .iter()
            .map(|statement| normalize_ir_statement(statement, depth + 1)),
    );
    normalize_ir_line(
        depth,
        "M",
        "Match.Arm",
        arm.span,
        "",
        &arm.expr.ty.source_name(),
        &children,
    )
}

fn normalize_ir_match_block_arm(arm: &IrMatchBlockArm, depth: usize) -> String {
    normalize_ir_line(
        depth,
        "M",
        "Match.BlockArm",
        arm.span,
        "",
        "unit",
        &[
            normalize_ir_match_pattern(&arm.pattern, arm.span, depth + 1),
            normalize_ir_block("Block.Match", &arm.body, arm.span, depth + 1),
        ],
    )
}

fn normalize_ir_match_pattern(pattern: &IrMatchPattern, span: Span, depth: usize) -> String {
    let (kind, value, ty, children) = match pattern {
        IrMatchPattern::Wildcard(ty) => (
            "Pattern.Wildcard",
            "",
            ty.source_name(),
            Vec::new(),
        ),
        IrMatchPattern::Binding { name, ty } => (
            "Pattern.Binding",
            name.as_str(),
            ty.source_name(),
            Vec::new(),
        ),
        IrMatchPattern::Variant {
            ty,
            variant,
            storage,
            payloads,
        } => (
            match storage {
                IrEnumStorage::Inline => "Pattern.Variant.Inline",
                IrEnumStorage::Owned => "Pattern.Variant.Owned",
            },
            variant.as_str(),
            ty.source_name(),
            payloads
                .iter()
                .map(|payload| normalize_ir_match_pattern(payload, span, depth + 1))
                .collect(),
        ),
    };
    normalize_ir_line(depth, "P", kind, span, value, &ty, &children)
}

fn normalize_ir_capture(capture: &IrClosureCaptureValue, depth: usize) -> String {
    normalize_ir_line(
        depth,
        "C",
        "Closure.Capture",
        capture.expr.span,
        &capture.name,
        &capture.expr.ty.source_name(),
        &[normalize_ir_expression(&capture.expr, depth + 1)],
    )
}

fn normalize_ir_argument(argument: &IrArg, depth: usize) -> String {
    let kind = match argument.mode {
        ArgMode::Owned => "Arg.Owned",
        ArgMode::Con => "Arg.Con",
        ArgMode::Mut => "Arg.Mut",
    };
    normalize_ir_line(
        depth,
        "A",
        kind,
        argument.span,
        "",
        &argument.expr.ty.source_name(),
        &[normalize_ir_expression(&argument.expr, depth + 1)],
    )
}

fn normalize_ir_line(
    depth: usize,
    category: &str,
    kind: &str,
    span: Span,
    value: &str,
    ty: &str,
    children: &[String],
) -> String {
    let mut lines = vec![format!(
        "I|{depth}|{category}|{kind}|{}|{}|{}|{}|{ty}|{}",
        span.source.index(),
        span.start,
        span.end,
        encode_bytes(value),
        children.len()
    )];
    lines.extend(children.iter().cloned());
    lines.join("\n")
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
    if let Some(intrinsic) = declaration.intrinsic {
        children.push(NormalizedNode::new(
            "Intrinsic.Type",
            format!("{intrinsic:?}"),
            declaration.span,
            declaration
                .intrinsic_args
                .iter()
                .map(normalize_type_ref)
                .collect(),
        ));
    }
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
    if let Some(intrinsic) = declaration.intrinsic {
        children.push(NormalizedNode::new(
            "Intrinsic.Type",
            format!("{intrinsic:?}"),
            declaration.span,
            Vec::new(),
        ));
    }
    if let Some(origin) = &declaration.specialization_origin {
        children.push(NormalizedNode::new(
            "SpecializationOrigin",
            origin,
            declaration.span,
            Vec::new(),
        ));
    }
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
    if let Some(intrinsic) = function.intrinsic {
        children.push(NormalizedNode::new(
            "Intrinsic.Function",
            format!("{intrinsic:?}"),
            function.span,
            Vec::new(),
        ));
    }
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
    NormalizedNode::new(
        "Block",
        "",
        block.span,
        block.statements.iter().map(normalize_stmt).collect(),
    )
}

fn normalize_stmt(statement: &Stmt) -> NormalizedNode {
    match &statement.kind {
        StmtKind::Let {
            mutable,
            name,
            expr,
        } => NormalizedNode::new(
            if *mutable {
                "Stmt.Let.Mutable"
            } else {
                "Stmt.Let.Immutable"
            },
            name,
            statement.span,
            vec![normalize_expr(expr)],
        ),
        StmtKind::Assign { name, expr } => NormalizedNode::new(
            "Stmt.Assign",
            name,
            statement.span,
            vec![normalize_expr(expr)],
        ),
        StmtKind::FieldAssign { base, field, expr } => NormalizedNode::new(
            "Stmt.FieldAssign",
            field,
            statement.span,
            vec![normalize_expr(base), normalize_expr(expr)],
        ),
        StmtKind::IndexAssign { base, index, expr } => NormalizedNode::new(
            "Stmt.IndexAssign",
            "",
            statement.span,
            vec![
                normalize_expr(base),
                normalize_expr(index),
                normalize_expr(expr),
            ],
        ),
        StmtKind::Return { expr } => NormalizedNode::new(
            "Stmt.Return",
            "",
            statement.span,
            vec![normalize_expr(expr)],
        ),
        StmtKind::If {
            condition,
            then_block,
            else_block,
        } => {
            let mut children = vec![normalize_expr(condition), normalize_block(then_block)];
            children.extend(else_block.iter().map(normalize_block));
            NormalizedNode::new("Stmt.If", "", statement.span, children)
        }
        StmtKind::For {
            init,
            condition,
            post,
            body,
        } => {
            let mut children = init.iter().map(normalize_for_init).collect::<Vec<_>>();
            children.extend(condition.iter().map(|condition| {
                NormalizedNode::new(
                    "ForCondition",
                    "",
                    condition.span,
                    vec![normalize_expr(condition)],
                )
            }));
            children.extend(post.iter().map(normalize_for_post));
            children.push(normalize_block(body));
            NormalizedNode::new("Stmt.For", "", statement.span, children)
        }
        StmtKind::RangeFor {
            index_name,
            value_name,
            source,
            body,
        } => NormalizedNode::new(
            "Stmt.RangeFor",
            format!("{index_name},{value_name}"),
            statement.span,
            vec![normalize_expr(source), normalize_block(body)],
        ),
        StmtKind::Break => NormalizedNode::new("Stmt.Break", "", statement.span, Vec::new()),
        StmtKind::Continue => {
            NormalizedNode::new("Stmt.Continue", "", statement.span, Vec::new())
        }
        StmtKind::Match { scrutinee, arms } => {
            let mut children = vec![normalize_expr(scrutinee)];
            children.extend(arms.iter().map(|arm| {
                NormalizedNode::new(
                    "MatchBlockArm",
                    "",
                    arm.span,
                    vec![
                        normalize_pattern(&arm.pattern, arm.span),
                        normalize_block(&arm.block),
                    ],
                )
            }));
            NormalizedNode::new("Stmt.Match", "", statement.span, children)
        }
        StmtKind::Assert { condition } => NormalizedNode::new(
            "Stmt.Assert",
            "",
            statement.span,
            vec![normalize_expr(condition)],
        ),
        StmtKind::Expr { expr } => NormalizedNode::new(
            "Stmt.Expr",
            "",
            statement.span,
            vec![normalize_expr(expr)],
        ),
    }
}

fn normalize_for_init(init: &ForInit) -> NormalizedNode {
    match init {
        ForInit::Let {
            mutable,
            name,
            expr,
        } => NormalizedNode::new(
            if *mutable {
                "ForInit.Let.Mutable"
            } else {
                "ForInit.Let.Immutable"
            },
            name,
            expr.span,
            vec![normalize_expr(expr)],
        ),
    }
}

fn normalize_for_post(post: &ForPost) -> NormalizedNode {
    match post {
        ForPost::Assign { target, expr } => NormalizedNode::new(
            "ForPost.Assign",
            "",
            target.span.join(expr.span),
            vec![normalize_expr(target), normalize_expr(expr)],
        ),
    }
}

fn normalize_expr(expression: &Expr) -> NormalizedNode {
    match &expression.kind {
        ExprKind::Int(value) => NormalizedNode::new(
            "Expr.Int",
            value.to_string(),
            expression.span,
            Vec::new(),
        ),
        ExprKind::String(value) => {
            NormalizedNode::new("Expr.String", value, expression.span, Vec::new())
        }
        ExprKind::Bool(value) => NormalizedNode::new(
            "Expr.Bool",
            value.to_string(),
            expression.span,
            Vec::new(),
        ),
        ExprKind::Nil => NormalizedNode::new("Expr.Nil", "", expression.span, Vec::new()),
        ExprKind::Var(name) => {
            NormalizedNode::new("Expr.Var", name, expression.span, Vec::new())
        }
        ExprKind::FunctionLiteral(function) => {
            let mut children = function
                .params
                .iter()
                .map(|param| normalize_param(param, "Param"))
                .collect::<Vec<_>>();
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
                if function.mutable {
                    "Expr.FunctionLiteral.Mutable"
                } else {
                    "Expr.FunctionLiteral.Immutable"
                },
                "",
                expression.span,
                children,
            )
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => NormalizedNode::new(
            "Expr.If",
            "",
            expression.span,
            vec![
                normalize_expr(condition),
                normalize_expr(then_branch),
                normalize_expr(else_branch),
            ],
        ),
        ExprKind::Match { scrutinee, arms } => {
            let mut children = vec![normalize_expr(scrutinee)];
            children.extend(arms.iter().map(|arm| {
                NormalizedNode::new(
                    "MatchArm",
                    "",
                    arm.span,
                    vec![
                        normalize_pattern(&arm.pattern, arm.span),
                        normalize_expr(&arm.expr),
                    ],
                )
            }));
            NormalizedNode::new("Expr.Match", "", expression.span, children)
        }
        ExprKind::StructLiteral {
            type_name,
            type_args,
            fields,
        } => {
            let mut children = type_args.iter().map(normalize_type_ref).collect::<Vec<_>>();
            children.extend(fields.iter().map(normalize_field_init));
            NormalizedNode::new(
                "Expr.StructLiteral",
                type_name,
                expression.span,
                children,
            )
        }
        ExprKind::ArrayLiteral { ty, elements } => {
            let mut children = vec![normalize_type_ref(ty)];
            children.extend(elements.iter().map(normalize_expr));
            NormalizedNode::new("Expr.ArrayLiteral", "", expression.span, children)
        }
        ExprKind::FieldAccess { base, field } => NormalizedNode::new(
            "Expr.FieldAccess",
            field,
            expression.span,
            vec![normalize_expr(base)],
        ),
        ExprKind::Index { base, index } => NormalizedNode::new(
            "Expr.Index",
            "",
            expression.span,
            vec![normalize_expr(base), normalize_expr(index)],
        ),
        ExprKind::TypeApply { base, args } => {
            let mut children = vec![normalize_expr(base)];
            children.extend(args.iter().map(normalize_type_ref));
            NormalizedNode::new("Expr.TypeApply", "", expression.span, children)
        }
        ExprKind::EnumConstructor {
            enum_name,
            variant,
            args,
        } => NormalizedNode::new(
            "Expr.EnumConstructor",
            format!("{enum_name}.{variant}"),
            expression.span,
            args.iter()
                .flatten()
                .map(normalize_arg)
                .collect::<Vec<_>>(),
        ),
        ExprKind::Call { callee, args } => {
            let mut children = vec![normalize_expr(callee)];
            children.extend(args.iter().map(normalize_arg));
            NormalizedNode::new("Expr.Call", "", expression.span, children)
        }
        ExprKind::Unary { op, expr } => NormalizedNode::new(
            format!("Expr.Unary.{}", unary_name(*op)),
            "",
            expression.span,
            vec![normalize_expr(expr)],
        ),
        ExprKind::Binary { op, left, right } => NormalizedNode::new(
            format!("Expr.Binary.{}", binary_name(*op)),
            "",
            expression.span,
            vec![normalize_expr(left), normalize_expr(right)],
        ),
    }
}

fn normalize_field_init(field: &FieldInit) -> NormalizedNode {
    NormalizedNode::new(
        "FieldInit",
        &field.name,
        field.span,
        vec![normalize_expr(&field.expr)],
    )
}

fn normalize_arg(arg: &Arg) -> NormalizedNode {
    NormalizedNode::new(
        format!("Arg.{}", arg_mode_name(arg.mode)),
        "",
        arg.span,
        vec![normalize_expr(&arg.expr)],
    )
}

fn normalize_pattern(pattern: &MatchPattern, span: Span) -> NormalizedNode {
    match pattern {
        MatchPattern::Some(binding) => {
            NormalizedNode::new("Pattern.Some", binding, span, Vec::new())
        }
        MatchPattern::None => NormalizedNode::new("Pattern.None", "", span, Vec::new()),
        MatchPattern::Ok(binding) => {
            NormalizedNode::new("Pattern.Ok", binding, span, Vec::new())
        }
        MatchPattern::Err(binding) => {
            NormalizedNode::new("Pattern.Err", binding, span, Vec::new())
        }
        MatchPattern::Wildcard => {
            NormalizedNode::new("Pattern.Wildcard", "", span, Vec::new())
        }
        MatchPattern::Binding(binding) => {
            NormalizedNode::new("Pattern.Binding", binding, span, Vec::new())
        }
        MatchPattern::Variant {
            type_name,
            variant,
            payloads,
        } => NormalizedNode::new(
            "Pattern.Variant",
            format!("{type_name}.{variant}"),
            span,
            payloads
                .iter()
                .map(|payload| normalize_pattern(payload, span))
                .collect(),
        ),
        MatchPattern::NestedBuiltin { variant, payload } => NormalizedNode::new(
            "Pattern.NestedBuiltin",
            variant,
            span,
            vec![normalize_pattern(payload, span)],
        ),
    }
}

fn arg_mode_name(mode: ArgMode) -> &'static str {
    match mode {
        ArgMode::Owned => "Owned",
        ArgMode::Con => "Con",
        ArgMode::Mut => "Mut",
    }
}

fn unary_name(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "Negate",
        UnaryOp::Not => "Not",
    }
}

fn binary_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "Add",
        BinaryOp::Subtract => "Subtract",
        BinaryOp::Multiply => "Multiply",
        BinaryOp::Divide => "Divide",
        BinaryOp::Remainder => "Remainder",
        BinaryOp::Equal => "Equal",
        BinaryOp::NotEqual => "NotEqual",
        BinaryOp::LogicalAnd => "LogicalAnd",
        BinaryOp::LogicalOr => "LogicalOr",
        BinaryOp::Less => "Less",
        BinaryOp::LessEqual => "LessEqual",
        BinaryOp::Greater => "Greater",
        BinaryOp::GreaterEqual => "GreaterEqual",
    }
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

fn package_declaration_kind(kind: PackageDeclarationKind) -> &'static str {
    match kind {
        PackageDeclarationKind::Struct => "Struct",
        PackageDeclarationKind::Opaque => "Opaque",
        PackageDeclarationKind::Enum => "Enum",
        PackageDeclarationKind::Function => "Function",
        PackageDeclarationKind::Method => "Method",
    }
}
