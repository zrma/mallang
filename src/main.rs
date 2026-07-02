use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

use mallang::{check, generate_c, lex, lower, parse};

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let program = args.first().cloned().unwrap_or_else(|| "mlg".to_string());
    if args.len() < 2 {
        usage(&program);
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
        "-h" | "--help" => {
            usage(&program);
            Ok(())
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

fn usage(program: &str) {
    eprintln!("usage:");
    eprintln!("  {program} lex <source-file>");
    eprintln!("  {program} parse <source-file>");
    eprintln!("  {program} check <source-file>");
    eprintln!("  {program} ir <source-file>");
    eprintln!("  {program} build <source-file> [-o <output>]");
    eprintln!("  {program} run <source-file>");
}

fn run_lex(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_source_arg(program, "lex", args)?;
    let source = read_source(path)?;

    match lex(&source) {
        Ok(tokens) => {
            for token in tokens {
                println!(
                    "{:?} @ {}..{}",
                    token.kind, token.span.start, token.span.end
                );
            }
            Ok(())
        }
        Err(error) => Err(format!("{path}: {error}")),
    }
}

fn run_parse(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_source_arg(program, "parse", args)?;
    let source = read_source(path)?;
    let program = parse(&source).map_err(|error| format!("{path}: {error}"))?;
    println!("{program:#?}");
    Ok(())
}

fn run_check(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_source_arg(program, "check", args)?;
    let source = read_source(path)?;
    let program_ast = parse(&source).map_err(|error| format!("{path}: {error}"))?;
    check(&program_ast).map_err(|error| format!("{path}: {error}"))?;
    println!("{path}: ok");
    Ok(())
}

fn run_ir(program: &str, args: &[String]) -> Result<(), String> {
    let path = single_source_arg(program, "ir", args)?;
    let source = read_source(path)?;
    let program_ast = parse(&source).map_err(|error| format!("{path}: {error}"))?;
    let checked = check(&program_ast).map_err(|error| format!("{path}: {error}"))?;
    let ir = lower(&checked).map_err(|error| format!("{path}: {error}"))?;
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
    let source = read_source(source_path)?;
    let program_ast = parse(&source).map_err(|error| format!("{source_path}: {error}"))?;
    check(&program_ast).map_err(|error| format!("{source_path}: {error}"))?;
    let c_source = generate_c(&program_ast).map_err(|error| format!("{source_path}: {error}"))?;

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

fn read_source(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{path}: failed to read source: {error}"))
}
