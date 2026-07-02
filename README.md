# Mallang PoC

Go-like syntax, Rust-like safety, and functional value style.

This repository is the Mallang language PoC workspace.

## Naming

- Language: Mallang
- File extension: `.mlg`
- User-facing CLI: `mlg`
- Compiler command shape: `mlg build`, not a separate long `mallangc` command
- Internal compiler crate or binary name, if needed later: `mlgc`

## Current Scope

- Go-like surface syntax.
- No pointer syntax.
- No `nil`.
- Immutable bindings by default.
- Ownership by default for non-copy values.
- Explicit `in` and `mut` borrow calls.
- Native compilation path through a C backend first.
- Functional features in the core language: `if` expressions, `Option`,
  `Result`, and `match`.

## Bootstrap

The current executable can lex, parse, and build the first native subset.

```sh
cargo run --bin mlg -- lex examples/hello.mlg
cargo run --bin mlg -- parse examples/first.mlg
cargo run --bin mlg -- build examples/first.mlg -o target/mallang/first
target/mallang/first
```

Run the full local gate:

```sh
scripts/check.sh
```

## Layout

- `SPEC.md`: v0 language specification.
- `ROADMAP.md`: implementation milestones.
- `examples/hello.mlg`: first target source program.
- `src/lexer.rs`: initial hand-written lexer.
- `src/token.rs`: token model shared by later parser work.

## First Milestone

Compile this program to a native binary that prints `30`.

```go
func main() {
    x := 10
    y := add(x, 20)
    print(y)
}

func add(a int, b int) int {
    return a + b
}
```

Current status: implemented for the first `int` subset via C source generation
and `clang`.
