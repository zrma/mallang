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
    lower_sources, parse_sources, prepare_project_tests, CompilerError, FormatError, FrontendError,
    PackageTest, Project, SourceId, SourceMap,
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
        Some("test") => utf8_cli_args(&args[2..]).and_then(|args| run_test(&program, &args)),
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
    writeln!(output, "  {program} test <input> [--exact <test-id>]")?;
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
    let test_files = project
        .discover_test_files()
        .map_err(|error| error.to_string())?;
    project
        .source_files()
        .iter()
        .chain(test_files.iter())
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

fn run_test(program: &str, args: &[String]) -> Result<(), String> {
    let (input, exact) = match args {
        [input] => (input.as_str(), None),
        [input, flag, test_id] if flag == "--exact" => (input.as_str(), Some(test_id.as_str())),
        [_, flag, ..] if flag.starts_with('-') && flag != "--exact" => {
            return Err(format!("unknown test argument `{flag}`"));
        }
        _ => {
            return Err(format!("usage: {program} test <input> [--exact <test-id>]"));
        }
    };

    if Path::new(input)
        .extension()
        .is_some_and(|extension| extension == "mlg")
    {
        return Err("mlg test requires a project directory or `mallang.toml`".to_string());
    }
    let project = discover_project(input).map_err(|error| error.to_string())?;
    let test_files = project
        .discover_test_files()
        .map_err(|error| error.to_string())?;
    let project_sources = project.compilation_source_files();
    let loaded = load_source_files(project_sources.into_iter().chain(test_files.iter()))
        .map_err(|error| error.to_string())?;
    let suite = prepare_project_tests(&project, &loaded.sources, &loaded.source_ids)
        .map_err(|error| format_compiler_error(&loaded.sources, input, error))?;
    let selected = select_tests(suite.tests(), exact)?;
    let artifacts = build_test_artifacts(&project, &loaded.sources, &suite, &selected, input)?;

    let mut passed = 0usize;
    let mut failed = 0usize;
    for artifact in artifacts {
        let test = &suite.tests()[artifact.test_index];
        let output = Command::new(&artifact.binary_path)
            .output()
            .map_err(|error| format!("failed to execute test `{}`: {error}", test.id))?;
        if output.status.success() {
            println!("test {} ... ok", test.id);
            passed += 1;
            continue;
        }

        println!("test {} ... FAILED", test.id);
        io::stdout()
            .lock()
            .write_all(&output.stdout)
            .map_err(|error| format!("failed to replay test stdout: {error}"))?;
        let stderr = normalize_test_stderr(&project, &loaded.sources, test, &output.stderr);
        io::stderr()
            .lock()
            .write_all(stderr.as_bytes())
            .map_err(|error| format!("failed to replay test stderr: {error}"))?;
        if let Some(diagnostic) = child_signal_diagnostic(&test.id, &output.status, &output.stderr)
        {
            eprintln!("{diagnostic}");
        }
        failed += 1;
    }

    if failed == 0 {
        println!("test result: ok. {passed} passed; 0 failed");
        return Ok(());
    }

    println!("test result: FAILED. {passed} passed; {failed} failed");
    process::exit(1);
}

struct TestArtifact {
    test_index: usize,
    binary_path: PathBuf,
}

fn select_tests(tests: &[PackageTest], exact: Option<&str>) -> Result<Vec<usize>, String> {
    let Some(exact) = exact else {
        return Ok((0..tests.len()).collect());
    };
    tests
        .iter()
        .position(|test| test.id == exact)
        .map(|index| vec![index])
        .ok_or_else(|| format!("unknown test id `{exact}`"))
}

