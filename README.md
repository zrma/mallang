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
- Functional features in the core language: `if` statements/expressions,
  `Option`, `Result`, and `match`.
- Go-like data modeling with `type Name struct`, named struct literals, and
  field access/assignment.
- Go-like receiver methods with Mallang parameter modes.

## Bootstrap

The current executable can lex, parse, check, and build the first native subset.

```sh
cargo run --bin mlg -- lex examples/hello.mlg
cargo run --bin mlg -- parse examples/first.mlg
cargo run --bin mlg -- check examples/first.mlg
cargo run --bin mlg -- ir examples/adt.mlg
cargo run --bin mlg -- build examples/first.mlg -o target/mallang/first
target/mallang/first
cargo run --bin mlg -- build examples/if.mlg -o target/mallang/if
target/mallang/if
cargo run --bin mlg -- build examples/if-statement.mlg -o target/mallang/if-statement
target/mallang/if-statement
cargo run --bin mlg -- build examples/adt.mlg -o target/mallang/adt
target/mallang/adt
cargo run --bin mlg -- build examples/match-temp.mlg -o target/mallang/match-temp
target/mallang/match-temp
cargo run --bin mlg -- build examples/structs.mlg -o target/mallang/structs
target/mallang/structs
cargo run --bin mlg -- build examples/methods.mlg -o target/mallang/methods
target/mallang/methods
cargo run --bin mlg -- build examples/field-assignment.mlg -o target/mallang/field-assignment
target/mallang/field-assignment
```

Run the full local gate:

```sh
scripts/check.sh
```

## Layout

- `SPEC.md`: v0 language specification.
- `ROADMAP.md`: implementation milestones.
- `examples/hello.mlg`: first target source program.
- `examples/if.mlg`: native smoke for `if` expressions.
- `examples/if-statement.mlg`: native smoke for statement-form `if`.
- `examples/adt.mlg`: native smoke for `Option` / `Result` constructors and `match`.
- `examples/match-temp.mlg`: native smoke for expression scrutinees in `match`.
- `examples/structs.mlg`: native smoke for struct declarations, literals, and field access.
- `examples/methods.mlg`: native smoke for struct receiver methods.
- `examples/field-assignment.mlg`: native smoke for mutable struct field assignment.
- `src/lexer.rs`: initial hand-written lexer.
- `src/parser.rs`: AST parser for the current v0 subset.
- `src/semantic.rs`: semantic checker for name/type/function diagnostics.
- `src/ir.rs`: typed IR lowering after semantic analysis.
- `src/backend.rs`: C backend for typed IR in the first native subset.
- `src/token.rs`: token model shared by the frontend.

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

Current status: implemented for the first `int`, `bool`, `string`,
statement/expression `if`, struct/method, and built-in ADT subset via C source
generation and `clang`.
