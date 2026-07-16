# Mallang v1 Candidate Language Contract

Status: candidate freeze inventory after v0.8.0; not yet the stable v1 contract

This document indexes the source, standard-library, CLI, diagnostic, runtime,
and distribution behavior proposed for Mallang v1. Detailed syntax examples and
edge-case wording remain in `SPEC.md` and `docs/STANDARD_LIBRARY.md`. If those
documents disagree with a rule here during v0.9, the disagreement is a freeze
blocker and must be resolved before release.

The words MUST, MUST NOT, SHOULD, and MAY are normative. Rule IDs are permanent:
a rule may be clarified, deprecated, or superseded, but its ID is not reused.
P169 maps every rule to executable or explicit verification evidence.

## Source and lexical contract

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-SRC-001` | Mallang source files MUST use the `.mlg` extension. The user-facing compiler command MUST be `mlg`. | `SPEC.md` Naming |
| `V1-SRC-002` | An executable entrypoint MUST be exactly `func main()` with no receiver, parameters, or explicit return type. | `SPEC.md` Source Shape |
| `V1-SRC-003` | A source program MUST NOT expose raw pointers, address-of operations, `nil`, user-visible lifetimes, first-class references, or an unsafe block. | `SPEC.md` Design Goals |
| `V1-SRC-004` | Project and standalone inputs MUST share the same source-language semantics; selecting a direct `.mlg` file MUST remain manifest-free standalone mode. | `SPEC.md` Project Model |
| `V1-LEX-001` | Identifiers MUST use ASCII letters, digits, and `_`, and MUST NOT begin with a digit. | `SPEC.md` Lexical Rules |
| `V1-LEX-002` | Line comments MUST begin with `//`; strings MUST use double quotes; integer literals MUST be base 10. | `SPEC.md` Lexical Rules |
| `V1-LEX-003` | The keyword and reserved built-in name sets in `SPEC.md` MUST remain unavailable for conflicting source declarations or bindings. | `SPEC.md` Lexical Rules, Source Formatting |
| `V1-LEX-004` | Integer arithmetic MUST reject statically evident zero division and overflow, and native dynamic violations MUST terminate with a Mallang runtime error. | `SPEC.md` Lexical Rules |
| `V1-LEX-005` | `&&` and `||` MUST evaluate left to right and MUST short-circuit the right operand. | `SPEC.md` Lexical Rules |

## Project and package contract

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-PRJ-001` | A project MUST use `mallang.toml`, a lowercase project path name, and production sources under `src/`. | `SPEC.md` Project Model |
| `V1-PRJ-002` | Every project source MUST declare `package <name>`; files in one directory MUST use one package name. | `SPEC.md` Project Model |
| `V1-PRJ-003` | `import "project/path"` MUST bind the final path segment as the qualifier. Import aliases, dot imports, wildcard imports, and remote imports MUST NOT be accepted. | `SPEC.md` Project Model |
| `V1-PRJ-004` | Top-level declarations MUST be package-private unless marked `pub`; a public API MUST NOT expose a private type. | `SPEC.md` Project Model |
| `V1-PRJ-005` | Relative `[dependencies]` entries MUST use exact project names, remain within their declared source roots, load in deterministic dependency-first order, and reject cycles and undeclared transitive imports. | `SPEC.md` Local Path Dependencies |
| `V1-PRJ-006` | Library projects MAY omit `src/main.mlg`; format, check, and test MUST work, while build and run MUST report a missing executable entrypoint. | `SPEC.md` Local Path Dependencies |
| `V1-PRJ-007` | Source discovery, declaration processing, test discovery, and package graph traversal MUST be deterministic. | `SPEC.md` Project Model, Test Workflow |

## Declarations and types

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-TYP-001` | Primitive types MUST be `int`, `bool`, `string`, and `unit`. `int`, `bool`, and `unit` are Copy; `string` is immutable UTF-8 and move-only. | `SPEC.md` Types |
| `V1-TYP-002` | Fixed arrays MUST use `[N]T`, require a non-negative literal length and exactly `N` literal elements, and remain move-only regardless of element type. | `SPEC.md` Types |
| `V1-TYP-003` | Owned slices MUST use `[]T`, remain move-only and non-aliasing at the language level, and support literals, `len`, checked indexing, assignment, range, and consuming `append`. | `SPEC.md` Types |
| `V1-TYP-004` | Structs MUST use named fields and named-field literals. Every field MUST appear exactly once; struct values MUST remain move-only. | `SPEC.md` Structs |
| `V1-TYP-005` | User enums MUST support zero or more positional payloads and qualified constructors. User enums are move-only; the compiler-owned `errors.Kind` is the documented Copy exception. | `SPEC.md` Option and Result, Match |
| `V1-TYP-006` | `Option[T]` and `Result[T,E]` MUST behave as built-in generic algebraic data types with conditional Copy classification based on their payloads. | `SPEC.md` Option and Result |
| `V1-TYP-007` | Generic structs, enums, functions, and concrete receiver methods MUST use explicit type parameters and statically specialized concrete instantiations. | `SPEC.md` Generic Data Model |
| `V1-TYP-008` | Function types MUST include parameter modes and an explicit return type; function values and closures MUST remain move-only. | `SPEC.md` Function Values and Closures |
| `V1-TYP-009` | Productive recursive user enums MAY use compiler-owned indirection. Recursive by-value struct cycles and non-productive recursive type cycles MUST be rejected. | `SPEC.md` Structs, Option and Result |

