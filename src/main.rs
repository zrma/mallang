use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command},
};

use mallang::{
    check_project_sources, check_sources, discover_project, generate_c_project_sources,
    generate_c_sources, lex_with_source, load_source_files, lower_sources, parse_sources,
    CompilerError, FrontendError, Project, SourceId, SourceMap,
};

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let program = args.first().cloned().unwrap_or_else(|| "mlg".to_string());
    if args.len() < 2 {
        let mut stderr = io::stderr().lock();
        let _ = write_usage(&mut stderr, &program);
        process::exit(2);
    }
    args.remove(0);

    let result = match args[0].as_str() {
        "lex" => {
            args.remove(0);
            run_lex(&program, &args)
        }
        "parse" => {
            args.remove(0);
            run_parse(&program, &args)
        }
        "check" => {
            args.remove(0);
            run_check(&program, &args)
        }
        "ir" => {
            args.remove(0);
            run_ir(&program, &args)
        }
        "build" => {
            args.remove(0);
            run_build(&program, &args)
        }
        "run" => {
            args.remove(0);
            run_run(&program, &args)
        }
        "-V" | "--version" => {
            println!("mlg {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "-h" | "--help" => {
            let mut stdout = io::stdout().lock();
            write_usage(&mut stdout, &program)
                .map_err(|error| format!("failed to write usage: {error}"))
        }
        command => Err(format!(
            "unknown subcommand `{command}`; run `{program} --help` for usage"
        )),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn write_usage(output: &mut impl Write, program: &str) -> io::Result<()> {
    writeln!(output, "usage:")?;
    writeln!(output, "  {program} lex <source-file>")?;
    writeln!(output, "  {program} parse <source-file>")?;
    writeln!(output, "  {program} check <input>")?;
    writeln!(output, "  {program} ir <source-file>")?;
    writeln!(output, "  {program} build <input> [-o <output>]")?;
    writeln!(output, "  {program} run <input>")?;
    writeln!(output, "  {program} --version")
}

fn run_lex(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_source_arg(program, "lex", args)?;
    let (sources, source_id) = load_source(path)?;
    let source = source_text(&sources, source_id);

    match lex_with_source(source, source_id) {
        Ok(tokens) => {
            for token in tokens {
                println!(
                    "{:?} @ {}..{}",
                    token.kind, token.span.start, token.span.end
                );
            }
            Ok(())
        }
        Err(error) => Err(sources.format_diagnostic(&error.message, error.span)),
    }
}

fn run_parse(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_source_arg(program, "parse", args)?;
    let (sources, source_id) = load_source(path)?;
    let program = parse_loaded_source(&sources, source_id)?;
    println!("{program:#?}");
    Ok(())
}

fn run_check(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_input_arg(program, "check", args)?;
    match load_compilation_input(path)? {
        CompilationInput::Standalone { sources, source_id } => {
            check_sources(&sources, &[source_id])
                .map_err(|error| format_compiler_error(&sources, path, error))?;
        }
        CompilationInput::Project {
            project,
            sources,
            source_ids,
        } => {
            check_project_sources(&project, &sources, &source_ids)
                .map_err(|error| format_compiler_error(&sources, path, error))?;
        }
    }
    println!("{path}: ok");
    Ok(())
}

fn run_ir(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_source_arg(program, "ir", args)?;
    let (sources, source_id) = load_source(path)?;
    let ir = lower_sources(&sources, &[source_id])
        .map_err(|error| format_compiler_error(&sources, path, error))?;
    println!("{ir:#?}");
    Ok(())
}

fn run_build(program: &str, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err(format!("usage: {program} build <input> [-o <output>]"));
    }

    let source_path = &args[0];
    let mut output_path = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing value for -o/--output".to_string());
                };
                output_path = Some(PathBuf::from(path));
                index += 2;
            }
            arg => return Err(format!("unknown build argument `{arg}`")),
        }
    }

    let output_path = compile_input(source_path, output_path, OutputKind::Build)?;
    println!("{}", output_path.display());
    Ok(())
}

