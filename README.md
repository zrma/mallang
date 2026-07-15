# Mallang PoC

Go-like syntax, Rust-like safety, and functional value style.

This repository is the Mallang language PoC workspace.

## Naming

- Language: Mallang
- File extension: `.mlg`
- User-facing CLI: `mlg`
- Compiler command shape: `mlg build`, not a separate long `mallangc` command
- Run command shape: `mlg run`
- Format command shape: `mlg fmt`
- Test command shape: `mlg test`
- Machine diagnostic shape: `mlg --diagnostic-format json check <input>`
- Version command shape: `mlg --version`
- Help command shape: `mlg --help`
- Internal compiler crate or binary name, if needed later: `mlgc`

## Current Scope

- Go-like surface syntax.
- Entrypoint is exactly `func main()`; `main` cannot be used for methods,
  parameters, or return-bearing signatures in v0.
- No pointer syntax.
- No `nil`.
- Immutable bindings by default.
- Built-in value names such as `print`, `len`, `append`, `Some`, `None`, `Ok`,
  and `Err` are reserved in user value bindings.
- Top-level `type` and non-method `func` declarations cannot use the same name.
- Ownership by default for non-copy values.
- `string` is one immutable move-only value type across static literals and
  future heap-owned buffers. Storage kind stays internal; equality, printing,
  moves, and recursive cleanup share one runtime contract.
- Explicit `con` and `mut` borrow calls.
- `con` and `mut` are direct call-scoped argument modes, not first-class
  reference expressions. Non-copy range traversal is index-only and uses
  indexed `con`/`mut` calls or indexed assignment for element access.
- Borrow mode syntax is canonical in v0; there are no legacy aliases such as
  `in`.
- Native `con`/`mut` parameter ABI uses hidden references, so `mut` parameter
  assignments are visible to the caller without exposing pointer syntax.
- Native compilation path through a C backend first.
- Functional features in the core language: `if` statements/expressions,
  condition-only, conditionless, and `for init; condition; post` loops with
  `break` / `continue`, `else if` sugar, `bool` operators, `|>`
  pipeline call sugar, `Option`, `Result`, and expression/statement `match`.
- Named functions are move-only first-class values with typed higher-order
  parameters, returns, and native indirect calls. Plain function literals use
  owned-by-value captures with heap environments and exactly-once cleanup.
  Mutable function literals preserve exclusive call access and can update owned
  captures from mutable source bindings. Nested function literals propagate
  free variables through owned closure environments. Package-qualified named
  functions and public function-typed APIs work across project packages.
- `Option` and `Result` values with printable payloads can be printed natively.
- `mlg check` rejects `print` for non-printable values such as fixed-size
  arrays and composites containing fixed-size arrays, and rejects `print` in
  value positions.
- Branch-aware return completeness for statement-form `if`.
- Go-like data modeling with `type Name struct`, named struct literals, and
  nested field access/assignment.
- Generic structs, functions, and receiver methods use explicit type arguments
  and project-wide demand-driven static specialization, including public APIs
  imported across packages. Generic declarations are also checked once with
  non-Copy symbolic types. Concrete function values and slice type arguments
  reuse the existing ownership, typed IR, and native C backend path.
- Struct values with printable fields can be printed natively.
- Recursive struct value definitions are rejected by `mlg check`.
- Go-like receiver methods with Mallang parameter modes.
- Field-level borrow arguments for local-rooted field paths such as
  `con user.name` and `mut user.profile.name`.
- Fixed-size arrays with range loops, element borrow arguments, and `con`/`mut`
  method receivers plus read-only indexed expressions for non-copy elements.
- Owned move-only slices with `[]T` literals, `len(slice)`, Copy-only
  `slice[i]`, consuming `append(slice, item)`, and range loops with Copy value
  iteration plus element borrow, assignment, indexed field assignment, and
  read-only indexed expressions for non-copy elements.
- Struct cleanup for owned slice fields, plus local-rooted slice field
  `len`/index/range/borrow reads, element assignment, and same-field append
  reassignment for direct and stable indexed field paths. Append can also take
  a slice field source and leave that source field empty. Ordinary owned value
  positions can also take a slice field, such as `taken := bag.values` or
  `consume(bag.values)`, with the source field reset to empty.
- Cleanup-valued computed expressions use compiler-owned full-expression
  temporaries. Inline slice `len`/index/range sources, discarded results, and
  computed `con`/`mut` call arguments clean up after their final use while
  preserving logical short-circuit evaluation.