## Bindings, functions, and closures

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-FUN-001` | Bindings MUST be immutable by default; reassignment MUST require `mut`; same-block redeclaration MUST fail while nested shadowing remains valid. | `SPEC.md` Bindings |
| `V1-FUN-002` | Parameter and receiver modes MUST be owned (`name T`), shared call borrow (`con name T`), or exclusive call borrow (`mut name T`). Legacy `in` and suffix mode spellings MUST fail. | `SPEC.md` Functions |
| `V1-FUN-003` | Call sites MUST write `con expr` or `mut expr` for borrowed arguments. Those forms MUST NOT be accepted outside a direct call argument. | `SPEC.md` Functions |
| `V1-FUN-004` | Non-`unit` functions MUST return on every path the checker can prove; `unit` MUST remain the implicit no-value return type. | `SPEC.md` Functions |
| `V1-FUN-005` | Methods MUST use concrete same-package struct receivers with owned, `con`, or `mut` mode. Interfaces, dynamic dispatch, and method values MUST NOT be part of v1. | `SPEC.md` Structs, Project Model |
| `V1-FUN-006` | Named functions, package-qualified named functions, and closure literals MUST be usable as values and through indirect calls. | `SPEC.md` Function Values and Closures |
| `V1-FUN-007` | Closures MUST capture free locals by owned value: Copy values copy and non-Copy values move into a compiler-owned environment. | `SPEC.md` Function Values and Closures |
| `V1-FUN-008` | Plain closures MUST not mutate captures. `func mut` closures MUST require exclusive call access and MAY mutate captures originating from mutable bindings. | `SPEC.md` Function Values and Closures |
| `V1-FUN-009` | Borrowed captures, explicit capture lists, recursive local closures, and borrowed callable returns MUST NOT be part of v1. | `SPEC.md` Function Values and Closures |

## Expressions and control flow

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-EXP-001` | Operators MUST type-check only the operand families documented in `SPEC.md`; string equality MUST compare contents without moving either operand. | `SPEC.md` Lexical Rules |
| `V1-EXP-002` | `if` MUST support statement and value forms. A value form MUST have `else`, produce one value from each branch, and use one result type. | `SPEC.md` Expressions |
| `V1-EXP-003` | `value |> f(args...)` MUST be equivalent to the owned call `f(value, args...)`; borrowed pipeline arguments MUST NOT be part of v1. | `SPEC.md` Expressions |
| `V1-EXP-004` | `print(expr)` MUST be statement-only and MUST reject non-printable types. | `SPEC.md` Built-in Statements |
| `V1-EXP-005` | Array and slice indexing MUST evaluate the index before the right-hand assignment value and MUST guard every runtime access before touching memory. | `SPEC.md` Types |
| `V1-EXP-006` | A computed cleanup-valued expression MUST remain owned by a full-expression temporary through its final read or call and MUST then be dropped exactly once on normal flow. | `SPEC.md` Types |
| `V1-CTL-001` | `for` MUST support condition-only, conditionless, and three-clause forms with optional init and condition and one assignment post clause. | `SPEC.md` Expressions |
| `V1-CTL-002` | `continue` in a three-clause loop MUST execute the post clause before the next condition; `break` and `continue` MUST target the nearest loop. | `SPEC.md` Expressions |
| `V1-CTL-003` | Range MUST support arrays and owned slices, one index binding or index/value bindings, `_`, and Copy-only element value bindings. | `SPEC.md` Expressions |
| `V1-CTL-004` | Range bindings MUST NOT accept `con` or `mut`. Non-Copy traversal MUST use index-only range plus call-scoped indexed borrows or mutation. | `SPEC.md` Expressions |
| `V1-CTL-005` | `match` MUST evaluate its scrutinee once, support nested qualified ADT patterns and wildcard payloads, reject unreachable arms, and require exhaustiveness. | `SPEC.md` Match |
| `V1-CTL-006` | Every `match` expression arm MUST produce the same non-`unit` type; statement match return completeness MUST require every arm to return. | `SPEC.md` Match |
| `V1-CTL-007` | Pattern guards, borrowed patterns, map range, and string range MUST NOT be part of v1. | `SPEC.md` Match, Expressions |

