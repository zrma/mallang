use std::{fs, path::PathBuf};

use mallang::{
    check_sources_with_diagnostics, generate_c_sources_with_diagnostics, lex, CompilerStage,
    Parser, SourceMap, Span, Token, TokenKind, MAX_PARSE_ERRORS_PER_SOURCE,
};

#[test]
fn deterministic_utf8_lexer_inputs_have_bounded_spans() {
    for seed in 0..256 {
        let source = generated_utf8(seed);
        match lex(&source) {
            Ok(tokens) => {
                assert!(matches!(
                    tokens.last().map(|token| &token.kind),
                    Some(TokenKind::Eof)
                ));
                let mut previous_end = 0;
                for token in tokens {
                    assert_span(&source, token.span);
                    assert!(token.span.start >= previous_end, "seed {seed}");
                    previous_end = token.span.end;
                }
            }
            Err(error) => assert_span(&source, error.span),
        }
    }
}

#[test]
fn deterministic_parser_token_mutations_never_panic_or_overrun_the_cap() {
    let sources = [
        "func main() { print(1) }",
        "type User struct { name string age int }\nfunc main() { user := User{name: \"kim\", age: 30} print(user.age) }",
        "func choose(value Option[int]) int { return match value { case Some(found) found case None 0 } }\nfunc main() {}",
        "func main() { apply := func(value int) int { return value + 1 } print(apply(2)) }",
    ];
    let replacements = [
        TokenKind::Ident("mutated".to_string()),
        TokenKind::LeftParen,
        TokenKind::RightBrace,
        TokenKind::Comma,
        TokenKind::Eof,
    ];

    assert_parser_result(Vec::new());
    for source in sources {
        let tokens = lex(source).unwrap();
        assert_parser_result(tokens.clone());
        for index in 0..tokens.len() {
            let mut removed = tokens.clone();
            removed.remove(index);
            assert_parser_result(removed);

            let mut duplicated = tokens.clone();
            duplicated.insert(index, tokens[index].clone());
            assert_parser_result(duplicated);

            for kind in &replacements {
                let mut replaced = tokens.clone();
                replaced[index].kind = kind.clone();
                assert_parser_result(replaced);
            }
        }
    }
}

#[test]
fn known_invalid_type_and_ownership_transformations_are_rejected() {
    let cases = [
        InvalidTransformation {
            name: "argument-type",
            valid: "func add(left int, right int) int { return left + right }\nfunc main() { print(add(1, 1)) }\n",
            from: "add(1, 1)",
            to: "add(\"bad\", 1)",
            message: "argument type mismatch",
        },
        InvalidTransformation {
            name: "missing-con",
            valid: "func show(con value string) { print(value) }\nfunc main() { value := \"ok\" show(con value) }\n",
            from: "show(con value)",
            to: "show(value)",
            message: "expects `con` argument",
        },
        InvalidTransformation {
            name: "immutable-mut-borrow",
            valid: "func touch(mut value string) { print(value) }\nfunc main() { mut value := \"ok\" touch(mut value) }\n",
            from: "mut value := \"ok\"",
            to: "value := \"ok\"",
            message: "cannot mutably borrow immutable binding",
        },
        InvalidTransformation {
            name: "borrow-return",
            valid: "func leak(con value string) string { return \"safe\" }\nfunc main() { value := \"ok\" print(leak(con value)) }\n",
            from: "return \"safe\"",
            to: "return value",
            message: "cannot move borrowed value",
        },
        InvalidTransformation {
            name: "use-after-move",
            valid: "func consume(value string) { print(value) }\nfunc main() { value := \"ok\" consume(value) // after move\n}\n",
            from: "// after move",
            to: "print(value)",
            message: "use of moved value",
        },
    ];

    for case in cases {
        assert_check_ok(case.name, case.valid);
        assert_eq!(case.valid.matches(case.from).count(), 1, "{}", case.name);
        let invalid = case.valid.replacen(case.from, case.to, 1);
        let errors = check_source(&invalid).unwrap_err();
        assert_eq!(errors[0].stage, CompilerStage::Semantic, "{}", case.name);
        assert!(
            errors[0].message.contains(case.message),
            "{}: {}",
            case.name,
            errors[0].message
        );
    }
}