- Compiler-owned slice, closure, recursive enum, and string allocations share
  one internal accounting/failure boundary. Normal program return is checked
  against zero live allocations without exposing allocator APIs in Mallang.
- Integer division and remainder guard zero divisors before native execution can
  reach C undefined behavior.
- Integer arithmetic guards overflow before native execution can reach C signed
  overflow undefined behavior.
- Multi-file projects use `mallang.toml`, an `src/` tree, directory packages,
  explicit imports, and `pub` visibility. `mlg fmt`, `mlg check`, `mlg build`, and
  `mlg run` accept either a standalone `.mlg` file or a project directory or
  manifest.
- Optional `[dependencies]` entries map an exact project name to a
  manifest-relative local directory path. Dependencies load in deterministic
  dependency-first order and expose only ordinary `pub` package APIs. Project
  cycles, name/path collisions, undeclared transitive imports, dependency
  entrypoints, and dependency tests are excluded or rejected by the project
  graph.
- Projects without `src/main.mlg` are libraries: they support format, check, and
  test, while build and run require an executable root entry source.
- `mlg fmt` applies the comment-preserving canonical style. `mlg fmt --check`
  performs the same deterministic project traversal without writing files and
  exits non-zero when formatting changes are required.
- Project tests live in an optional `tests/` tree mirroring `src/` packages.
  Contextual `test Name() { ... }` declarations use standalone `assert(bool)`
  statements. `mlg test <project> [--exact <test-id>]` preflights the whole suite,
  then runs selected tests as deterministic isolated native children without
  invoking the application `main`.
- `mlg --diagnostic-format json <subcommand> ...` emits versioned
  `mallang.diagnostic.v1` JSON Lines for compiler-owned errors on stderr. Human
  diagnostics remain the default, and both forms share stage, message, source,
  UTF-8 byte span, and 1-based Unicode scalar location data.
- v0.8 parser recovery reports up to 32 deterministic frontend diagnostics per
  source without entering later compiler stages. Deterministic lexer/parser/type/
  ownership properties, a checked-in crash corpus, typed IR preflight validation,
  and release-binary corpus smoke defend user-reachable compiler invariants.
- v0.8 observational performance data records representative check/build/runtime
  medians and generated C/native sizes without imposing noisy CI thresholds.
  Repeated generated C and release archive builds are byte-identical within the
  documented scope; native executable identity remains excluded.
- v0.8 release tooling builds deterministic native archives for macOS arm64 and
  Linux x86_64, writes `SHA256SUMS`, and installs or replaces `mlg` only after
  checksum, archive shape, and binary version verification. `clang` remains a
  runtime prerequisite for `mlg build`, `mlg run`, and `mlg test`.
- Compiler-owned `std/errors`, `std/fs`, `std/io`, `std/os`, `std/strings`, and
  `std/collections` packages resolve in both project and standalone mode. Their
  exact signatures, ownership checks, explicit generic specialization, opaque
  `Map[K,V]`, and typed intrinsic IR are implemented. `std/strings` now provides
  UTF-8 byte/scalar operations, search, split/join, strict int/bool conversion,
  standard `Error` results, function-value thunks, and allocation-safe native
  cleanup. `std/os` provides validated process arguments, environment lookup,
  and explicit exit; `std/io` provides UTF-8 stdin and exact stdout/stderr writes
  with recoverable errors. `std/fs` provides UTF-8 text reads and create-or-
  overwrite exact writes with recoverable open/read/write/close errors.
  `std/collections` provides an opaque move-only `Map[K,V]` with deterministic
  `int`/`bool`/`string` hashing, owned insert/remove, call-scoped read/update
  callbacks, allocation-safe growth, and recursive cleanup.
- `examples/projects/textstats` is a multi-package native CLI that reads UTF-8
  input, summarizes it with `std/strings` and `Map[int,int]`, and writes to a
  file or stdout with explicit `Result`-to-stderr/exit handling.

## Bootstrap

The current executable can lex, parse, format, check, build, run, and test the
implemented native subset.

