use std::{env, fs, process};

use mallang::lex;

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "mlg".to_string());

    let Some(path) = args.next() else {
        eprintln!("usage: {program} <source-file>");
        process::exit(2);
    };

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{path}: failed to read source: {error}");
            process::exit(1);
        }
    };

    match lex(&source) {
        Ok(tokens) => {
            for token in tokens {
                println!(
                    "{:?} @ {}..{}",
                    token.kind, token.span.start, token.span.end
                );
            }
        }
        Err(error) => {
            eprintln!("{path}: {error}");
            process::exit(1);
        }
    }
}
