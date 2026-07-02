# Roadmap

## Milestone 0: Bootstrap

- Create a standalone PoC folder.
- Write the v0 language spec.
- Add a minimal lexer and token model.
- Keep the implementation dependency-free until the first parser shape is clear.
- Use `Mallang` as the language name, `.mlg` as the source extension, and `mlg`
  as the short user-facing command.

## Milestone 1: Syntax Frontend

- Expand the lexer as syntax settles.
- Implement a recursive descent parser for declarations, statements, and blocks.
- Implement a Pratt parser for expressions.
- Produce an AST with precise source spans.
- Parse the first target program.

## Milestone 2: Static Semantics

- Add name resolution.
- Add primitive type checking.
- Add function signature checking.
- Reject `nil`, pointer-like syntax, and unresolved identifiers.
- Support `if` expressions with compatible branch types.

## Milestone 3: Ownership and Borrowing

- Treat `int` and `bool` as `Copy`.
- Treat `string`, arrays, and structs as move-only by default.
- Support explicit read borrow calls with `in expr`.
- Support explicit mutable borrow calls with `mut expr`.
- Reject use-after-move.
- Reject overlapping mutable/read borrows.
- For v0, disallow storing or returning borrowed values.

## Milestone 4: Native Backend

- Lower typed AST to a small typed IR.
- Generate C source from IR.
- Use `clang` as the first native backend.
- Produce a native executable for the first target program.
- Expose compilation through `mlg build` rather than a separate long compiler
  command.

## Later

- Struct literals and methods.
- `Option[T]`, `Result[T, E]`, and `match`.
- Modules/packages.
- Closures and higher-order functions.
- C interop boundary.
- LLVM or Cranelift backend if the C backend starts limiting the design.
