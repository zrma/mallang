# Roadmap

## Milestone 0: Bootstrap

- Create a standalone PoC folder.
- Write the v0 language spec.
- Add a minimal lexer and token model.
- Keep the implementation dependency-free until the first parser shape is clear.
- Use `Mallang` as the language name, `.mlg` as the source extension, and `mlg`
  as the short user-facing command.

## Milestone 1: Syntax Frontend

- [x] Expand the lexer as syntax settles.
- [x] Implement a recursive descent parser for declarations, statements, and blocks.
- [x] Implement a Pratt parser for expressions.
- [x] Produce an AST with precise source spans.
- [x] Parse the first target program.
- [x] Parse `else if` as nested `if` sugar.
- [x] Parse `|>` pipeline call sugar.
- [x] Parse condition-only `for` statements.

## Milestone 2: Static Semantics

- [x] Add first-subset name resolution for local variables and direct function calls.
- [x] Add first-subset primitive type checking for `int`, `bool`, `string`, and `unit`.
- [x] Support `string` equality without moving compared values.
- [x] Support `bool` logical operators `&&` and `||`.
- [x] Add first-subset function signature checking.
- [x] Reject `nil`, pointer-like syntax, and unresolved identifiers.
- [x] Reject immutable binding reassignment.
- [x] Support `if` expressions with compatible branch types.
- [x] Support statement-form `if` with branch-local bindings.
- [x] Support return-completeness analysis across statement-form `if` branches.
- [x] Support condition-only `for` statement checking with loop-local bindings.

## Milestone 3: Ownership and Borrowing

- [x] Treat `int` and `bool` as `Copy`.
- [x] Treat `string`, arrays, and structs as move-only by default.
- [x] Support explicit read borrow calls with `in expr`.
- [x] Support explicit mutable borrow calls with `mut expr`.
- [x] Reject use-after-move.
- [x] Reject overlapping mutable/read borrows within one call.
- [x] Disallow moving non-copy borrowed parameters into owned locals, owned
  arguments, or returns.

## Milestone 4: Native Backend

- [x] Lower typed AST to a small typed IR.
- [x] Generate C source for the first `int` subset.
- [x] Use `clang` as the first native backend.
- [x] Produce a native executable for the first target program.
- [x] Expose compilation through `mlg build` rather than a separate long compiler
  command.
- [x] Generate native C blocks for statement-form `if`.
- [x] Lower `in`/`mut` parameters to a hidden-reference C ABI.
- [x] Generate C temps for `if` expression branches that need prelude statements.
- [x] Generate C temps for `match` expression arms that need prelude statements.
- [x] Lower `bool` logical operators to native short-circuit code.
- [x] Compile `|>` pipeline call sugar through the existing call backend.
- [x] Generate native C `while` loops for condition-only `for` statements.

## Later

- Struct literals and methods.
- [x] `Option[T]`, `Result[T, E]`, and `match` surface design.
- [x] Parse generic type references for `Option[T]` and `Result[T, E]`.
- [x] Type-check `Some`, `None`, `Ok`, and `Err` constructors.
- [x] Add exhaustive `match` for built-in ADTs.
- [x] Lower built-in ADTs into tagged typed IR.
- [x] Add C backend layout for built-in ADTs.
- [x] Print built-in ADT values with printable payloads in the C backend.
- [x] Generate native code for non-local `match` scrutinee expressions.
- [x] Add statement-form `match` with block arms.
- [x] Struct declarations, named literals, and field access.
- [x] Struct receiver methods.
- [x] Caller-visible `mut` receiver methods in the native backend.
- [x] Direct mutable struct field assignment.
- [x] Field-level borrow arguments for direct local fields.
- [x] Nested field assignment and nested field borrow arguments.
- [x] Print struct values with printable fields in the C backend.
- Modules/packages.
- Closures and higher-order functions.
- C interop boundary.
- LLVM or Cranelift backend if the C backend starts limiting the design.
