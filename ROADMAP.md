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
- [x] Parse `break` and `continue` loop control statements.
- [x] Parse Go-like `for init; condition; post` clause loops.
- [x] Parse initless `for ; condition; post` clause loops.
- [x] Parse conditionless `for` loops and empty-condition clause loops.
- [x] Parse fixed-size array type references.
- [x] Parse fixed-size array literals.
- [x] Parse array-only `range` loops.
- [x] Parse blank identifiers in array-only `range` loops.
- [x] Parse one-variable array-only `range` loops.
- [x] Parse fixed-size array indexing expressions.
- [x] Parse fixed-size array element assignment statements.
- [x] Parse fixed-size array element assignment in `for` clause post targets.
- [x] Parse `con`/`mut` prefix parameter and call modes.
- [x] Parse reserved slice type syntax `[]T`.

## Milestone 2: Static Semantics

- [x] Add first-subset name resolution for local variables and direct function calls.
- [x] Add first-subset primitive type checking for `int`, `bool`, `string`, and `unit`.
- [x] Support `string` equality without moving compared values.
- [x] Support `bool` logical operators `&&` and `||`.
- [x] Support bool unary operator `!`.
- [x] Reject literal integer division and remainder by zero.
- [x] Reject literal integer arithmetic overflow.
- [x] Add first-subset function signature checking.
- [x] Reject `nil`, pointer-like syntax, and unresolved identifiers.
- [x] Reject immutable binding reassignment.
- [x] Support `if` expressions with compatible branch types.
- [x] Support statement-form `if` with branch-local bindings.
- [x] Support return-completeness analysis across statement-form `if` branches.
- [x] Support condition-only `for` statement checking with loop-local bindings.
- [x] Support `for init; condition; post` checking with header-local bindings.
- [x] Support initless `for ; condition; post` checking.
- [x] Support conditionless `for` loops and empty-condition clause loops.
- [x] Reject `break` and `continue` outside loops.
- [x] Type-check fixed-size array literals.
- [x] Type-check array-only `range` loops with immutable `int` index and copy
  element bindings.
- [x] Type-check blank identifiers in array-only `range` loops, including
  index-only range over non-copy element arrays.
- [x] Type-check one-variable array-only `range` loops as index-only iteration.
- [x] Type-check slice `range` loops with Copy value iteration and index-only
  non-Copy iteration.
- [x] Type-check fixed-size array indexing for copy elements.
- [x] Type-check `len([N]T)` as a read-only built-in returning `int`.
- [x] Type-check fixed-size array element assignment for mutable copy and
  non-copy element arrays.
- [x] Type-check fixed-size array element assignment in `for` clause post
  targets.
- [x] Type-check consuming `append(slice, item)` for owned slices.
- [x] Reject recursive struct value type definitions.
- [x] Reject top-level type/function declaration name conflicts.
- [x] Reject user value bindings that collide with built-in value names.
- [x] Reserve `append` ahead of slice growth support.
- [x] Allow shadowing only in nested blocks or arm-local scopes while rejecting
  same-block redeclarations.
- [x] Reject non-printable `print` arguments before native backend lowering.
- [x] Reject statement-only `print` in value positions before native backend
  lowering.
- [x] Reject reserved slice type syntax `[]T` with an explicit diagnostic.

## Milestone 3: Ownership and Borrowing

- [x] Treat `int` and `bool` as `Copy`.
- [x] Treat `string` and structs as move-only by default.
- [x] Decide fixed-size array ownership and defer slices.
- [x] Treat fixed-size arrays as move-only values.
- [x] Support explicit read borrow calls with `con expr`.
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
- [x] Expose compile-and-execute through `mlg run`.
- [x] Expose crate version through `mlg --version`.
- [x] Keep CLI help/error streams covered by smoke checks.
- [x] Generate native C blocks for statement-form `if`.
- [x] Lower `con`/`mut` parameters to a hidden-reference C ABI.
- [x] Generate C temps for `if` expression branches that need prelude statements.
- [x] Generate C temps for `match` expression arms that need prelude statements.
- [x] Lower `bool` logical operators to native short-circuit code.
- [x] Lower bool unary operator `!` to native C.
- [x] Guard integer division and remainder by zero in native C.
- [x] Guard integer arithmetic overflow in native C.
- [x] Compile `|>` pipeline call sugar through the existing call backend.
- [x] Generate native C `while` loops for condition-only `for` statements.
- [x] Generate native C `break` and `continue` statements.
- [x] Generate native C `for` loops for `for init; condition; post`.
- [x] Generate native C `for` loops for initless clause loops.
- [x] Generate native C loops for conditionless `for` forms.
- [x] Generate native C layout for fixed-size arrays.
- [x] Generate native C loops for array-only `range`.
- [x] Support local-rooted slice field len/index/range/borrow reads.
- [x] Generate native C loops for one-variable array-only `range`.
- [x] Generate native C loops for slice `range`.
- [x] Preserve nested block shadowing in generated C `for` and `range` bodies.
- [x] Generate native C for fixed-size array indexing.
- [x] Generate native C for fixed-size array `len`.
- [x] Generate native C bounds checks for fixed-size array indexing.
- [x] Generate native C for fixed-size array element assignment.
- [x] Generate native C for fixed-size array element assignment in `for` clause
  post targets.
- [x] Generate native C for owned slice literals, `len(slice)`, and Copy-only
  `slice[i]` value access.