fn run_run(program: &str, args: &[String]) -> Result<(), String> {
    let source_path = single_input_arg(program, "run", args)?;
    let binary_path = compile_input(source_path, None, OutputKind::Run)?;

    let status = Command::new(&binary_path)
        .status()
        .map_err(|error| format!("failed to execute {}: {error}", binary_path.display()))?;
    if !status.success() {
        return Err(format!("program exited with status {status}"));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum OutputKind {
    Build,
    Run,
}

fn compile_input(
    input_path: &str,
    output_path: Option<PathBuf>,
    kind: OutputKind,
) -> Result<PathBuf, String> {
    let (c_source, artifact_name, build_dir) = match load_compilation_input(input_path)? {
        CompilationInput::Standalone { sources, source_id } => {
            let c_source = generate_c_sources(&sources, &[source_id])
                .map_err(|error| format_compiler_error(&sources, input_path, error))?;
            (
                c_source,
                source_stem(input_path).to_string(),
                PathBuf::from("target/mallang"),
            )
        }
        CompilationInput::Project {
            project,
            sources,
            source_ids,
        } => {
            let c_source = generate_c_project_sources(&project, &sources, &source_ids)
                .map_err(|error| format_compiler_error(&sources, input_path, error))?;
            let artifact_name = project.name().to_string();
            let build_dir = project.root().join("target/mallang");
            (c_source, artifact_name, build_dir)
        }
    };

    fs::create_dir_all(&build_dir)
        .map_err(|error| format!("failed to create {}: {error}", build_dir.display()))?;
    let c_path = build_dir.join(format!("{artifact_name}.c"));
    fs::write(&c_path, c_source)
        .map_err(|error| format!("failed to write {}: {error}", c_path.display()))?;

    let default_output_dir = match kind {
        OutputKind::Build => build_dir.clone(),
        OutputKind::Run => build_dir.join("run"),
    };
    let output_path = output_path.unwrap_or_else(|| default_output_dir.join(&artifact_name));
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let status = Command::new("clang")
        .arg(&c_path)
        .arg("-o")
        .arg(&output_path)
        .status()
        .map_err(|error| format!("failed to execute clang: {error}"))?;
    if !status.success() {
        return Err(format!("clang failed with status {status}"));
    }

    Ok(output_path)
}

fn source_stem(source_path: &str) -> &str {
    Path::new(source_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("mallang")
}

fn single_source_arg<'a>(
    program: &str,
    subcommand: &str,
    args: &'a [String],
) -> Result<&'a str, String> {
    if args.len() != 1 {
        return Err(format!("usage: {program} {subcommand} <source-file>"));
    }
    Ok(&args[0])
}

fn single_input_arg<'a>(
    program: &str,
    subcommand: &str,
    args: &'a [String],
) -> Result<&'a str, String> {
    if args.len() != 1 {
        return Err(format!("usage: {program} {subcommand} <input>"));
    }
    Ok(&args[0])
}

enum CompilationInput {
    Standalone {
        sources: SourceMap,
        source_id: SourceId,
    },
    Project {
        project: Project,
        sources: SourceMap,
        source_ids: Vec<SourceId>,
    },
}

fn load_compilation_input(path: &str) -> Result<CompilationInput, String> {
    let input = Path::new(path);
    if input
        .extension()
        .is_some_and(|extension| extension == "mlg")
    {
        let (sources, source_id) = load_source(path)?;
        return Ok(CompilationInput::Standalone { sources, source_id });
    }

    let project = discover_project(input).map_err(|error| error.to_string())?;
    let loaded =
        load_source_files(project.source_files().iter()).map_err(|error| error.to_string())?;
    Ok(CompilationInput::Project {
        project,
        sources: loaded.sources,
        source_ids: loaded.source_ids,
    })
}

fn load_source(path: &str) -> Result<(SourceMap, SourceId), String> {
    let loaded = load_source_files([path]).map_err(|error| error.to_string())?;
    let source_id = loaded
        .source_ids
        .first()
        .copied()
        .ok_or_else(|| "source loader returned no source IDs".to_string())?;
    Ok((loaded.sources, source_id))
}

fn source_text(sources: &SourceMap, source_id: SourceId) -> &str {
    sources
        .file(source_id)
        .expect("source ID returned by SourceMap must resolve")
        .text()
}

fn parse_loaded_source(
    sources: &SourceMap,
    source_id: SourceId,
) -> Result<mallang::ast::Program, String> {
    parse_sources(sources, &[source_id]).map_err(|error| format_frontend_error(sources, error))
}

fn format_frontend_error(sources: &SourceMap, error: FrontendError) -> String {
    match error.span {
        Some(span) => sources.format_diagnostic(&error.message, span),
        None => error.message,
    }
}

fn format_compiler_error(sources: &SourceMap, fallback_path: &str, error: CompilerError) -> String {
    match error.span {
        Some(span) => sources.format_diagnostic(&error.message, span),
        None => format!("{fallback_path}: {}", error.message),
    }
}