#[test]
fn checked_in_crash_corpus_returns_owned_stage_diagnostics() {
    let cases = [
        CrashCase {
            file: "frontend-missing-parameter.mlg",
            source: include_str!("fixtures/hardening/crash-corpus/frontend-missing-parameter.mlg"),
            stage: CompilerStage::Frontend,
            message: "expected parameter name",
        },
        CrashCase {
            file: "package-unresolved-import.mlg",
            source: include_str!("fixtures/hardening/crash-corpus/package-unresolved-import.mlg"),
            stage: CompilerStage::Package,
            message: "unresolved import",
        },
        CrashCase {
            file: "semantic-empty-match.mlg",
            source: include_str!("fixtures/hardening/crash-corpus/semantic-empty-match.mlg"),
            stage: CompilerStage::Semantic,
            message: "match requires at least one arm",
        },
        CrashCase {
            file: "link-invalid-receiver.mlg",
            source: include_str!("fixtures/hardening/crash-corpus/link-invalid-receiver.mlg"),
            stage: CompilerStage::Link,
            message: "method receiver type must be declared in the same package",
        },
        CrashCase {
            file: "ownership-borrow-return.mlg",
            source: include_str!("fixtures/hardening/crash-corpus/ownership-borrow-return.mlg"),
            stage: CompilerStage::Semantic,
            message: "cannot move borrowed value",
        },
        CrashCase {
            file: "ownership-use-after-move.mlg",
            source: include_str!("fixtures/hardening/crash-corpus/ownership-use-after-move.mlg"),
            stage: CompilerStage::Semantic,
            message: "use of moved value",
        },
    ];

    let corpus =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hardening/crash-corpus");
    let mut checked_in = fs::read_dir(corpus)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".mlg"))
        .collect::<Vec<_>>();
    checked_in.sort();
    let mut declared = cases.iter().map(|case| case.file).collect::<Vec<_>>();
    declared.sort();
    assert_eq!(checked_in, declared);

    for case in cases {
        let errors = compile_source(case.source).unwrap_err();
        assert_eq!(errors[0].stage, case.stage, "{}", case.file);
        assert!(
            errors[0].message.contains(case.message),
            "{}: {}",
            case.file,
            errors[0].message
        );
    }
}

fn generated_utf8(seed: u64) -> String {
    const SYNTAX: &[char] = &[
        '\0', ' ', '\n', '\t', 'a', 'Z', '0', '_', '"', '\\', '{', '}', '(', ')', '[', ']', ',',
        '.', ':', ';', '+', '-', '*', '/', '%', '=', '!', '&', '|', '<', '>', '\u{d55c}',
        '\u{b9d0}', '\u{3bb}', '\u{4e2d}',
    ];

    let mut state = seed.wrapping_add(1);
    let length = (next_u64(&mut state) % 65) as usize;
    let mut source = String::new();
    for index in 0..length {
        let value = next_u64(&mut state);
        let ch = if index % 3 == 0 {
            SYNTAX[value as usize % SYNTAX.len()]
        } else {
            let scalar = (value % 0x11_0000) as u32;
            char::from_u32(scalar).unwrap_or('\u{fffd}')
        };
        source.push(ch);
    }
    source
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn assert_span(source: &str, span: Span) {
    assert!(span.start <= span.end);
    assert!(span.end <= source.len());
    assert!(source.is_char_boundary(span.start));
    assert!(source.is_char_boundary(span.end));
}

fn assert_parser_result(tokens: Vec<Token>) {
    if let Err(errors) = Parser::new(tokens).parse_program_with_diagnostics() {
        assert!(!errors.is_empty());
        assert!(errors.len() <= MAX_PARSE_ERRORS_PER_SOURCE);
        assert!(errors
            .windows(2)
            .all(|pair| (pair[0].span.start, pair[0].span.end)
                <= (pair[1].span.start, pair[1].span.end)));
    }
}

fn assert_check_ok(name: &str, source: &str) {
    assert!(check_source(source).is_ok(), "{name}");
}

fn check_source(source: &str) -> Result<mallang::ast::Program, Vec<mallang::CompilerError>> {
    let mut sources = SourceMap::new();
    let source_id = sources.add_file("property.mlg", source);
    check_sources_with_diagnostics(&sources, &[source_id])
}

fn compile_source(source: &str) -> Result<String, Vec<mallang::CompilerError>> {
    let mut sources = SourceMap::new();
    let source_id = sources.add_file("corpus.mlg", source);
    generate_c_sources_with_diagnostics(&sources, &[source_id])
}

struct InvalidTransformation {
    name: &'static str,
    valid: &'static str,
    from: &'static str,
    to: &'static str,
    message: &'static str,
}

struct CrashCase {
    file: &'static str,
    source: &'static str,
    stage: CompilerStage,
    message: &'static str,
}