```sh
cargo run --bin mlg -- lex examples/hello.mlg
cargo run --bin mlg -- --version
cargo run --bin mlg -- --help
cargo run --bin mlg -- parse examples/first.mlg
cargo run --bin mlg -- fmt --check examples/first.mlg
cargo run --bin mlg -- check examples/first.mlg
cargo run --bin mlg -- --diagnostic-format json check examples/first.mlg
cargo run --bin mlg -- ir examples/adt.mlg
cargo run --bin mlg -- build examples/first.mlg -o target/mallang/first
target/mallang/first
cargo run --bin mlg -- run examples/function-values.mlg
cargo run --bin mlg -- run examples/closures.mlg
cargo run --bin mlg -- run examples/mutable-closures.mlg
cargo run --bin mlg -- run examples/nested-closures.mlg
cargo run --bin mlg -- run examples/generics.mlg
cargo run --bin mlg -- build examples/if.mlg -o target/mallang/if
target/mallang/if
cargo run --bin mlg -- build examples/int-division.mlg -o target/mallang/int-division
target/mallang/int-division
cargo run --bin mlg -- build examples/checked-arithmetic.mlg -o target/mallang/checked-arithmetic
target/mallang/checked-arithmetic
cargo run --bin mlg -- build examples/if-statement.mlg -o target/mallang/if-statement
target/mallang/if-statement
cargo run --bin mlg -- build examples/for-loop.mlg -o target/mallang/for-loop
target/mallang/for-loop
cargo run --bin mlg -- build examples/loop-control.mlg -o target/mallang/loop-control
target/mallang/loop-control
cargo run --bin mlg -- build examples/for-clause.mlg -o target/mallang/for-clause
target/mallang/for-clause
cargo run --bin mlg -- build examples/for-clause-initless.mlg -o target/mallang/for-clause-initless
target/mallang/for-clause-initless
cargo run --bin mlg -- build examples/for-empty-condition.mlg -o target/mallang/for-empty-condition
target/mallang/for-empty-condition
cargo run --bin mlg -- build examples/range-blank.mlg -o target/mallang/range-blank
target/mallang/range-blank
cargo run --bin mlg -- build examples/range-index.mlg -o target/mallang/range-index
target/mallang/range-index
cargo run --bin mlg -- run examples/range-index.mlg
cargo run --bin mlg -- build examples/non-copy-array-assignment.mlg -o target/mallang/non-copy-array-assignment
target/mallang/non-copy-array-assignment
cargo run --bin mlg -- build examples/for-clause-prelude.mlg -o target/mallang/for-clause-prelude
target/mallang/for-clause-prelude
cargo run --bin mlg -- build examples/string-equality.mlg -o target/mallang/string-equality
target/mallang/string-equality
cargo run --bin mlg -- run examples/string-runtime.mlg
cargo run --bin mlg -- run examples/borrow-range-contract.mlg
cargo run --bin mlg -- run examples/allocation-accounting.mlg
cargo run --bin mlg -- run examples/standard-strings.mlg
printf 'input' | MALLANG_P149_TEST=값 cargo run --bin mlg -- run examples/process-io.mlg -- alpha
printf 'text' > target/mallang/file-input.txt
cargo run --bin mlg -- run examples/file-io.mlg -- target/mallang/file-input.txt target/mallang/file-output.txt
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
cargo run --bin mlg -- build examples/slice-field-read.mlg -o target/mallang/slice-field-read
target/mallang/slice-field-read
cargo run --bin mlg -- build examples/slice-field-assignment.mlg -o target/mallang/slice-field-assignment
target/mallang/slice-field-assignment
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
cargo run --bin mlg -- build examples/array-element-borrow.mlg -o target/mallang/array-element-borrow
target/mallang/array-element-borrow
cargo run --bin mlg -- build examples/slice-element-borrow.mlg -o target/mallang/slice-element-borrow
target/mallang/slice-element-borrow
cargo run --bin mlg -- build examples/slice-element-assignment.mlg -o target/mallang/slice-element-assignment
target/mallang/slice-element-assignment
cargo run --bin mlg -- build examples/indexed-field-assignment.mlg -o target/mallang/indexed-field-assignment
target/mallang/indexed-field-assignment
cargo run --bin mlg -- build examples/array-element-methods.mlg -o target/mallang/array-element-methods
target/mallang/array-element-methods
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
cargo run --bin mlg -- check examples/projects/hello
cargo run --bin mlg -- build examples/projects/hello -o target/mallang/project-hello
target/mallang/project-hello
cargo run --bin mlg -- run examples/projects/hello/mallang.toml
cargo run --bin mlg -- test examples/projects/hello
cargo run --bin mlg -- test examples/projects/hello --exact hello::GenericAndClosure
cargo run --bin mlg -- run examples/projects/local-deps/app
cargo run --bin mlg -- test examples/projects/local-deps/model
```

