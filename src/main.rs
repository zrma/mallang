use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command},
};

use mallang::{
    check_project_sources, check_sources, discover_project, format_source,
    generate_c_project_sources, generate_c_sources, lex_with_source, load_source_files,
    lower_sources, parse_sources, CompilerError, FormatError, FrontendError, Project, SourceId,
    SourceMap,
};

fn main() {
    let args: Vec<OsString> = env::args_os().collect();
    let program = args
        .first()
        .map(|argument| argument.to_string_lossy().into_owned())
        .unwrap_or_else(|| "mlg".to_string());
    if args.len() < 2 {
        let mut stderr = io::stderr().lock();
        let _ = write_usage(&mut stderr, &program);
        process::exit(2);
    }

    let result = match args[1].to_str() {
        Some("lex") => utf8_cli_args(&args[2..]).and_then(|args| run_lex(&program, &args)),
        Some("parse") => utf8_cli_args(&args[2..]).and_then(|args| run_parse(&program, &args)),
        Some("check") => utf8_cli_args(&args[2..]).and_then(|args| run_check(&program, &args)),
        Some("fmt") => utf8_cli_args(&args[2..]).and_then(|args| run_fmt(&program, &args)),
        Some("ir") => utf8_cli_args(&args[2..]).and_then(|args| run_ir(&program, &args)),
        Some("build") => utf8_cli_args(&args[2..]).and_then(|args| run_build(&program, &args)),
        Some("run") => run_run(&program, &args[2..]),
        Some("-V" | "--version") => {
            println!("mlg {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("-h" | "--help") => {
            let mut stdout = io::stdout().lock();
            write_usage(&mut stdout, &program)
                .map_err(|error| format!("failed to write usage: {error}"))
        }
        Some(command) => Err(format!(
            "unknown subcommand `{command}`; run `{program} --help` for usage"
        )),
        None => Err("subcommand is not valid UTF-8".to_string()),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        process::exit(1);
    }
}

fn utf8_cli_args(args: &[OsString]) -> Result<Vec<String>, String> {
    args.iter()
        .cloned()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| "command argument is not valid UTF-8".to_string())
        })
        .collect()
}

fn write_usage(output: &mut impl Write, program: &str) -> io::Result<()> {
    writeln!(output, "usage:")?;
    writeln!(output, "  {program} lex <source-file>")?;
    writeln!(output, "  {program} parse <source-file>")?;
    writeln!(output, "  {program} check <input>")?;
    writeln!(output, "  {program} fmt [--check] <input>")?;
    writeln!(output, "  {program} ir <source-file>")?;
    writeln!(output, "  {program} build <input> [-o <output>]")?;
    writeln!(output, "  {program} run <input> [-- <program-args>...]")?;
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

fn run_fmt(program: &str, args: &[String]) -> Result<(), String> {
    let (check_only, input) = match args {
        [input] if input != "--check" => (false, input.as_str()),
        [flag, input] if flag == "--check" => (true, input.as_str()),
        [flag, ..] if flag.starts_with('-') && flag != "--check" => {
            return Err(format!("unknown fmt argument `{flag}`"));
        }
        _ => return Err(format!("usage: {program} fmt [--check] <input>")),
    };

    let files = load_format_inputs(input)?;
    let mut changes = Vec::new();

    for file in files {
        let formatted = format_source(&file.source)
            .map_err(|error| format_format_error(&file.display_path, &file.source, error))?;
        if formatted != file.source {
            changes.push((file.path, file.display_path, formatted));
        }
    }

    if check_only {
        if changes.is_empty() {
            return Ok(());
        }
        return Err(changes
            .iter()
            .map(|(_, display_path, _)| format!("{}: not formatted", display_path.display()))
            .collect::<Vec<_>>()
            .join("\n"));
    }

    for (path, display_path, formatted) in changes {
        fs::write(&path, formatted)
            .map_err(|error| format!("failed to write {}: {error}", display_path.display()))?;
        println!("{}: formatted", display_path.display());
    }
    Ok(())
}

struct FormatInput {
    path: PathBuf,
    display_path: PathBuf,
    source: String,
}

fn load_format_inputs(input: &str) -> Result<Vec<FormatInput>, String> {
    let input_path = Path::new(input);
    if input_path
        .extension()
        .is_some_and(|extension| extension == "mlg")
    {
        return Ok(vec![load_format_input(
            input_path.to_path_buf(),
            input_path.to_path_buf(),
        )?]);
    }

    let project = discover_project(input_path).map_err(|error| error.to_string())?;
    project
        .source_files()
        .iter()
        .map(|path| {
            let display_path = path
                .strip_prefix(project.root())
                .unwrap_or(path)
                .to_path_buf();
            load_format_input(path.clone(), display_path)
        })
        .collect()
}

fn load_format_input(path: PathBuf, display_path: PathBuf) -> Result<FormatInput, String> {
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", display_path.display()))?;
    Ok(FormatInput {
        path,
        display_path,
        source,
    })
}

fn format_format_error(display_path: &Path, source: &str, error: FormatError) -> String {
    let mut sources = SourceMap::new();
    let source_id = sources.add_file(display_path, source);
    sources.format_diagnostic(
        &error.message,
        mallang::Span::new(source_id, error.span.start, error.span.end),
    )
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

fn run_run(program: &str, args: &[OsString]) -> Result<(), String> {
    let Some(source_path) = args.first() else {
        return Err(format!(
            "usage: {program} run <input> [-- <program-args>...]"
        ));
    };
    let source_path = source_path
        .to_str()
        .ok_or_else(|| "run input path is not valid UTF-8".to_string())?;
    let program_args = match args.get(1) {
        None => &args[1..],
        Some(argument) if argument == OsStr::new("--") => &args[2..],
        Some(argument) => {
            return Err(format!(
                "unknown run argument `{}`; program arguments must follow `--`",
                argument.to_string_lossy()
            ));
        }
    };
    let binary_path = compile_input(source_path, None, OutputKind::Run)?;

    let status = Command::new(&binary_path)
        .args(program_args)
        .status()
        .map_err(|error| format!("failed to execute {}: {error}", binary_path.display()))?;
    if !status.success() {
        if let Some(code) = status.code() {
            process::exit(code);
        }
        return Err(format!("program terminated by signal: {status}"));
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