## Ownership and runtime safety

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-OWN-001` | Every non-Copy value MUST have exactly one owner; an owned read, assignment, argument, return, match, or capture MUST transfer ownership. | `SPEC.md` Ownership Rules |
| `V1-OWN-002` | A moved binding or place MUST NOT be read, borrowed, assigned through, or moved again. | `SPEC.md` Ownership Rules |
| `V1-OWN-003` | General partial field moves MUST fail. Taking an owned slice field MAY transfer its buffer only by resetting the source field to an empty slice. | `SPEC.md` Types, Structs |
| `V1-OWN-004` | A `con` borrow MUST be read-only and call-scoped. A `mut` borrow MUST be exclusive, call-scoped, and require a mutable root. | `SPEC.md` Functions, Ownership Rules |
| `V1-OWN-005` | Overlapping shared borrows MAY coexist in one call. Any overlapping exclusive borrow MUST conflict; disjoint struct fields MAY be borrowed independently, while indexed places with one root MUST overlap conservatively. | `SPEC.md` Ownership Rules |
| `V1-OWN-006` | A borrowed non-Copy value MUST NOT move, escape, return, store, or enter a closure environment. Borrowed Copy values MAY copy out. | `SPEC.md` Functions, Ownership Rules |
| `V1-OWN-007` | Overwrite MUST evaluate the replacement first, evaluate the destination place once, drop the old owner, and then store the replacement. | `SPEC.md` Ownership Rules |
| `V1-OWN-008` | At a control-flow merge, a value moved on any reachable incoming path MUST remain unavailable. A loop-persistent move-only value MUST NOT move on a repeatable path. | `SPEC.md` Ownership Rules |
| `V1-OWN-009` | A return value MUST be transferred to caller-owned storage before remaining callee locals are dropped. | `SPEC.md` Ownership Rules |
| `V1-OWN-010` | Owned strings, slices, aggregates, ADTs, closures, maps, and recursive nodes MUST be dropped exactly once on normal control flow. | `SPEC.md` Types, Ownership Rules; `docs/STANDARD_LIBRARY.md` |
| `V1-OWN-011` | First-class references, borrowed returns, statement-spanning borrows, shared backing buffers, implicit cloning, and a garbage collector MUST NOT be part of v1. | `SPEC.md` Ownership Rules |

## Standard library

The signatures in `docs/STANDARD_LIBRARY.md` are part of each rule below and
MUST NOT be changed during the freeze except to correct a soundness defect or a
contradiction with an already implemented v1 rule.

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-STD-001` | The compiler MUST provide `std/errors`, `std/strings`, `std/os`, `std/io`, `std/fs`, and `std/collections` as the v1 baseline compiler-versioned packages. Compatible 1.x additions remain governed by `V1-COMP-007`. | `docs/STANDARD_LIBRARY.md` |
| `V1-STD-002` | Standard APIs MUST expose no native pointer, allocator, handle, borrowed return, or platform numeric error code. Recoverable failures MUST use `Result[..., errors.Error]`. | `docs/STANDARD_LIBRARY.md` Imports and ownership |
| `V1-STD-003` | `string` and all text APIs MUST preserve valid UTF-8 semantics, documented byte-versus-scalar indexing, exact embedded-NUL behavior, and strict parse behavior. | `docs/STANDARD_LIBRARY.md` strings, io, fs |
| `V1-STD-004` | `std/os` MUST expose arguments, environment lookup, and process exit with the documented UTF-8, missing-value, NUL, and `0..255` behavior. | `docs/STANDARD_LIBRARY.md` os |
| `V1-STD-005` | `std/io` and `std/fs` MUST use length-based exact I/O and MUST return recoverable read, write, flush, open, and close failures. | `docs/STANDARD_LIBRARY.md` io, fs |
| `V1-STD-006` | `collections.Map[K,V]` MUST remain opaque and move-only; keys MUST be concrete `int`, `bool`, or `string`, with value-based deterministic hash and equality. | `docs/STANDARD_LIBRARY.md` collections |
| `V1-STD-007` | Map insertion, lookup callbacks, update callbacks, removal, replacement, and drop MUST preserve the exact ownership transfer and call-scoped borrow rules in the reference. | `docs/STANDARD_LIBRARY.md` collections |
| `V1-STD-008` | `?`, exceptions, implicit process exit, stack unwinding, networking, async I/O, public FFI, and long-lived native handles MUST NOT be part of v1. | `docs/STANDARD_LIBRARY.md` Error flow, Supported native acceptance |