Run the full local gate:

```sh
scripts/check.sh
```

Run the complete v0.8 hardening acceptance, including release binary and deep
generated C sanitizer coverage:

```sh
scripts/check-v08-acceptance.sh
```

Run the heavier generated C sanitizer sweep before publication:

```sh
scripts/check-generated-c-sanitizers.sh
```

Run the complete local v0 release-candidate gate:

```sh
scripts/verify-v0-rc.sh
```

Run the lightweight release helper contract check:

```sh
scripts/check-release-helpers.sh
```

Run the release binary smoke:

```sh
scripts/check-release-binary.sh
```

Run the release archive and clean-prefix installation smoke:

```sh
scripts/check-release-artifacts.sh
```

For a future approved source release, run the finalizer with the release
message:

```sh
VERSION=0.8.0
scripts/finalize-and-push.sh --message "chore: publish mallang ${VERSION}"
```

The finalizer fetches `origin` before the expensive local verification and
again before the real push, prefers Homebrew Git when available, and fails if
the remote bookmark no longer matches the local bookmark base. After pushing,
it fetches again and verifies the remote bookmark points at the published
commit.

For publish-readiness verification without changing the jj description, moving
bookmarks, or pushing:

```sh
scripts/finalize-and-push.sh --verify-only
```

For a local dry run that also writes the final jj description and runs remote
freshness checks but still does not move bookmarks or push:

```sh
VERSION=0.8.0
scripts/finalize-and-push.sh --message "chore: publish mallang ${VERSION}" --no-push
```

## Canonical Project Workflow

Mallang does not yet provide a project generator. Create a minimal project with
the manifest and source tree below after installing `mlg`:

```text
hello/
|-- mallang.toml
|-- src/
|   `-- main.mlg
`-- tests/
    `-- main_test.mlg
```

```toml
[project]
name = "hello"
```

```mlg
package main

func main() {
    print(42)
}
```

```mlg
package main

