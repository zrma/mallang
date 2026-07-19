# Spec: B3 Self-Hosting C Backend

Status: active; P177a complete

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
   Stage1 from current sources before the complete scalar backend gate.
3. Compiler-core differential: run
   `scripts/diagnose-self-hosting-compiler-ir.sh --rebuild-bootstrap` after IR,
   ownership or cleanup changes. `--reuse-bootstrap` is diagnostic only.
4. Publication loop: run the argument-free self-hosting gate through
   `scripts/check.sh`; the backend slice reuses the fresh Stage1 produced by the
   preceding compiler gate.

On one development host, the P177a reuse gate took about two seconds, the fresh
backend gate about fourteen seconds and a complete compiler-source IR reuse
comparison about eleven seconds. These observations justify the layer split;
they are not portable performance thresholds.

## Acceptance

- [x] read-only typed-IR backend API
- [x] standalone Mallang `c` host mode
- [x] deterministic scalar C emitter
- [x] byte-identical Stage0/Stage1 scalar C
- [x] strict native, accounting and ASan/UBSan scalar gate
- [x] compiler-source Stage0/Stage1 typed-IR parity after cleanup regression fix
- [x] sub-two-second artifact-reuse edit loop on the observed host
- [ ] owned values and control flow
- [ ] callable, specialization and project surface
- [ ] complete compiler-source C generation
- [ ] B3 canonical, publication and supported-platform CI acceptance

## Excluded

- Stage1 compiling the Mallang compiler into Stage2, which belongs to B4
- changing the frozen v1 syntax or ownership model to shorten the backend
- treating native executable bytes as deterministic compiler output
- deleting the Rust bootstrap seed or differential oracle