## CLI, diagnostics, and developer workflow

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-CLI-001` | `mlg fmt`, `check`, `build`, `run`, and `test`, plus `--version` and `--help`, MUST remain the stable user workflow. | `SPEC.md` Naming; `README.md` |
| `V1-CLI-002` | `mlg lex`, `parse`, and `ir` MUST remain available inspection commands, but their successful textual stdout is not a v1 serialization format. Their diagnostics and exit behavior remain governed by the diagnostic rules. | `src/main.rs`; `SPEC.md` Naming |
| `V1-CLI-003` | `mlg fmt` MUST be deterministic, comment-preserving, idempotent, and all-or-nothing for a selected project. `fmt --check` MUST perform no writes. | `SPEC.md` Source Formatting |
| `V1-CLI-004` | `mlg test` MUST preflight the complete deterministic suite, run selected tests in isolated native children, continue after failures, and return non-zero when any selected test fails. | `SPEC.md` Project Test Workflow |
| `V1-CLI-005` | `mlg build` MUST emit a native executable through the supported C11 host path. `mlg run` MUST forward arguments after `--` and propagate the program's numeric exit status. | `SPEC.md` Standard Library, Canonical Acceptance |
| `V1-CLI-006` | Compiler usage and validation errors MUST exit non-zero; an invocation with no arguments MUST use exit 2 for usage, while ordinary compiler diagnostics use exit 1. | `src/main.rs`; release binary checks |
| `V1-CLI-007` | `mlg [--diagnostic-format <human|json>] <subcommand>` and the `=value` spelling MUST select human or JSON diagnostics without changing successful stdout or status. | `SPEC.md` Machine-readable Diagnostics |
| `V1-CLI-008` | Installed release binaries MUST report exactly `mlg <package-version>` and MUST support standalone and project format/check/test/build/run workflows. | `SPEC.md` Release Artifacts, Canonical Acceptance |
| `V1-DIAG-001` | Compiler-owned JSON diagnostics MUST use schema `mallang.diagnostic.v1` and one JSON object per line on stderr. | `SPEC.md` Machine-readable Diagnostics |
| `V1-DIAG-002` | Human and JSON rendering MUST describe the same severity, stage, message, source identity, and optional span. | `SPEC.md` Machine-readable Diagnostics |
| `V1-DIAG-003` | Source spans MUST use UTF-8 byte offsets and 1-based line/Unicode-scalar columns. Project paths MUST be root-relative with dependency ownership identified. | `SPEC.md` Machine-readable Diagnostics |
| `V1-DIAG-004` | Parser recovery MUST be deterministic, MUST emit no later-stage output after a frontend error, and MUST cap source diagnostics at 32 while preserving first-error library APIs. | `SPEC.md` Compiler Hardening |

## Native runtime and distribution

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-RUN-001` | The supported v1 native targets MUST be macOS arm64 and Linux x86_64 through a generated C11 program compiled by `clang`. | `docs/STANDARD_LIBRARY.md`; `.github/workflows/ci.yml` |
| `V1-RUN-002` | Bounds errors, arithmetic violations, allocation failure or overflow, malformed compiler-owned storage, invalid fatal process operations, and internal accounting failures MUST terminate with a stable Mallang runtime error and MUST NOT execute unchecked C behavior. | `SPEC.md` Types, Ownership Rules |
| `V1-RUN-003` | Fatal runtime errors MUST use no-unwind process termination; pending cleanup and zero live-allocation accounting are promised only for normal flow. | `SPEC.md` Types; `docs/STANDARD_LIBRARY.md` |
| `V1-RUN-004` | Generated C and release archives MUST be byte-identical for the same compiler, input, options, and documented host scope. Native executable byte identity MUST NOT be promised. | `SPEC.md` Compiler Hardening |
| `V1-RUN-005` | A release MUST provide target-named archives, `install.sh`, and `SHA256SUMS`. Each archive MUST contain `bin/mlg`, README, MIT license, and Apache-2.0 license. | `SPEC.md` Release Artifacts and Installation |
| `V1-RUN-006` | The installer MUST require an explicit version, verify checksum, archive entries, and staged binary version, and atomically replace only the destination `mlg`. | `SPEC.md` Release Artifacts and Installation |
| `V1-RUN-007` | The C backend, compiler allocation counters, failure-injection controls, generated symbols, and native layouts MUST remain implementation details rather than stable source or native ABI. | `SPEC.md` Backend Strategy |

