use std::{
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command},
};

use mallang::{
    check_sources, generate_c_sources, lex_with_source, load_source_files, lower_sources,
    parse_sources, CompilerError, FrontendError, SourceId, SourceMap,
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
    writeln!(output, "  {program} check <source-file>")?;
    writeln!(output, "  {program} ir <source-file>")?;
    writeln!(output, "  {program} build <source-file> [-o <output>]")?;
    writeln!(output, "  {program} run <source-file>")?;
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
    let path = single_source_arg(program, "check", args)?;
    let (sources, source_id) = load_source(path)?;
    check_sources(&sources, &[source_id])
        .map_err(|error| format_compiler_error(&sources, path, error))?;
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
        return Err(format!(
            "usage: {program} build <source-file> [-o <output>]"
        ));
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

    let output_path = compile_source(source_path, output_path)?;
    println!("{}", output_path.display());
    Ok(())
}

fn run_run(program: &str, args: &[String]) -> Result<(), String> {
    let source_path = single_source_arg(program, "run", args)?;
    let source_stem = source_stem(source_path);
    let output_path = PathBuf::from("target")
        .join("mallang")
        .join("run")
        .join(source_stem);
    let binary_path = compile_source(source_path, Some(output_path))?;

    let status = Command::new(&binary_path)
        .status()
        .map_err(|error| format!("failed to execute {}: {error}", binary_path.display()))?;
    if !status.success() {
        return Err(format!("program exited with status {status}"));
    }

    Ok(())
}

fn compile_source(source_path: &str, output_path: Option<PathBuf>) -> Result<PathBuf, String> {
    let (sources, source_id) = load_source(source_path)?;
    let c_source = generate_c_sources(&sources, &[source_id])
        .map_err(|error| format_compiler_error(&sources, source_path, error))?;

    let source_stem = source_stem(source_path);
    let build_dir = PathBuf::from("target/mallang");
    fs::create_dir_all(&build_dir)
        .map_err(|error| format!("failed to create {}: {error}", build_dir.display()))?;
    let c_path = build_dir.join(format!("{source_stem}.c"));
    fs::write(&c_path, c_source)
        .map_err(|error| format!("failed to write {}: {error}", c_path.display()))?;

    let output_path = output_path.unwrap_or_else(|| build_dir.join(source_stem));
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