- [x] Generate native C for consuming slice `append(slice, item)` growth.
- [x] Generate native C for direct slice field append reassignment.
- [x] Generate native C for owned slice field take expressions.
- [x] Split backend public API from C backend implementation module.
- [x] Split C backend name/type helper utilities into a dedicated module.
- [x] Split C backend type layout and drop helper emission into a dedicated module.
- [x] Split C backend statement emission into a dedicated module.
- [x] Split C backend expression emission into a dedicated module.
- [x] Split C backend shared utility and temp-name helpers into a dedicated module.
- [x] Split C backend unit tests into a dedicated test module.
- [x] Keep mutable range value syntax deferred with parser/semantic regressions.
- [x] Keep by-reference range iteration syntax deferred with parser regression coverage.
- [x] Keep general non-slice field partial moves deferred with semantic regression coverage.
- [x] Keep statement-spanning borrow syntax deferred with parser regression coverage.
- [x] Keep legacy borrow alias syntax out with parser regression coverage.
- [x] Refresh SPEC slice cleanup wording from staging plan to current model.
- [x] Centralize generated C runtime failures through a single helper.
- [x] Verify native runtime failure stderr in smoke checks.
- [x] Add C backend IR invariant regression coverage.
- [x] Guard native slice literal allocation-size overflow.
- [x] Cover indexed slice field append-take C lowering.
- [x] Keep every checked-in example connected to the smoke gate.
- [x] Run cleanup-heavy generated C through sanitizer smoke.
- [x] Keep representative generated C warning-clean under strict clang flags.

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
- [x] Canonical `con name T` / `mut name T` prefix parameter modes.
- [x] Native lowering for `for` clause condition/post expressions that need
  temporary prelude statements.
- [x] Fixed-size array element borrow arguments for copy and non-copy elements.
- [x] Fixed-size array non-copy element assignment.
- [x] Fixed-size array element method receivers.
- [x] Parse slice type syntax `[]T` and reserve it at semantic checking.
- [x] Open source-level owned slice literals, `len(slice)`, and Copy-only
  `slice[i]` value access.
- [x] Slice append/growth.
- [x] Slice range with Copy value iteration.
- [x] Slice element borrow arguments for direct local owned slices.
- [x] Slice element assignment for local-rooted mutable owned slices.
- [x] Same-field append reassignment for direct owned slice field paths.
- [x] Same-field append reassignment for stable indexed owned slice field paths.
- [x] Field-take append sources for owned slice fields.
- [x] Owned slice field take expressions.
- [x] Indexed field assignment for array/slice element fields.
- [x] Borrowed indexing expressions for read-only non-copy element inspection.
- [x] Struct cleanup for owned slice fields.
- [x] Local-rooted slice field len/index/range/borrow reads.
- [x] Local-rooted array/slice element assignment.
- [x] C backend name/type helper module split.
- [x] C backend type emitter module split.
- [x] C backend statement emitter module split.
- [x] C backend expression emitter module split.
- [x] C backend utility/temp-name helper module split.
- [x] C backend unit test module split.
- [x] Mutable range values deferred with explicit regression coverage.
- [x] By-reference range iteration deferred with explicit regression coverage.
- [x] General non-slice field partial moves deferred with explicit regression coverage.
- [x] Statement-spanning borrow syntax deferred with explicit regression coverage.
- [x] Legacy borrow alias syntax rejected with explicit regression coverage.
- [x] SPEC slice cleanup wording refreshed to current model.
- [x] Generated C runtime failure helper centralization.
- [x] Native runtime failure stderr smoke coverage.
- [x] C backend IR invariant regression coverage.
- [x] Native slice literal allocation-size overflow guard.
- [x] Indexed slice field append-take C lowering regression.
- [x] `mlg --version` CLI smoke.
- [x] `mlg --help` and CLI error stream smoke.
- [x] v0 release-candidate audit and example smoke coverage guard.
- [x] Generated C memory/lifetime sanitizer smoke.
- [x] Generated C strict warning smoke.
- [x] Deep generated C sanitizer sweep command.
- [x] Full generated C strict warning gate.
- [x] v0 release-candidate pre-publish verification command.
- [x] Publish finalizer wired to v0 RC verification gate.
- [x] v0 release-candidate notes.
- [x] Publish readiness verify-only finalizer path.
- [x] Release helper contract checks wired into v0 RC verification.
- [x] Publish finalizer remote freshness guard.
- [x] Publish finalizer preflight and final remote freshness gates.
- [x] Publish finalizer no-push dry run remote freshness checks.
- [x] Publish finalizer missing option value diagnostics.
- [x] Publish finalizer post-push remote bookmark verification.
- [x] Release binary smoke wired into v0 RC verification.
- [x] Release binary run command smoke wired into v0 RC verification.
- [x] Release binary frontend and IR smoke wired into v0 RC verification.
- [x] Release binary safety rejection smoke wired into v0 RC verification.
- [x] Release binary CLI error stream smoke wired into v0 RC verification.
- [x] Release binary build argument error smoke wired into v0 RC verification.
- [x] Blank identifiers in array-only `range` loops.
- [x] One-variable array-only `range` loops.

## Post-v0 to v1

- The nine planned release milestones from `v0.2.0` through `v1.0.0` are owned
  by `docs/V1_ROADMAP.md`.
- `v0.2.0: Projects and Modules` is implemented locally; release publication is
  still pending.
- `v0.3.0: Functions and Closures` is implemented locally; release publication
  is still pending.
- `v0.4.0: Generic Data Model` is implemented locally; release publication is
  still pending.
- `v0.5.0: Ownership and Runtime` is implemented locally; release publication
  is still pending.
- `v0.6.0: Standard Library` is in progress. P147 standard package registry,
  P148 UTF-8 text/error runtime, and P149 process/stream I/O are complete; P150
  file I/O is next.
- Generic ADTs, ownership/runtime completion, standard library, tooling,
  hardening, and language freeze must advance through the acceptance gates in
  that document.
- First-class references, interfaces/dynamic dispatch, C interop, and an
  LLVM/Cranelift backend remain decision-gated rather than implied v1 features.