## Versioning and compatibility

`docs/COMPATIBILITY.md` is the detail owner for the following rules.

| Rule | Normative requirement | Detail owner |
| --- | --- | --- |
| `V1-COMP-001` | One semantic version MUST identify the compiler, implemented language contract, compiler-owned standard packages, installer, and native archives. | `docs/COMPATIBILITY.md` Version model |
| `V1-COMP-002` | v1.0.0 MUST be the first stable implementation of Mallang v1; every v1.x compiler MUST implement that same language version. | `docs/COMPATIBILITY.md` Version model |
| `V1-COMP-003` | A later v1.x compiler MUST continue to accept every valid v1 program. | `docs/COMPATIBILITY.md` 1.x guarantees |
| `V1-COMP-004` | A later v1.x compiler MUST NOT silently change a valid v1 program's observable semantics. | `docs/COMPATIBILITY.md` Compatibility unit |
| `V1-COMP-005` | Stable standard-library signatures, ownership/failure behavior, user CLI meanings, diagnostic schema, archive shape, and installer verification MUST remain compatible throughout v1.x. | `docs/COMPATIBILITY.md` 1.x guarantees |
| `V1-COMP-006` | Patch releases MUST preserve valid source and observable semantics except for the documented soundness and security exception. | `docs/COMPATIBILITY.md` Release classes |
| `V1-COMP-007` | Minor releases MAY add only backward-compatible language, library, tooling, diagnostic, or target surface. | `docs/COMPATIBILITY.md` Release classes |
| `V1-COMP-008` | A source rejection, semantic change, stable API/CLI removal, or supported-target removal MUST require a new major version unless the soundness exception applies. | `docs/COMPATIBILITY.md` Release classes |
| `V1-COMP-009` | Deprecation MUST preserve source through v1.x, identify a replacement and major-version removal, and SHOULD provide at least one minor release of compiler diagnostic notice when practical. | `docs/COMPATIBILITY.md` Deprecation |
| `V1-COMP-010` | A soundness or security compatibility exception MUST be narrow, rule-identified, regression-tested, migrated, and release-noted. | `docs/COMPATIBILITY.md` Soundness and security exception |
| `V1-COMP-011` | Rule IDs MUST NOT be recycled; clarification, deprecation, and supersession MUST preserve the original ID history. | `docs/COMPATIBILITY.md` Deprecation |
| `V1-COMP-012` | Mallang v1 MUST NOT add an edition, manifest language-version field, source pragma, or per-project compatibility switch. | `docs/COMPATIBILITY.md` Version model |
| `V1-COMP-013` | Exact human diagnostic wording, inspection stdout, generated C/native ABI, cross-version artifact bytes, and numerical performance MUST NOT be compatibility guarantees. | `docs/COMPATIBILITY.md` 1.x guarantees |

## Explicit v1 exclusions

The following are outside this contract: goroutines, async functions, coroutines,
interfaces, dynamic dispatch, raw pointers, user-visible FFI, unsafe blocks,
first-class references, borrowed returns, user-visible lifetimes, statement-
spanning borrows, garbage collection, runtime reflection, a centralized package
registry, remote dependencies, cross compilation, Windows support, and a stable
generated C or native binary ABI.

Performance observations in `docs/baselines/v0.8-performance.json` are evidence,
not numerical v1 performance guarantees.
