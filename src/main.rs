use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self, Command},
};

use mallang::{
    check_project_sources_with_diagnostics, check_sources_with_diagnostics, discover_project,
    format_source, generate_c_project_sources_with_diagnostics,
    generate_c_sources_with_diagnostics, lex_with_source, load_source_files,
    lower_sources_with_diagnostics, parse_sources_with_diagnostics,
    prepare_project_tests_with_diagnostics, CompilerError, Diagnostic, DiagnosticStage,
    FormatError, FrontendError, PackageTest, Project, SourceId, SourceLoadError, SourceMap,
};

const SELF_COMPILER_BINARY: &str = "mlgc";
const SELF_COMPILER_PROTOCOL: &str = "mlgc protocol 1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticFormat {
    Human,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompilerImplementation {
    Stage0,
    SelfHosted,
}

impl CompilerImplementation {
    fn label(self) -> &'static str {
        match self {
            Self::Stage0 => "stage0",
            Self::SelfHosted => "self",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompilerSelection {
    implementation: CompilerImplementation,
    self_compiler: Option<PathBuf>,
}

impl Default for CompilerSelection {
    fn default() -> Self {
        Self {
            implementation: CompilerImplementation::Stage0,
            self_compiler: None,
        }
    }
}

impl CompilerSelection {
    fn self_compiler_path(&self) -> CliResult<PathBuf> {
        let path = match &self.self_compiler {
            Some(path) => path.clone(),
            None => env::current_exe()
                .map_err(|error| {
                    CliError::cli(format!("failed to locate the mlg driver: {error}"))
                })?
                .parent()
                .map(|directory| directory.join(SELF_COMPILER_BINARY))
                .ok_or_else(|| CliError::cli("mlg executable has no parent directory"))?,
        };
        if !path.is_file() {
            return Err(CliError::cli(format!(
                "self-hosted compiler not found at {}; build or install `{SELF_COMPILER_BINARY}` or pass `--self-compiler <path>`",
                path.display()
            )));
        }
        Ok(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlobalOptions {
    diagnostic_format: DiagnosticFormat,
    compiler: CompilerSelection,
    command_index: usize,
}

#[derive(Debug)]
struct CliError {
    diagnostics: Vec<Diagnostic>,
}

impl CliError {
    fn one(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    fn many(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    fn cli(message: impl Into<String>) -> Self {
        Self::one(Diagnostic::error(DiagnosticStage::Cli, message))
    }

    fn input(message: impl Into<String>) -> Self {
        Self::one(Diagnostic::error(DiagnosticStage::Input, message))
    }

    fn native(message: impl Into<String>) -> Self {
        Self::one(Diagnostic::error(DiagnosticStage::Native, message))
    }
}

impl From<String> for CliError {
    fn from(message: String) -> Self {
        Self::cli(message)
    }
}

impl From<Diagnostic> for CliError {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::one(diagnostic)
    }
}

type CliResult<T> = Result<T, CliError>;

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

    let options = match parse_global_options(&args) {
        Ok(parsed) => parsed,
        Err(error) => {
            emit_diagnostics(error, DiagnosticFormat::Human);
            process::exit(1);
        }
    };
    let diagnostic_format = options.diagnostic_format;
    let Some(command) = args.get(options.command_index) else {
        emit_diagnostics(
            CliError::cli(format!(
                "missing subcommand; run `{program} --help` for usage"
            )),
            diagnostic_format,
        );
        process::exit(1);
    };
    let command_args = &args[options.command_index + 1..];

    let result = match command.to_str() {
        Some("lex") => require_stage0(&options.compiler, "lex")
            .and_then(|()| utf8_cli_args(command_args))
            .and_then(|args| run_lex(&program, &args)),
        Some("parse") => require_stage0(&options.compiler, "parse")
            .and_then(|()| utf8_cli_args(command_args))
            .and_then(|args| run_parse(&program, &args)),
        Some("check") => utf8_cli_args(command_args)
            .and_then(|args| run_check(&program, &args, &options.compiler)),
        Some("fmt") => require_stage0(&options.compiler, "fmt")
            .and_then(|()| utf8_cli_args(command_args))
            .and_then(|args| run_fmt(&program, &args)),
        Some("ir") => require_stage0(&options.compiler, "ir")
            .and_then(|()| utf8_cli_args(command_args))
            .and_then(|args| run_ir(&program, &args)),
        Some("build") => utf8_cli_args(command_args)
            .and_then(|args| run_build(&program, &args, &options.compiler)),
        Some("run") => run_run(&program, command_args, &options.compiler),
        Some("test") => require_stage0(&options.compiler, "test")
            .and_then(|()| utf8_cli_args(command_args))
            .and_then(|args| run_test(&program, &args, diagnostic_format)),
        Some("-V" | "--version") => run_version(&program, command_args, &options.compiler),
        Some("-h" | "--help") => {
            let mut stdout = io::stdout().lock();
            write_usage(&mut stdout, &program)
                .map_err(|error| CliError::cli(format!("failed to write usage: {error}")))
        }
        Some(command) => Err(CliError::cli(format!(
            "unknown subcommand `{command}`; run `{program} --help` for usage"
        ))),
        None => Err(CliError::cli("subcommand is not valid UTF-8")),
    };

    if let Err(error) = result {
        emit_diagnostics(error, diagnostic_format);
        process::exit(1);
    }
}

fn parse_global_options(args: &[OsString]) -> CliResult<GlobalOptions> {
    let mut diagnostic_format = DiagnosticFormat::Human;
    let mut diagnostic_format_seen = false;
    let mut compiler = CompilerSelection::default();
    let mut compiler_seen = false;
    let mut self_compiler_seen = false;
    let mut index = 1;

    while let Some(argument) = args.get(index).and_then(|argument| argument.to_str()) {
        if argument == "--diagnostic-format" {
            if diagnostic_format_seen {
                return Err(CliError::cli("duplicate --diagnostic-format option"));
            }
            let value = args
                .get(index + 1)
                .and_then(|value| value.to_str())
                .ok_or_else(|| CliError::cli("missing UTF-8 value for --diagnostic-format"))?;
            diagnostic_format = parse_diagnostic_format_value(value)?;
            diagnostic_format_seen = true;
            index += 2;
            continue;
        }
        if let Some(value) = argument.strip_prefix("--diagnostic-format=") {
            if diagnostic_format_seen {
                return Err(CliError::cli("duplicate --diagnostic-format option"));
            }
            diagnostic_format = parse_diagnostic_format_value(value)?;
            diagnostic_format_seen = true;
            index += 1;
            continue;
        }
        if argument == "--compiler" {
            if compiler_seen {
                return Err(CliError::cli("duplicate --compiler option"));
            }
            let value = args
                .get(index + 1)
                .and_then(|value| value.to_str())
                .ok_or_else(|| CliError::cli("missing UTF-8 value for --compiler"))?;
            compiler.implementation = parse_compiler_implementation(value)?;
            compiler_seen = true;
            index += 2;
            continue;
        }
        if let Some(value) = argument.strip_prefix("--compiler=") {
            if compiler_seen {
                return Err(CliError::cli("duplicate --compiler option"));
            }
            compiler.implementation = parse_compiler_implementation(value)?;
            compiler_seen = true;
            index += 1;
            continue;
        }
        if argument == "--self-compiler" {
            if self_compiler_seen {
                return Err(CliError::cli("duplicate --self-compiler option"));
            }
            let path = args
                .get(index + 1)
                .filter(|path| {
                    !path.is_empty() && path.to_str().is_none_or(|value| !value.starts_with("--"))
                })
                .ok_or_else(|| CliError::cli("missing value for --self-compiler"))?;
            compiler.self_compiler = Some(PathBuf::from(path));
            self_compiler_seen = true;
            index += 2;
            continue;
        }
        break;
    }

    if self_compiler_seen && compiler.implementation != CompilerImplementation::SelfHosted {
        return Err(CliError::cli(
            "--self-compiler requires explicit `--compiler self`",
        ));
    }

    Ok(GlobalOptions {
        diagnostic_format,
        compiler,
        command_index: index,
    })
}

fn parse_diagnostic_format_value(value: &str) -> CliResult<DiagnosticFormat> {
    match value {
        "human" => Ok(DiagnosticFormat::Human),
        "json" => Ok(DiagnosticFormat::Json),
        _ => Err(CliError::cli(format!(
            "unknown diagnostic format `{value}`; expected `human` or `json`"
        ))),
    }
}

fn parse_compiler_implementation(value: &str) -> CliResult<CompilerImplementation> {
    match value {
        "stage0" => Ok(CompilerImplementation::Stage0),
        "self" => Ok(CompilerImplementation::SelfHosted),
        _ => Err(CliError::cli(format!(
            "unknown compiler `{value}`; expected `stage0` or `self`"
        ))),
    }
}

fn require_stage0(selection: &CompilerSelection, command: &str) -> CliResult<()> {
    if selection.implementation == CompilerImplementation::Stage0 {
        return Ok(());
    }
    Err(CliError::cli(format!(
        "self-hosted compiler does not yet support public `{command}`; use explicit `--compiler stage0`"
    )))
}

fn emit_diagnostics(error: CliError, format: DiagnosticFormat) {
    for diagnostic in error.diagnostics {
        match format {
            DiagnosticFormat::Human => eprintln!("{}", diagnostic.render_human()),
            DiagnosticFormat::Json => eprintln!("{}", diagnostic.render_json()),
        }
    }
}

fn utf8_cli_args(args: &[OsString]) -> CliResult<Vec<String>> {
    args.iter()
        .cloned()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| CliError::cli("command argument is not valid UTF-8"))
        })
        .collect()
}

fn run_version(program: &str, args: &[OsString], compiler: &CompilerSelection) -> CliResult<()> {
    let verbose = match args {
        [] => false,
        [flag] if flag == OsStr::new("--verbose") => true,
        _ => {
            return Err(CliError::cli(format!(
                "usage: {program} [--compiler <stage0|self>] --version [--verbose]"
            )))
        }
    };
    let core = match compiler.implementation {
        CompilerImplementation::Stage0 => "rust-stage0".to_string(),
        CompilerImplementation::SelfHosted => self_compiler_version(compiler)?,
    };

    println!("mlg {}", env!("CARGO_PKG_VERSION"));
    if verbose {
        println!("driver: rust");
        println!("compiler: {}", compiler.implementation.label());
        println!("core: {core}");
    }
    Ok(())
}

fn self_compiler_version(compiler: &CompilerSelection) -> CliResult<String> {
    let path = compiler.self_compiler_path()?;
    let output = Command::new(&path)
        .arg("--version")
        .output()
        .map_err(|error| {
            CliError::cli(format!(
                "failed to execute self-hosted compiler {}: {error}",
                path.display()
            ))
        })?;
    let stdout = String::from_utf8(output.stdout)
        .map_err(|_| CliError::cli("self-hosted compiler version is not valid UTF-8"))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || stdout.trim_end() != SELF_COMPILER_PROTOCOL || !stderr.is_empty()
    {
        return Err(CliError::cli(format!(
            "self-hosted compiler protocol mismatch at {}; expected `{SELF_COMPILER_PROTOCOL}`",
            path.display()
        )));
    }
    Ok(SELF_COMPILER_PROTOCOL.to_string())
}

fn write_usage(output: &mut impl Write, program: &str) -> io::Result<()> {
    writeln!(output, "usage:")?;
    writeln!(
        output,
        "  {program} [--diagnostic-format <human|json>] [--compiler <stage0|self>] [--self-compiler <path>] <subcommand> ..."
    )?;
    writeln!(output, "  {program} lex <source-file>")?;
    writeln!(output, "  {program} parse <source-file>")?;
    writeln!(output, "  {program} check <input>")?;
    writeln!(output, "  {program} fmt [--check] <input>")?;
    writeln!(output, "  {program} ir <source-file>")?;
    writeln!(output, "  {program} build <input> [-o <output>]")?;
    writeln!(output, "  {program} run <input> [-- <program-args>...]")?;
    writeln!(output, "  {program} test <input> [--exact <test-id>]")?;
    writeln!(output, "  {program} --version [--verbose]")
}

fn run_lex(program: &str, args: &[String]) -> CliResult<()> {
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
        Err(error) => Err(source_diagnostic(
            DiagnosticStage::Frontend,
            error.message,
            &sources,
            error.span,
            None,
        )
        .into()),
    }
}

fn run_parse(program: &str, args: &[String]) -> CliResult<()> {
    let path = single_source_arg(program, "parse", args)?;
    let (sources, source_id) = load_source(path)?;
    let program = parse_loaded_source(&sources, source_id)?;
    println!("{program:#?}");
    Ok(())
}

fn run_check(program: &str, args: &[String], compiler: &CompilerSelection) -> CliResult<()> {
    let path = single_input_arg(program, "check", args)?;
    if compiler.implementation == CompilerImplementation::SelfHosted {
        match load_compilation_input(path)? {
            CompilationInput::Standalone { sources, .. } => {
                let stdout = invoke_self_hosted_compiler("check", path, compiler)?;
                finish_self_hosted_check(path, stdout, &sources, None)?;
            }
            CompilationInput::Project {
                project,
                sources,
                source_ids,
            } => {
                let stdout = invoke_self_hosted_project_compiler(
                    "check-project",
                    &project,
                    &sources,
                    &source_ids,
                    path,
                    compiler,
                )?;
                finish_self_hosted_check(path, stdout, &sources, Some(&project))?;
            }
        }
        println!("{path}: ok");
        return Ok(());
    }

    match load_compilation_input(path)? {
        CompilationInput::Standalone { sources, source_id } => {
            check_sources_with_diagnostics(&sources, &[source_id])
                .map_err(|errors| compiler_diagnostics(&sources, None, path, errors))?;
        }
        CompilationInput::Project {
            project,
            sources,
            source_ids,
        } => {
            check_project_sources_with_diagnostics(&project, &sources, &source_ids)
                .map_err(|errors| compiler_diagnostics(&sources, Some(&project), path, errors))?;
        }
    }
    println!("{path}: ok");
    Ok(())
}

fn run_fmt(program: &str, args: &[String]) -> CliResult<()> {
    let (check_only, input) = match args {
        [input] if input != "--check" => (false, input.as_str()),
        [flag, input] if flag == "--check" => (true, input.as_str()),
        [flag, ..] if flag.starts_with('-') && flag != "--check" => {
            return Err(CliError::cli(format!("unknown fmt argument `{flag}`")));
        }
        _ => {
            return Err(CliError::cli(format!(
                "usage: {program} fmt [--check] <input>"
            )))
        }
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
        return Err(CliError::many(
            changes
                .iter()
                .map(|(_, display_path, _)| {
                    Diagnostic::error(DiagnosticStage::Input, "not formatted")
                        .with_path(display_path)
                })
                .collect(),
        ));
    }

    for (path, display_path, formatted) in changes {
        fs::write(&path, formatted).map_err(|error| {
            CliError::one(
                Diagnostic::error(
                    DiagnosticStage::Input,
                    format!("failed to write source: {error}"),
                )
                .with_path(&display_path),
            )
        })?;
        println!("{}: formatted", display_path.display());
    }
    Ok(())
}

struct FormatInput {
    path: PathBuf,
    display_path: PathBuf,
    source: String,
}

fn load_format_inputs(input: &str) -> CliResult<Vec<FormatInput>> {
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

    let project =
        discover_project(input_path).map_err(|error| CliError::input(error.to_string()))?;
    let test_files = project
        .discover_test_files()
        .map_err(|error| CliError::input(error.to_string()))?;
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

fn load_format_input(path: PathBuf, display_path: PathBuf) -> CliResult<FormatInput> {
    let source = fs::read_to_string(&path).map_err(|error| {
        CliError::one(
            Diagnostic::error(
                DiagnosticStage::Input,
                format!("failed to read source: {error}"),
            )
            .with_path(&display_path),
        )
    })?;
    Ok(FormatInput {
        path,
        display_path,
        source,
    })
}

fn format_format_error(display_path: &Path, source: &str, error: FormatError) -> Diagnostic {
    let mut sources = SourceMap::new();
    let source_id = sources.add_file(display_path, source);
    source_diagnostic(
        DiagnosticStage::Frontend,
        error.message,
        &sources,
        mallang::Span::new(source_id, error.span.start, error.span.end),
        Some(display_path),
    )
}

fn run_ir(program: &str, args: &[String]) -> CliResult<()> {
    let path = single_source_arg(program, "ir", args)?;
    let (sources, source_id) = load_source(path)?;
    let ir = lower_sources_with_diagnostics(&sources, &[source_id])
        .map_err(|errors| compiler_diagnostics(&sources, None, path, errors))?;
    println!("{ir:#?}");
    Ok(())
}

fn run_build(program: &str, args: &[String], compiler: &CompilerSelection) -> CliResult<()> {
    if args.is_empty() {
        return Err(CliError::cli(format!(
            "usage: {program} build <input> [-o <output>]"
        )));
    }

    let source_path = &args[0];
    let mut output_path = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                let Some(path) = args.get(index + 1) else {
                    return Err(CliError::cli("missing value for -o/--output"));
                };
                output_path = Some(PathBuf::from(path));
                index += 2;
            }
            arg => return Err(CliError::cli(format!("unknown build argument `{arg}`"))),
        }
    }

    let output_path = compile_input(source_path, output_path, OutputKind::Build, compiler)?;
    println!("{}", output_path.display());
    Ok(())
}