test PrintsAnswer() {
    assert(40 + 2 == 42)
}
```

The canonical development-to-native path is:

```sh
mlg fmt hello
mlg fmt --check hello
mlg check hello
mlg test hello
mlg build hello -o hello-app
./hello-app
```

`scripts/check-v08-acceptance.sh` recreates this workflow from an empty work
directory with a local library dependency and an installed release artifact,
then runs the compiler hardening and sanitizer gates. It is the cross-platform
release acceptance used by CI.

## v0.8 Binary Distribution

The v0.8 GitHub Release contains:

- `mallang-v<version>-aarch64-apple-darwin.tar.gz`
- `mallang-v<version>-x86_64-unknown-linux-gnu.tar.gz`
- `SHA256SUMS`
- `install.sh`

Install or update the explicit version with:

```sh
curl -fsSLO https://github.com/zrma/mallang/releases/download/v0.8.0/install.sh
chmod +x install.sh
./install.sh --version 0.8.0
```

The default destination is `$HOME/.local/bin/mlg`; use `--bin-dir <directory>`
for another prefix. The installer requires `clang`, downloads only the detected
host archive over HTTPS, verifies its entry in `SHA256SUMS`, and atomically
replaces the destination binary. Re-running it with another explicit version is
the explicit update workflow.

Build and inspect the current native development artifact with:

```sh
scripts/build-release-artifact.sh --output-dir target/mallang/release-artifacts
python3 scripts/write-release-checksums.py \
  --output target/mallang/release-artifacts/SHA256SUMS \
  target/mallang/release-artifacts/*.tar.gz
```

Mallang is available under your choice of the MIT License or the Apache License,
Version 2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.

## Layout

- `SPEC.md`: published language and tooling contract through v0.8.
- `docs/STANDARD_LIBRARY.md`: implemented v0.6 standard package API and semantics.
- `docs/V1_ROADMAP.md`: `v0.2.0`부터 `v1.0.0`까지의 장기 milestone과 완료 조건.
- `docs/todo-v04-generic-data-model/`: approved and implemented v0.4 generic enum and specialization contract.
- `docs/todo-v05-ownership-runtime/`: approved v0.5 minimal ownership model and transparent recursive ADT contract.
- `docs/todo-v06-standard-library/`: approved v0.6 contract and completed P147-P153 acceptance evidence.
- `docs/todo-v07-tooling-platforms/`: approved v0.7 tooling/platform contract and P155-P160 implementation evidence.
- `docs/todo-v08-compiler-hardening/`: approved v0.8 compiler-hardening decision gate.
- `docs/todo-v09-language-freeze/`: approved v0.9 language-freeze and compatibility contract.
- `docs/releases/`: v0.1.0 through v0.8.0 release notes and verification records.
- `ROADMAP.md`: implementation milestones.
- `examples/hello.mlg`: first target source program.
- `examples/function-values.mlg`: native smoke for named function values,
  higher-order parameters, returns, and repeated indirect calls.
- `examples/closures.mlg`: native smoke for escaping plain closures with Copy
  and owned slice captures.
- `examples/mutable-closures.mlg`: native smoke for mutable Copy/slice/callable
  captures, isolated source state, and nested cleanup.
- `examples/nested-closures.mlg`: native smoke for nested plain/mutable closure
  environments, propagated captures, and independent owned state.
- `examples/generics.mlg`: native smoke for explicit generic struct/function
  specialization, concrete function values, slice type arguments, and mutable
  generic receiver cleanup.
- `examples/generic-enums.mlg`: native smoke for generic enum specialization,
  nested user/built-in patterns, and owned payload cleanup.
- `examples/standard-strings.mlg`: native smoke for UTF-8 byte/scalar behavior,
  search, split/join, strict conversion, standard errors, and intrinsic function
  values.
- `examples/process-io.mlg`: native smoke for arguments, environment, stdin,
  stdout, stderr, standard errors, and process exit behavior.
- `examples/file-io.mlg`: native smoke for function-valued UTF-8 file reads,
  create-or-overwrite writes, arguments, and recoverable file errors.
- `examples/projects/hello`: two-package project and native test smoke for imported functions,
  structs, generic APIs, receivers and enums, function values, higher-order APIs,
  closure returns, private package access, recursive ADTs, maps, and standard I/O.
- `examples/projects/local-deps`: local dependency workspace smoke for deterministic
  diamond discovery, library projects, public generic/recursive APIs, dependency
  entrypoint/test exclusion, and cross-project native tests.
- `tests/fixtures/invalid-closures`: CLI rejection fixtures for invalid capture,
  function move/alias, and recursive closure behavior.
- `tests/fixtures/invalid-generic-enums`: CLI rejection fixtures for generic enum
  constructor payload and nested exhaustiveness diagnostics.
- `examples/if.mlg`: native smoke for `if` expressions.
- `examples/int-division.mlg`: native smoke for guarded integer division and remainder.
- `examples/checked-arithmetic.mlg`: native smoke for checked integer arithmetic.
- `examples/if-statement.mlg`: native smoke for statement-form `if`.
- `examples/for-loop.mlg`: native smoke for condition-only `for` loops.
- `examples/loop-control.mlg`: native smoke for `break` and `continue`.
- `examples/for-clause.mlg`: native smoke for `for init; condition; post`.
- `examples/for-clause-initless.mlg`: native smoke for initless `for ; condition; post`.
- `examples/for-empty-condition.mlg`: native smoke for `for {}` and `for ; ; post`.
- `examples/range-blank.mlg`: native smoke for blank identifiers in array range loops.
- `examples/range-index.mlg`: native build/run smoke for one-variable array range over non-copy elements.
- `examples/shadowing.mlg`: native smoke for nested block shadowing, match payload shadowing, and outer move isolation.
- `examples/non-copy-array-assignment.mlg`: native smoke for replacing non-copy fixed array elements.
- `examples/for-clause-prelude.mlg`: native smoke for `for` clause condition/post preludes.
- `examples/string-equality.mlg`: native smoke for `string` equality without moving values.
- `examples/string-runtime.mlg`: native and sanitizer smoke for static/owned
  string value semantics, aggregate cleanup, mutable overwrite, and closure capture.
- `examples/borrow-range-contract.mlg`: native and sanitizer smoke for non-copy
  index-only range with direct indexed `con`/`mut` call access.
- `examples/allocation-accounting.mlg`: native accounting and sanitizer smoke
  for slice, closure, recursive enum, branch, loop, and overwrite cleanup.
- `examples/logical-operators.mlg`: native smoke for `bool` operators and short-circuiting.
- `examples/pipeline.mlg`: native smoke for `|>` pipeline call sugar.
- `examples/adt.mlg`: native smoke for `Option` / `Result` constructors and `match`.
- `examples/print-adt.mlg`: native smoke for printing `Option` / `Result` values.
- `examples/match-temp.mlg`: native smoke for expression scrutinees in `match`.
- `examples/if-match-expression.mlg`: native smoke for `if` expression branches that need C preludes.
- `examples/match-arm-prelude.mlg`: native smoke for `match` expression arms that need C preludes.
- `examples/full-expression-cleanup.mlg`: native and sanitizer smoke for temporary cleanup across calls, conditions, indexing, range loops, early return, and short-circuit evaluation.
- `examples/slice-field-read.mlg`: native smoke for local-rooted slice field len/index/range/borrow reads.
- `examples/slice-field-assignment.mlg`: native smoke for local-rooted slice field element assignment.
- `examples/slice-field-append.mlg`: native smoke for direct slice field append reassignment.
- `examples/indexed-slice-field-append.mlg`: native smoke for indexed slice field append reassignment.
- `examples/slice-field-take-append.mlg`: native smoke for taking slice field append sources.
- `examples/slice-field-take.mlg`: native smoke for owned slice field take expressions.
- `examples/structs.mlg`: native smoke for struct declarations, literals, and field access.
- `examples/print-struct.mlg`: native smoke for printing struct values with nested fields.
- `examples/methods.mlg`: native smoke for struct receiver methods.
- `examples/mut-receiver.mlg`: native smoke for caller-visible `mut` receiver methods.
- `examples/field-assignment.mlg`: native smoke for mutable struct field assignment.
- `examples/field-borrow.mlg`: native smoke for direct field borrow arguments.
- `examples/array-element-borrow.mlg`: native smoke for fixed array element borrow arguments.
- `examples/slice-element-borrow.mlg`: native smoke for owned slice element borrow arguments.
- `examples/slice-element-assignment.mlg`: native smoke for owned slice element assignment.
- `examples/indexed-field-assignment.mlg`: native smoke for indexed array/slice field assignment.
- `examples/indexed-field-read.mlg`: native smoke for borrowed indexed array/slice field reads.
- `examples/struct-slice-field.mlg`: native smoke for struct cleanup with owned slice fields.
- `examples/array-element-methods.mlg`: native smoke for fixed array element method receivers.
- `examples/mut-parameter-abi.mlg`: native smoke for caller-visible `mut` parameter mutation.
- `examples/nested-fields.mlg`: native smoke for nested field assignment and borrow arguments.
- `examples/return-completeness.mlg`: native smoke for branch-aware return analysis.
- `examples/else-if.mlg`: native smoke for statement-form `else if` sugar.
- `examples/match-statement.mlg`: native smoke for statement-form `match` block arms.
- `src/lexer.rs`: initial hand-written lexer.
- `src/parser.rs`: AST parser for the current v0 subset.
- `src/project.rs`: manifest parsing and deterministic project source discovery.
- `src/package.rs`: package declaration tables, imports, cycles, and build order.
- `src/linker.rs`: cross-package resolution, visibility, and internal symbols.
- `src/semantic.rs`: semantic checker for name/type/function diagnostics and
  reserved-feature boundaries.
- `src/specialize.rs`: demand-driven concrete specialization for generic
  structs and functions.
- `src/ir.rs`: typed IR lowering after semantic analysis.
- `src/backend/mod.rs`: backend public API boundary.
- `src/backend/c.rs`: C backend for typed IR in the first native subset.
- `src/backend/c/names.rs`: C backend identifier, type-name, and operator helpers.
- `src/backend/c/runtime.rs`: common allocation accounting/failure helpers and
  conditionally emitted string layout, validation, equality, and print runtime.
- `src/backend/c/expressions.rs`: C backend expression, literal, call,
  borrow-lvalue, and expression-match emission.
- `src/backend/c/statements.rs`: C backend statement, loop, match, and print emission.
- `src/backend/c/types.rs`: C backend type layout and drop helper emission.
- `src/backend/c/utils.rs`: C backend formatting, temp-name, and parameter-env helpers.
- `src/backend/c/tests.rs`: C backend unit tests.
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
guarded integer division/remainder, checked integer arithmetic, `bool` operators, `|>` pipeline call sugar, statement/expression `if`,
condition-only, conditionless, and `for init; condition; post` loops with
`break` / `continue`, `else if` sugar, branch-aware returns,
struct/method/nested-field, nested block shadowing, recursive struct value-type rejection, semantic printability checks, struct print output, and built-in ADT
expression/statement `match` plus ADT print output via C source generation and
`clang`.
