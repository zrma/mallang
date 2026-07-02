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
- Native `in`/`mut` parameter ABI uses hidden references, so `mut` parameter
  assignments are visible to the caller without exposing pointer syntax.
- Native compilation path through a C backend first.
- Functional features in the core language: `if` statements/expressions,
  condition-only `for` loops with `break` / `continue`, `else if` sugar,
  `bool` logical operators, `|>` pipeline call sugar, `Option`, `Result`, and
  expression/statement `match`.
- `Option` and `Result` values with printable payloads can be printed natively.
- Branch-aware return completeness for statement-form `if`.
- Go-like data modeling with `type Name struct`, named struct literals, and
  nested field access/assignment.
- Struct values with printable fields can be printed natively.
- Go-like receiver methods with Mallang parameter modes.
- Field-level borrow arguments for local-rooted field paths such as
  `in user.name` and `mut user.profile.name`.

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
cargo run --bin mlg -- build examples/for-loop.mlg -o target/mallang/for-loop
target/mallang/for-loop
cargo run --bin mlg -- build examples/loop-control.mlg -o target/mallang/loop-control
target/mallang/loop-control
cargo run --bin mlg -- build examples/string-equality.mlg -o target/mallang/string-equality
target/mallang/string-equality
cargo run --bin mlg -- build examples/logical-operators.mlg -o target/mallang/logical-operators
target/mallang/logical-operators
cargo run --bin mlg -- build examples/pipeline.mlg -o target/mallang/pipeline
target/mallang/pipeline
cargo run --bin mlg -- build examples/adt.mlg -o target/mallang/adt
target/mallang/adt
cargo run --bin mlg -- build examples/print-adt.mlg -o target/mallang/print-adt
target/mallang/print-adt
cargo run --bin mlg -- build examples/match-temp.mlg -o target/mallang/match-temp
target/mallang/match-temp
cargo run --bin mlg -- build examples/if-match-expression.mlg -o target/mallang/if-match-expression
target/mallang/if-match-expression
cargo run --bin mlg -- build examples/match-arm-prelude.mlg -o target/mallang/match-arm-prelude
target/mallang/match-arm-prelude
cargo run --bin mlg -- build examples/structs.mlg -o target/mallang/structs
target/mallang/structs
cargo run --bin mlg -- build examples/print-struct.mlg -o target/mallang/print-struct
target/mallang/print-struct
cargo run --bin mlg -- build examples/methods.mlg -o target/mallang/methods
target/mallang/methods
cargo run --bin mlg -- build examples/mut-receiver.mlg -o target/mallang/mut-receiver
target/mallang/mut-receiver
cargo run --bin mlg -- build examples/field-assignment.mlg -o target/mallang/field-assignment
target/mallang/field-assignment
cargo run --bin mlg -- build examples/field-borrow.mlg -o target/mallang/field-borrow
target/mallang/field-borrow
cargo run --bin mlg -- build examples/mut-parameter-abi.mlg -o target/mallang/mut-parameter-abi
target/mallang/mut-parameter-abi
cargo run --bin mlg -- build examples/nested-fields.mlg -o target/mallang/nested-fields
target/mallang/nested-fields
cargo run --bin mlg -- build examples/return-completeness.mlg -o target/mallang/return-completeness
target/mallang/return-completeness
cargo run --bin mlg -- build examples/else-if.mlg -o target/mallang/else-if
target/mallang/else-if
cargo run --bin mlg -- build examples/match-statement.mlg -o target/mallang/match-statement
target/mallang/match-statement
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
- `examples/for-loop.mlg`: native smoke for condition-only `for` loops.
- `examples/loop-control.mlg`: native smoke for `break` and `continue`.
- `examples/string-equality.mlg`: native smoke for `string` equality without moving values.
- `examples/logical-operators.mlg`: native smoke for `bool` logical operators and short-circuiting.
- `examples/pipeline.mlg`: native smoke for `|>` pipeline call sugar.
- `examples/adt.mlg`: native smoke for `Option` / `Result` constructors and `match`.
- `examples/print-adt.mlg`: native smoke for printing `Option` / `Result` values.
- `examples/match-temp.mlg`: native smoke for expression scrutinees in `match`.
- `examples/if-match-expression.mlg`: native smoke for `if` expression branches that need C preludes.
- `examples/match-arm-prelude.mlg`: native smoke for `match` expression arms that need C preludes.
- `examples/structs.mlg`: native smoke for struct declarations, literals, and field access.
- `examples/print-struct.mlg`: native smoke for printing struct values with nested fields.
- `examples/methods.mlg`: native smoke for struct receiver methods.
- `examples/mut-receiver.mlg`: native smoke for caller-visible `mut` receiver methods.
- `examples/field-assignment.mlg`: native smoke for mutable struct field assignment.
- `examples/field-borrow.mlg`: native smoke for direct field borrow arguments.
- `examples/mut-parameter-abi.mlg`: native smoke for caller-visible `mut` parameter mutation.
- `examples/nested-fields.mlg`: native smoke for nested field assignment and borrow arguments.
- `examples/return-completeness.mlg`: native smoke for branch-aware return analysis.
- `examples/else-if.mlg`: native smoke for statement-form `else if` sugar.
- `examples/match-statement.mlg`: native smoke for statement-form `match` block arms.
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

Current status: implemented for the first `int`, `bool`, `string`, string equality,
`bool` logical operators, `|>` pipeline call sugar, statement/expression `if`,
condition-only `for` loops with `break` / `continue`, `else if` sugar, branch-aware returns,
struct/method/nested-field, struct print output, and built-in ADT
expression/statement `match` plus ADT print output via C source generation and
`clang`.
