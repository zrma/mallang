# Spec: B3 Self-Hosting C Backend

Status: active; P177a and P177b1-P177b3 complete

## Objective

B3 moves C generation behind the Mallang compiler core without claiming a
Stage1-to-Stage2 fixed point. Rust Stage0 remains the byte-level oracle while
the Mallang backend grows through small typed-IR slices.

## Work Breakdown

### P177a: Scalar Backend Foundation

- expose a read-only Mallang API over function, parameter, statement,
  expression, type, span and child metadata in the typed IR arena
- add a standalone `c` host mode that runs parse, prepare, semantic check,
  typed-IR lowering and Mallang C generation
- emit deterministic C for `int`, `bool`, `unit`, owned scalar parameters,
  direct calls, local bindings, assignment, return, `print`, unary `!`, checked
  arithmetic, comparisons and logical operators
- compare generated C byte-for-byte with Rust Stage0
- compile with strict C11, run exact native output, allocation accounting and
  ASan/UBSan

P177a is complete. The scalar fixture prints `30` and `true`; Stage0 and Stage1
generate byte-identical C, and two Stage1 emissions are deterministic. The
compiler source set also matches Stage0 for all 713 normalized typed-IR
functions.

### P177b: Owned Values And Control Flow

- add strings, arrays, slices, structs, inline and owned ADTs
- add `if`, statement and expression `match`, loops and explicit cleanup nodes
- preserve evaluation order, checked runtime failures and ownership cleanup
- add positive, rejection, native, accounting and sanitizer fixtures per slice

P177b1 is complete. The Mallang backend now emits the string runtime, static
UTF-8 literals, owned string function returns and locals, read-only string
printing/equality, full-expression temporary cleanup and explicit string drops.
Statement `if`/`else`, condition loops, `break` and `continue` share the same
recursive block emitter. The `owned-control` fixture combines these paths,
including non-ASCII and equality literals, and matches Stage0 C byte-for-byte
before strict native, allocation-accounting and ASan/UBSan execution. The
expanded compiler source also matches Stage0 across 725 normalized typed-IR
functions.

P177b2 is complete. Semantic type shapes and specialized struct/enum
declarations are preserved as read-only IR metadata instead of being reparsed
from display strings. The Mallang backend uses that metadata to emit fixed
arrays, heap-backed slices, structs, nested type definitions and recursive drop
helpers. Array/slice literals, struct literals, field access, checked indexing
and `len` preserve Stage0 evaluation and cleanup order. The
`composite-values` fixture matches Stage0 C byte-for-byte before strict native,
allocation-accounting and ASan/UBSan execution. A dynamic bounds failure and the
still-unsupported `SliceAppend` node are deterministic rejection fixtures. The
expanded compiler source matches Stage0 across 806 normalized typed-IR
functions.

P177b3 is complete. Built-in `Option`/`Result`, inline user enums and recursive
owned enums now share metadata-driven C definitions, constructor lowering and
drop helpers. Expression `if`, statement/expression `match`, recursive pattern
planning and span-stable binding environments preserve Stage0 evaluation and
cleanup order. The `adt-match` fixture executes single/multi-payload variants,
nested inline and owned patterns, recursive node consumption and pattern-local
string cleanup. A forced recursive-enum allocation failure and the still-open
`RangeFor` node are deterministic rejection paths. All four positive, two
runtime-rejection and two boundary-rejection paths pass byte parity, strict C,
native, accounting and sanitizer checks where applicable. Existing recursive,
nested-match and branch-prelude examples also match Stage0 C byte-for-byte. The
compiler source matches Stage0 across 831 normalized typed-IR functions.

P177b remains active. Range and three-clause loops, overwrite cleanup, slice
append and dynamic owned-string construction remain in later slices.

### P177c: Callable And Project Surface

- add methods, function values, closures, captures and indirect calls
- add package-qualified symbols, standard intrinsics and specialization output
- compare representative multi-file project C and native behavior with Stage0

### P177d: B3 Closure

- cover every typed-IR node required by the Mallang compiler source set
- move deterministic project compilation behind the declared host boundary
- pass the complete B3 differential, strict C, accounting, sanitizer and
  canonical repository gates
- document the remaining host operations that B4 must move behind Stage1

## Development Loop

The validation layers are deliberately asymmetric so feature work does not pay
the publication cost on every edit.

1. Edit loop: run focused Rust/Mallang tests and
   `scripts/check-self-hosting-backend.sh --assume-bootstrap`. This reuses an
   explicitly existing Stage1 and is not milestone evidence.
2. Integration loop: run `scripts/check-self-hosting-backend.sh`. This rebuilds
   Stage1 from current sources before the complete tracked backend fixture gate.
3. Compiler-core differential: run
   `scripts/diagnose-self-hosting-compiler-ir.sh --rebuild-bootstrap` after IR,
   ownership or cleanup changes. `--reuse-bootstrap` is diagnostic only.
4. Publication loop: run the argument-free self-hosting gate through
   `scripts/check.sh`; the backend slice reuses the fresh Stage1 produced by the
   preceding compiler gate.

On one development host, the four-positive/four-rejection P177b3 reuse gate took
about ten seconds, the fresh backend gate about twenty-two to twenty-five
seconds and a fresh compiler-source IR comparison about twenty-four seconds.
These observations justify the layer split; they are not portable performance
thresholds.

## Acceptance

- [x] read-only typed-IR backend API
- [x] standalone Mallang `c` host mode
- [x] deterministic scalar C emitter
- [x] byte-identical Stage0/Stage1 scalar C
- [x] strict native, accounting and ASan/UBSan scalar gate
- [x] compiler-source Stage0/Stage1 typed-IR parity after cleanup regression fix
- [x] P177a sub-two-second artifact-reuse edit loop on the observed host
- [x] P177b1 owned strings, statement control flow and explicit cleanup
- [x] two-fixture byte parity, native, accounting and sanitizer gate
- [x] P177b2 array, slice and struct layout, expressions and recursive cleanup
- [x] three positive, one runtime-rejection and one boundary-rejection fixtures
- [x] P177b3 inline/owned ADTs, expression `if` and statement/expression `match`
- [x] four positive, two runtime-rejection and two boundary-rejection fixtures
- [ ] remaining loop, overwrite, append and dynamic-string owned values
- [ ] callable, specialization and project surface
- [ ] complete compiler-source C generation
- [ ] B3 canonical, publication and supported-platform CI acceptance

## Excluded

- Stage1 compiling the Mallang compiler into Stage2, which belongs to B4
- changing the frozen v1 syntax or ownership model to shorten the backend
- treating native executable bytes as deterministic compiler output
- deleting the Rust bootstrap seed or differential oracle