fn build_test_artifacts(
    project: &Project,
    sources: &SourceMap,
    suite: &mallang::ProjectTestSuite,
    selected: &[usize],
    fallback_path: &str,
) -> Result<Vec<TestArtifact>, String> {
    let build_dir = project.root().join("target/mallang/tests");
    fs::create_dir_all(&build_dir)
        .map_err(|error| format!("failed to create {}: {error}", build_dir.display()))?;
    let mut artifacts = Vec::with_capacity(selected.len());

    for test_index in selected.iter().copied() {
        let test = &suite.tests()[test_index];
        let c_source = suite
            .generate_c(test_index)
            .map_err(|error| format_compiler_error(sources, fallback_path, error))?;
        let stem = format!("test-{test_index:04}");
        let c_path = build_dir.join(format!("{stem}.c"));
        let binary_path = build_dir.join(stem);
        fs::write(&c_path, c_source)
            .map_err(|error| format!("failed to write test `{}` C source: {error}", test.id))?;
        let output = Command::new("clang")
            .arg(&c_path)
            .arg("-o")
            .arg(&binary_path)
            .output()
            .map_err(|error| format!("failed to execute clang for test `{}`: {error}", test.id))?;
        if !output.status.success() {
            return Err(format!(
                "native compilation failed for test `{}`:\n{}",
                test.id,
                String::from_utf8_lossy(&output.stderr).trim_end()
            ));
        }
        artifacts.push(TestArtifact {
            test_index,
            binary_path,
        });
    }

    Ok(artifacts)
}

fn normalize_test_stderr(
    project: &Project,
    sources: &SourceMap,
    test: &PackageTest,
    stderr: &[u8],
) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let mut normalized = String::new();
    for line in stderr.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        if let Some((source_id, offset)) = parse_assertion_marker(content) {
            let diagnostic = sources
                .file(SourceId::new(source_id))
                .and_then(|source| {
                    let location = source.location(offset)?;
                    let path = source
                        .path()
                        .strip_prefix(project.root())
                        .unwrap_or(source.path());
                    Some(format!(
                        "{}:{}:{}: assertion failed in test `{}`",
                        path.display(),
                        location.line,
                        location.column,
                        test.id
                    ))
                })
                .unwrap_or_else(|| format!("assertion failed in test `{}`", test.id));
            normalized.push_str(&diagnostic);
            normalized.push('\n');
        } else {
            normalized.push_str(line);
        }
    }
    normalized
}

fn parse_assertion_marker(line: &str) -> Option<(usize, usize)> {
    let marker = line.strip_prefix("__mlg_test_assert:")?;
    let (source_id, offset) = marker.split_once(':')?;
    Some((source_id.parse().ok()?, offset.parse().ok()?))
}

fn child_signal_diagnostic(
    test_id: &str,
    status: &process::ExitStatus,
    stderr: &[u8],
) -> Option<String> {
    (status.code().is_none() && stderr.is_empty())
        .then(|| format!("test {test_id} terminated by signal"))
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
            project
                .require_entrypoint()
                .map_err(|error| error.to_string())?;
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
        project: Box<Project>,
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
        load_source_files(project.compilation_source_files()).map_err(|error| error.to_string())?;
    Ok(CompilationInput::Project {
        project: Box::new(project),
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

#[cfg(test)]
mod tests {
    use super::{child_signal_diagnostic, parse_assertion_marker};

    #[test]
    fn parses_test_assertion_marker() {
        assert_eq!(
            parse_assertion_marker("__mlg_test_assert:3:42"),
            Some((3, 42))
        );
        assert_eq!(parse_assertion_marker("ordinary stderr"), None);
    }

    #[cfg(unix)]
    #[test]
    fn reports_signal_only_when_the_child_has_no_stderr() {
        use std::{os::unix::process::ExitStatusExt, process::ExitStatus};

        let signal_status = ExitStatus::from_raw(9);
        assert_eq!(
            child_signal_diagnostic("project::Test", &signal_status, b""),
            Some("test project::Test terminated by signal".to_string())
        );
        assert_eq!(
            child_signal_diagnostic("project::Test", &signal_status, b"runtime detail\n"),
            None
        );
    }
}