fn run_run(program: &str, args: &[OsString], compiler: &CompilerSelection) -> CliResult<()> {
    let Some(source_path) = args.first() else {
        return Err(CliError::cli(format!(
            "usage: {program} run <input> [-- <program-args>...]"
        )));
    };
    let source_path = source_path
        .to_str()
        .ok_or_else(|| CliError::cli("run input path is not valid UTF-8"))?;
    let program_args = match args.get(1) {
        None => &args[1..],
        Some(argument) if argument == OsStr::new("--") => &args[2..],
        Some(argument) => {
            return Err(CliError::cli(format!(
                "unknown run argument `{}`; program arguments must follow `--`",
                argument.to_string_lossy()
            )));
        }
    };
    let binary_path = compile_input(source_path, None, OutputKind::Run, compiler)?;

    let status = Command::new(&binary_path)
        .args(program_args)
        .status()
        .map_err(|error| {
            CliError::native(format!(
                "failed to execute {}: {error}",
                binary_path.display()
            ))
        })?;
    if !status.success() {
        if let Some(code) = status.code() {
            process::exit(code);
        }
        return Err(CliError::native(format!(
            "program terminated by signal: {status}"
        )));
    }

    Ok(())
}

fn run_test(program: &str, args: &[String], diagnostic_format: DiagnosticFormat) -> CliResult<()> {
    let (input, exact) = match args {
        [input] => (input.as_str(), None),
        [input, flag, test_id] if flag == "--exact" => (input.as_str(), Some(test_id.as_str())),
        [_, flag, ..] if flag.starts_with('-') && flag != "--exact" => {
            return Err(CliError::cli(format!("unknown test argument `{flag}`")));
        }
        _ => {
            return Err(CliError::cli(format!(
                "usage: {program} test <input> [--exact <test-id>]"
            )));
        }
    };

    if Path::new(input)
        .extension()
        .is_some_and(|extension| extension == "mlg")
    {
        return Err(CliError::cli(
            "mlg test requires a project directory or `mallang.toml`",
        ));
    }
    let project = discover_project(input).map_err(|error| CliError::input(error.to_string()))?;
    let test_files = project
        .discover_test_files()
        .map_err(|error| CliError::input(error.to_string()))?;
    let project_sources = project.compilation_source_files();
    let loaded = load_source_files(project_sources.into_iter().chain(test_files.iter()))
        .map_err(|error| project_source_load_error(&project, error))?;
    let suite =
        prepare_project_tests_with_diagnostics(&project, &loaded.sources, &loaded.source_ids)
            .map_err(|errors| {
                compiler_diagnostics(&loaded.sources, Some(&project), input, errors)
            })?;
    let selected = select_tests(suite.tests(), exact)?;
    let artifacts = build_test_artifacts(&project, &loaded.sources, &suite, &selected, input)?;

    let mut passed = 0usize;
    let mut failed = 0usize;
    for artifact in artifacts {
        let test = &suite.tests()[artifact.test_index];
        let output = Command::new(&artifact.binary_path)
            .arg(artifact.runner_case.to_string())
            .output()
            .map_err(|error| {
                CliError::native(format!("failed to execute test `{}`: {error}", test.id))
            })?;
        if output.status.success() {
            println!("test {} ... ok", test.id);
            passed += 1;
            continue;
        }

        println!("test {} ... FAILED", test.id);
        io::stdout()
            .lock()
            .write_all(&output.stdout)
            .map_err(|error| CliError::native(format!("failed to replay test stdout: {error}")))?;
        replay_test_stderr(
            &project,
            &loaded.sources,
            test,
            &output.stderr,
            diagnostic_format,
        )?;
        if let Some(diagnostic) = child_signal_diagnostic(&test.id, &output.status, &output.stderr)
        {
            emit_diagnostics(CliError::one(diagnostic), diagnostic_format);
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
    runner_case: usize,
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
) -> CliResult<Vec<TestArtifact>> {
    let build_dir = project.root().join("target/mallang/tests");
    if build_dir.exists() {
        fs::remove_dir_all(&build_dir).map_err(|error| {
            CliError::native(format!("failed to clean {}: {error}", build_dir.display()))
        })?;
    }
    fs::create_dir_all(&build_dir).map_err(|error| {
        CliError::native(format!("failed to create {}: {error}", build_dir.display()))
    })?;
    if selected.is_empty() {
        return Ok(Vec::new());
    }

    let c_source = suite
        .generate_c_runner(selected)
        .map_err(|error| compiler_diagnostic(sources, Some(project), fallback_path, error))?;
    let c_path = build_dir.join("runner.c");
    let binary_path = build_dir.join("runner");
    fs::write(&c_path, c_source).map_err(|error| {
        CliError::native(format!(
            "failed to write native test runner C source: {error}"
        ))
    })?;
    let output = Command::new("clang")
        .arg(&c_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .map_err(|error| {
            CliError::native(format!("failed to execute clang for test runner: {error}"))
        })?;
    if !output.status.success() {
        return Err(CliError::native(format!(
            "native test runner compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        )));
    }

    Ok(selected
        .iter()
        .copied()
        .enumerate()
        .map(|(runner_case, test_index)| TestArtifact {
            test_index,
            runner_case,
            binary_path: binary_path.clone(),
        })
        .collect())
}

fn replay_test_stderr(
    project: &Project,
    sources: &SourceMap,
    test: &PackageTest,
    stderr: &[u8],
    diagnostic_format: DiagnosticFormat,
) -> CliResult<()> {
    let stderr = String::from_utf8_lossy(stderr);
    for line in stderr.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        if let Some((source_id, offset)) = parse_assertion_marker(content) {
            let diagnostic = sources
                .file(SourceId::new(source_id))
                .map(|source| {
                    let path = project.diagnostic_path(source.path());
                    Diagnostic::error(
                        DiagnosticStage::Native,
                        format!("assertion failed in test `{}`", test.id),
                    )
                    .with_span(
                        sources,
                        mallang::Span::new(SourceId::new(source_id), offset, offset),
                        Some(&path),
                    )
                })
                .unwrap_or_else(|| {
                    Diagnostic::error(
                        DiagnosticStage::Native,
                        format!("assertion failed in test `{}`", test.id),
                    )
                });
            emit_diagnostics(CliError::one(diagnostic), diagnostic_format);
        } else {
            io::stderr()
                .lock()
                .write_all(line.as_bytes())
                .map_err(|error| {
                    CliError::native(format!("failed to replay test stderr: {error}"))
                })?;
        }
    }
    Ok(())
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
) -> Option<Diagnostic> {
    (status.code().is_none() && stderr.is_empty()).then(|| {
        Diagnostic::error(
            DiagnosticStage::Native,
            format!("test {test_id} terminated by signal"),
        )
    })
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
    compiler: &CompilerSelection,
) -> CliResult<PathBuf> {
    let (c_source, artifact_name, build_dir) = if compiler.implementation
        == CompilerImplementation::SelfHosted
    {
        match load_compilation_input(input_path)? {
            CompilationInput::Standalone { sources, .. } => (
                generate_self_hosted_c(input_path, &sources, compiler)?,
                source_stem(input_path).to_string(),
                PathBuf::from("target/mallang"),
            ),
            CompilationInput::Project {
                project,
                sources,
                source_ids,
            } => {
                project
                    .require_entrypoint()
                    .map_err(|error| CliError::input(error.to_string()))?;
                let c_source = generate_self_hosted_project_c(
                    &project,
                    &sources,
                    &source_ids,
                    input_path,
                    compiler,
                )?;
                let artifact_name = project.name().to_string();
                let build_dir = project.root().join("target/mallang");
                (c_source, artifact_name, build_dir)
            }
        }
    } else {
        match load_compilation_input(input_path)? {
            CompilationInput::Standalone { sources, source_id } => {
                let c_source = generate_c_sources_with_diagnostics(&sources, &[source_id])
                    .map_err(|errors| compiler_diagnostics(&sources, None, input_path, errors))?;
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
                    .map_err(|error| CliError::input(error.to_string()))?;
                let c_source =
                    generate_c_project_sources_with_diagnostics(&project, &sources, &source_ids)
                        .map_err(|errors| {
                            compiler_diagnostics(&sources, Some(&project), input_path, errors)
                        })?;
                let artifact_name = project.name().to_string();
                let build_dir = project.root().join("target/mallang");
                (c_source, artifact_name, build_dir)
            }
        }
    };

    fs::create_dir_all(&build_dir).map_err(|error| {
        CliError::native(format!("failed to create {}: {error}", build_dir.display()))
    })?;
    let c_path = build_dir.join(format!("{artifact_name}.c"));
    fs::write(&c_path, c_source).map_err(|error| {
        CliError::native(format!("failed to write {}: {error}", c_path.display()))
    })?;

    let default_output_dir = match kind {
        OutputKind::Build => build_dir.clone(),
        OutputKind::Run => build_dir.join("run"),
    };
    let output_path = output_path.unwrap_or_else(|| default_output_dir.join(&artifact_name));
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            CliError::native(format!("failed to create {}: {error}", parent.display()))
        })?;
    }
    let output = Command::new("clang")
        .arg(&c_path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .map_err(|error| CliError::native(format!("failed to execute clang: {error}")))?;
    if !output.status.success() {
        return Err(CliError::native(format!(
            "native compilation failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        )));
    }

    Ok(output_path)
}

fn generate_self_hosted_c(
    input_path: &str,
    sources: &SourceMap,
    compiler: &CompilerSelection,
) -> CliResult<String> {
    let stdout = invoke_self_hosted_compiler("c", input_path, compiler)?;
    if stdout.starts_with("#include <") {
        return Ok(stdout);
    }
    match parse_self_hosted_diagnostics(&stdout, sources, None) {
        Ok(diagnostics) => Err(CliError::many(diagnostics)),
        Err(detail) => Err(self_hosted_protocol_error(input_path, detail)),
    }
}

fn generate_self_hosted_project_c(
    project: &Project,
    sources: &SourceMap,
    source_ids: &[SourceId],
    input_path: &str,
    compiler: &CompilerSelection,
) -> CliResult<String> {
    let stdout = invoke_self_hosted_project_compiler(
        "c-project",
        project,
        sources,
        source_ids,
        input_path,
        compiler,
    )?;
    if stdout.starts_with("#include <") {
        return Ok(stdout);
    }
    match parse_self_hosted_diagnostics(&stdout, sources, Some(project)) {
        Ok(diagnostics) => Err(CliError::many(diagnostics)),
        Err(detail) => Err(self_hosted_protocol_error(input_path, detail)),
    }
}

fn invoke_self_hosted_compiler(
    command: &str,
    input_path: &str,
    compiler: &CompilerSelection,
) -> CliResult<String> {
    invoke_self_hosted_compiler_args(
        &[OsString::from(command), OsString::from(input_path)],
        input_path,
        compiler,
    )
}

fn invoke_self_hosted_project_compiler(
    command: &str,
    project: &Project,
    sources: &SourceMap,
    source_ids: &[SourceId],
    input_path: &str,
    compiler: &CompilerSelection,
) -> CliResult<String> {
    let units = project.compiler_units().collect::<Vec<_>>();
    let mut args = vec![
        OsString::from(command),
        OsString::from(units.len().to_string()),
    ];
    for unit in units {
        let dependencies = unit.direct_dependencies().collect::<Vec<_>>();
        args.push(OsString::from(unit.name()));
        args.push(unit.source_root().as_os_str().to_owned());
        args.push(OsString::from(dependencies.len().to_string()));
        args.extend(dependencies.into_iter().map(OsString::from));
    }
    for source_id in source_ids {
        let source = sources.file(*source_id).ok_or_else(|| {
            self_hosted_protocol_error(
                input_path,
                format!(
                    "source ID {} is missing from the project map",
                    source_id.index()
                ),
            )
        })?;
        args.push(source.path().as_os_str().to_owned());
    }
    invoke_self_hosted_compiler_args(&args, input_path, compiler)
}

fn invoke_self_hosted_compiler_args(
    args: &[OsString],
    input_path: &str,
    compiler: &CompilerSelection,
) -> CliResult<String> {
    let compiler_path = compiler.self_compiler_path()?;
    let output = Command::new(&compiler_path)
        .args(args)
        .output()
        .map_err(|error| {
            CliError::native(format!(
                "failed to execute self-hosted compiler {}: {error}",
                compiler_path.display()
            ))
        })?;
    let stdout = String::from_utf8(output.stdout).map_err(|_| {
        CliError::one(
            Diagnostic::error(
                DiagnosticStage::Backend,
                "self-hosted compiler output is not valid UTF-8",
            )
            .with_path(input_path),
        )
    })?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || !stderr.is_empty() {
        let detail = if !stderr.trim().is_empty() {
            stderr.trim()
        } else if !stdout.trim().is_empty() {
            stdout.trim()
        } else {
            "self-hosted compiler produced no output"
        };
        return Err(CliError::one(
            Diagnostic::error(
                DiagnosticStage::Backend,
                format!("self-hosted compiler failed: {detail}"),
            )
            .with_path(input_path),
        ));
    }
    Ok(stdout)
}

fn finish_self_hosted_check(
    input_path: &str,
    stdout: String,
    sources: &SourceMap,
    project: Option<&Project>,
) -> CliResult<()> {
    let Some(first_line) = stdout.lines().next() else {
        return Err(self_hosted_protocol_error(
            input_path,
            "check response is empty",
        ));
    };
    if first_line.starts_with("CHECKED|") {
        validate_self_hosted_check_header(first_line)
            .map_err(|detail| self_hosted_protocol_error(input_path, detail))?;
        return Ok(());
    }
    Err(CliError::many(
        parse_self_hosted_diagnostics(&stdout, sources, project)
            .map_err(|detail| self_hosted_protocol_error(input_path, detail))?,
    ))
}

fn validate_self_hosted_check_header(line: &str) -> Result<(), String> {
    let fields: Vec<_> = line.split('|').collect();
    if fields.len() != 5 || fields[0] != "CHECKED" {
        return Err("invalid CHECKED header".to_string());
    }
    for field in &fields[1..] {
        field
            .parse::<usize>()
            .map_err(|_| "invalid CHECKED count".to_string())?;
    }
    Ok(())
}

fn parse_self_hosted_diagnostics(
    output: &str,
    sources: &SourceMap,
    project: Option<&Project>,
) -> Result<Vec<Diagnostic>, String> {
    let mut diagnostics = Vec::new();
    for line in output.lines() {
        if line.is_empty() {
            continue;
        }
        diagnostics.push(parse_self_hosted_diagnostic(line, sources, project)?);
    }
    if diagnostics.is_empty() {
        return Err("diagnostic response is empty".to_string());
    }
    Ok(diagnostics)
}

fn parse_self_hosted_diagnostic(
    line: &str,
    sources: &SourceMap,
    project: Option<&Project>,
) -> Result<Diagnostic, String> {
    let fields: Vec<_> = line.split('|').collect();
    if fields.len() != 5 {
        return Err("diagnostic record has an invalid field count".to_string());
    }
    let stage = match fields[0] {
        "PERR" => DiagnosticStage::Frontend,
        "KERR" => DiagnosticStage::Package,
        "LERR" => DiagnosticStage::Link,
        "SERR" => DiagnosticStage::Semantic,
        "IERR" => DiagnosticStage::Ir,
        prefix => return Err(format!("unknown diagnostic prefix `{prefix}`")),
    };
    let source_index = parse_self_hosted_usize(fields[1], "source ID")?;
    let start = parse_self_hosted_usize(fields[2], "span start")?;
    let end = parse_self_hosted_usize(fields[3], "span end")?;
    let source_id = SourceId::new(source_index);
    let source = sources
        .file(source_id)
        .ok_or_else(|| format!("unknown source ID {source_index}"))?;
    if start > end
        || end > source.text().len()
        || !source.text().is_char_boundary(start)
        || !source.text().is_char_boundary(end)
    {
        return Err(format!("invalid source span {start}..{end}"));
    }
    let message = decode_self_hosted_bytes(fields[4])?;
    let display_path = project.map(|project| project.diagnostic_path(source.path()));
    Ok(source_diagnostic(
        stage,
        message,
        sources,
        mallang::Span::new(source_id, start, end),
        display_path.as_deref(),
    ))
}

fn parse_self_hosted_usize(value: &str, label: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("invalid diagnostic {label}"))
}

fn decode_self_hosted_bytes(encoded: &str) -> Result<String, String> {
    let bytes = if encoded.is_empty() {
        Vec::new()
    } else {
        encoded
            .split(',')
            .map(|value| {
                value
                    .parse::<u8>()
                    .map_err(|_| "invalid diagnostic message byte".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    String::from_utf8(bytes).map_err(|_| "diagnostic message is not valid UTF-8".to_string())
}

fn self_hosted_protocol_error(path: &str, detail: impl Into<String>) -> CliError {
    CliError::one(
        Diagnostic::error(
            DiagnosticStage::Backend,
            format!("invalid self-hosted compiler response: {}", detail.into()),
        )
        .with_path(path),
    )
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
) -> CliResult<&'a str> {
    if args.len() != 1 {
        return Err(CliError::cli(format!(
            "usage: {program} {subcommand} <source-file>"
        )));
    }
    Ok(&args[0])
}

fn single_input_arg<'a>(program: &str, subcommand: &str, args: &'a [String]) -> CliResult<&'a str> {
    if args.len() != 1 {
        return Err(CliError::cli(format!(
            "usage: {program} {subcommand} <input>"
        )));
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

fn load_compilation_input(path: &str) -> CliResult<CompilationInput> {
    let input = Path::new(path);
    if input
        .extension()
        .is_some_and(|extension| extension == "mlg")
    {
        let (sources, source_id) = load_source(path)?;
        return Ok(CompilationInput::Standalone { sources, source_id });
    }

    let project = discover_project(input).map_err(|error| CliError::input(error.to_string()))?;
    let loaded = load_source_files(project.compilation_source_files())
        .map_err(|error| project_source_load_error(&project, error))?;
    Ok(CompilationInput::Project {
        project: Box::new(project),
        sources: loaded.sources,
        source_ids: loaded.source_ids,
    })
}

fn load_source(path: &str) -> CliResult<(SourceMap, SourceId)> {
    let loaded = load_source_files([path]).map_err(source_load_error)?;
    let source_id = loaded
        .source_ids
        .first()
        .copied()
        .ok_or_else(|| CliError::input("source loader returned no source IDs"))?;
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
) -> CliResult<mallang::ast::Program> {
    parse_sources_with_diagnostics(sources, &[source_id])
        .map_err(|errors| frontend_diagnostics(sources, errors))
}

fn frontend_diagnostics(sources: &SourceMap, errors: Vec<FrontendError>) -> CliError {
    CliError::many(
        errors
            .into_iter()
            .map(|error| frontend_diagnostic(sources, error))
            .collect(),
    )
}

fn frontend_diagnostic(sources: &SourceMap, error: FrontendError) -> Diagnostic {
    match error.span {
        Some(span) => source_diagnostic(
            DiagnosticStage::Frontend,
            error.message,
            sources,
            span,
            None,
        ),
        None => Diagnostic::error(DiagnosticStage::Frontend, error.message),
    }
}

fn compiler_diagnostic(
    sources: &SourceMap,
    project: Option<&Project>,
    fallback_path: &str,
    error: CompilerError,
) -> Diagnostic {
    match error.span {
        Some(span) => {
            let display_path = sources.file(span.source).map(|source| {
                project
                    .map(|project| project.diagnostic_path(source.path()))
                    .unwrap_or_else(|| source.path().to_path_buf())
            });
            source_diagnostic(
                error.stage.into(),
                error.message,
                sources,
                span,
                display_path.as_deref(),
            )
        }
        None => Diagnostic::error(error.stage.into(), error.message).with_path(fallback_path),
    }
}

fn compiler_diagnostics(
    sources: &SourceMap,
    project: Option<&Project>,
    fallback_path: &str,
    errors: Vec<CompilerError>,
) -> CliError {
    CliError::many(
        errors
            .into_iter()
            .map(|error| compiler_diagnostic(sources, project, fallback_path, error))
            .collect(),
    )
}

fn source_diagnostic(
    stage: DiagnosticStage,
    message: impl Into<String>,
    sources: &SourceMap,
    span: mallang::Span,
    display_path: Option<&Path>,
) -> Diagnostic {
    Diagnostic::error(stage, message).with_span(sources, span, display_path)
}

fn source_load_error(error: SourceLoadError) -> CliError {
    CliError::one(
        Diagnostic::error(
            DiagnosticStage::Input,
            format!("failed to read source: {}", error.io_error()),
        )
        .with_path(error.path()),
    )
}

fn project_source_load_error(project: &Project, error: SourceLoadError) -> CliError {
    CliError::one(
        Diagnostic::error(
            DiagnosticStage::Input,
            format!("failed to read source: {}", error.io_error()),
        )
        .with_path(project.diagnostic_path(error.path())),
    )
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::{
        child_signal_diagnostic, parse_assertion_marker, parse_global_options,
        parse_self_hosted_diagnostics, CompilerImplementation, DiagnosticFormat,
    };
    use mallang::SourceMap;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_explicit_compiler_and_diagnostic_options() {
        let parsed = parse_global_options(&args(&[
            "mlg",
            "--compiler=self",
            "--self-compiler",
            "target/debug/mlgc",
            "--diagnostic-format",
            "json",
            "build",
            "example.mlg",
        ]))
        .unwrap();

        assert_eq!(parsed.diagnostic_format, DiagnosticFormat::Json);
        assert_eq!(
            parsed.compiler.implementation,
            CompilerImplementation::SelfHosted
        );
        assert_eq!(
            parsed.compiler.self_compiler,
            Some(PathBuf::from("target/debug/mlgc"))
        );
        assert_eq!(parsed.command_index, 6);
    }

    #[test]
    fn defaults_to_stage0_without_silent_self_selection() {
        let parsed = parse_global_options(&args(&["mlg", "check", "example.mlg"])).unwrap();

        assert_eq!(parsed.diagnostic_format, DiagnosticFormat::Human);
        assert_eq!(
            parsed.compiler.implementation,
            CompilerImplementation::Stage0
        );
        assert_eq!(parsed.compiler.self_compiler, None);
        assert_eq!(parsed.command_index, 1);
    }

    #[test]
    fn decodes_self_hosted_diagnostics_with_unicode_messages() {
        let mut sources = SourceMap::new();
        sources.add_file("unicode.mlg", "\u{ac00}x\n");

        let diagnostics =
            parse_self_hosted_diagnostics("PERR|0|0|3|236,152,164,235,165,152\n", &sources, None)
                .unwrap();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].render_human(),
            "unicode.mlg:1:1: \u{c624}\u{b958}"
        );
    }

    #[test]
    fn rejects_malformed_self_hosted_diagnostic_records() {
        let mut sources = SourceMap::new();
        sources.add_file("fixture.mlg", "func main() {}\n");

        for record in [
            "BERR|0|0|1|98,97,100",
            "PERR|1|0|1|98,97,100",
            "PERR|0|2|1|98,97,100",
            "PERR|0|0|1|256",
            "PERR|0|0|1",
        ] {
            assert!(
                parse_self_hosted_diagnostics(record, &sources, None).is_err(),
                "record should be rejected: {record}"
            );
        }
    }

    #[test]
    fn rejects_self_compiler_path_without_explicit_self_selection() {
        let error = parse_global_options(&args(&[
            "mlg",
            "--self-compiler",
            "target/debug/mlgc",
            "build",
            "example.mlg",
        ]))
        .unwrap_err();

        assert_eq!(
            error.diagnostics[0].render_human(),
            "--self-compiler requires explicit `--compiler self`"
        );
    }

    #[test]
    fn rejects_missing_self_compiler_path_before_the_command() {
        let error = parse_global_options(&args(&[
            "mlg",
            "--compiler",
            "self",
            "--self-compiler",
            "--version",
        ]))
        .unwrap_err();

        assert_eq!(
            error.diagnostics[0].render_human(),
            "missing value for --self-compiler"
        );
    }

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
            child_signal_diagnostic("project::Test", &signal_status, b"")
                .map(|diagnostic| diagnostic.render_human()),
            Some("test project::Test terminated by signal".to_string())
        );
        assert_eq!(
            child_signal_diagnostic("project::Test", &signal_status, b"runtime detail\n"),
            None
        );
    }
}
