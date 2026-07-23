# Mallang Self-Hosting

Status: active long-term program; B0-B3 complete, B4 active

## Objective

Mallang will become capable of compiling its own compiler without removing the
auditable Rust bootstrap seed. The transition is complete only after a Mallang
compiler built by the Rust implementation can rebuild the same Mallang compiler
and pass the repository conformance gates.

## Stage Definitions

- **Stage0**: the Rust `mlg` implementation built from this repository.
- **Stage1**: the native Mallang compiler built from the compiler's Mallang
  sources by Stage0.
- **Stage2**: the native Mallang compiler built from the same Mallang sources by
  Stage1.

The Rust implementation remains the trusted bootstrap seed and differential
oracle through the transition. Self-hosting does not require deleting it.

## Fixed-Point Contract

B4 is complete when all of the following hold on every supported platform:

1. Stage0 builds Stage1 from the declared compiler source set.
2. Stage1 builds Stage2 from the identical source set and declared inputs.
3. Stage1 and Stage2 generate byte-identical C for the compiler source set.
4. Stage1 and Stage2 produce equivalent diagnostics and runtime output for the
   complete conformance, positive, rejection and crash-corpus suites.
5. Generated C remains warning-clean and the relevant native programs pass the
   existing ASan/UBSan and allocation-accounting gates.

Native executable bytes are not fixed-point evidence because the host linker,
paths and platform metadata are outside Mallang's stable output contract.

## Host Boundary

The first compiler core owns source processing, lexical analysis, parsing,
semantic and ownership checking, typed IR, specialization and C generation. A
small host driver may initially provide deterministic source discovery, invoke
the compiler core and run `clang`. Project discovery, diagnostics, formatting,
testing and process execution move behind the Mallang compiler before B5.

No public language or standard-library feature is added merely because it would
make a port shorter. A missing capability must first block representative
Mallang compiler code, have a narrow compatibility classification and receive
positive, rejection and runtime evidence.

## Program

| Stage | Scope | Completion evidence |
| --- | --- | --- |
| B0 | Bootstrap contract and feasibility | Stage0 builds and runs the tracked Mallang bootstrap probe; same-input C is deterministic |
| B1 | Mallang lexer and parser | Differential token, AST and diagnostic corpus matches the Rust frontend |
| B2 | Semantic checker and typed IR | Positive/rejection ownership and type suites agree with Stage0 |
| B3 | C backend and host boundary | Representative projects build and run through the Mallang compiler core |
| B4 | Stage1 to Stage2 fixed point | Compiler C output and conformance behavior meet the fixed-point contract |
| B5 | Default transition | Mallang compiler is the default release path; Rust Stage0 remains a documented seed and oracle |

Version numbers are assigned only when a stage produces a release-worthy,
backward-compatible capability. The self-hosting schedule does not pre-commit
Mallang to specific 1.x versions.

## Current Layout

- `bootstrap/probe/`: B0 Mallang capability probe compiled by Stage0.
- `bootstrap/compiler/`: active Mallang compiler source, currently containing
  the complete lexer/parser, semantic and ownership checker, specialization,
  project package graph/linker, typed IR and the first scalar C backend slice.
- `scripts/check-self-hosting-bootstrap.sh`: current bootstrap gate.
- `scripts/check-self-hosting-lexer.sh`: deterministic Rust/Mallang lexer and
  parser differential plus incremental B2 semantic differential, ownership
  accounting and sanitizer gate. The argument-free command remains the full
  milestone gate; `--fast` keeps complete Stage0/Stage1 differential coverage,
  runs the complete compiler project test suite once, and retains focused
  accounting and representative sanitizer smoke. `--focus <area>` keeps
  edit loops to representative tests, differentials and one sanitizer path;
  `--jobs` controls bounded concurrency. The historical filename remains stable
  while the compiler gate grows through B2.
- `scripts/check-self-hosting-backend.sh`: B3 Stage0/Stage1 generated-C
  identity, determinism, strict native, allocation-accounting and sanitizer
  gate. The complete gate also generates the compiler project twice and
  compiles it as strict C11. `--assume-bootstrap --fixtures-only` is the
  explicit artifact-reuse edit loop.
- `scripts/check-self-hosting-fixed-point.sh`: isolated B4 Stage1/Stage2 C
  identity, strict compile, compiler-core differential and ASan/UBSan deep gate.
  It runs outside the ordinary canonical edit loop.
- `docs/todo-self-hosting-bootstrap/`: closed B0 contract and decisions.
- `docs/todo-self-hosting-frontend/`: closed B1 work breakdown and decisions.
- `docs/todo-self-hosting-semantics/`: closed B2 work breakdown and decisions.
- `docs/todo-self-hosting-backend/`: closed B3 work breakdown and decisions.
- `docs/todo-self-hosting-fixed-point/`: active B4 fixed-point work breakdown
  and decisions.
- `docs/todo-self-hosting-loop-performance/`: B2 full/fast gate and optimized C
  execution contract.
- `tests/fixtures/self-hosting/`: focused capabilities required by compiler code.

## B3 Closure

P177a exposes read-only typed-IR metadata to Mallang and adds a Mallang-owned
scalar C emitter plus standalone `c` host mode. Its fixture covers scalar
functions, checked arithmetic, calls, bindings, assignment, `print`, comparison
and unary logic. Stage0 and Stage1 produce byte-identical deterministic C; the
strict native, allocation-accounting and ASan/UBSan paths pass. The complete
compiler source set also matches across 713 normalized typed-IR functions.

P177b1 adds the string runtime, static UTF-8 literals, owned string
return/local/drop behavior, print/equality full-expression cleanup, statement
`if`/`else`, condition loops, `break` and `continue`. The scalar and
owned-control fixtures both match Stage0 C byte-for-byte and pass strict native,
allocation-accounting and ASan/UBSan execution. The expanded compiler source
matches Stage0 across 725 normalized typed-IR functions. P177b remains active
for the remaining composite and control-flow surfaces.

P177b2 preserves semantic type shapes and specialized struct/enum declarations
inside the Mallang IR, then uses them to emit fixed arrays, heap-backed slices,
structs and recursive cleanup helpers. Array/slice literals, struct literals,
field access, checked indexing and `len` match Stage0 byte-for-byte in the
composite fixture. The backend gate now covers three positive fixtures, a
dynamic bounds rejection and an explicit unsupported-node rejection before
strict native, accounting and sanitizer execution. The compiler source matches
Stage0 across 806 normalized typed-IR functions. At the P177b2 boundary, ADTs,
expression control flow, match/range forms, overwrite cleanup and dynamic
construction remained open.

P177b3 uses the retained metadata for built-in and user ADT definitions,
inline/owned constructors, recursive drop helpers, expression `if`, both match
forms and recursive patterns. Its positive fixture executes nested inline and
owned patterns, multi-payload bindings and cleanup-bearing branches. The gate
also forces recursive enum allocation failure and preserves `RangeFor` as an
explicit later-slice boundary. Four positive, two runtime-rejection and two
boundary-rejection paths pass the applicable byte-identity, deterministic,
strict native, accounting and ASan/UBSan checks. Broader recursive-enum,
branch-prelude and nested-pattern examples also match Stage0 C byte-for-byte,
and the compiler source matches across 831 normalized typed-IR functions.

P177b4 adds three-clause init/condition/post lowering, post-preserving nested
`continue`, loop-exit cleanup and array/slice range lowering. Range source
temporaries remain live for the complete loop and are cleaned after normal
exit or `break`, while loop-local cleanup runs before `continue`. Binary
operand preludes retain Stage0 left-to-right order and logical right-hand sides
remain short-circuited. The fifth positive fixture combines condition/post
preludes, indexed post targets, array/slice/temporary ranges and control flow;
all five positive and four rejection paths pass the applicable differential,
strict native, accounting and sanitizer gates. The compiler source matches
across 845 normalized typed-IR functions. At the P177b4 boundary, overwrite
cleanup, slice append and dynamic owned-string construction remained open.

P177b5 emits cleanup-bearing overwrite nodes after their owned RHS temporary is
fully evaluated. Local, field, slice index and indexed-field targets are
evaluated once before dropping the old value and moving the replacement; slice
base snapshots and bounds checks remain byte-identical to Stage0. The sixth
positive fixture passes strict native, allocation-accounting and ASan/UBSan
execution, leaving one explicit unsupported-node boundary. The compiler source
matches across 846 normalized typed-IR functions. At the P177b5 boundary,
slice append and dynamic owned-string construction remained open.

P177b6 implements consuming slice growth for direct values and local-rooted
field/index places. Field sources are reset to an empty header after transfer,
capacity growth preserves overflow and allocation failure diagnostics, and
self-consuming assignments bypass overwrite drop. The seventh positive fixture
and a forced realloc failure pass differential, strict native,
allocation-accounting and ASan/UBSan checks; no unsupported-node boundary
fixture remains. The compiler source matches across 851 normalized typed-IR
functions. At the P177b6 boundary, only dynamic owned-string construction
remained.

P177b7 adds an internal linked-project C operation and demand-driven
`strings.fromInt`/`strings.fromBool` runtime helpers. Both dynamic results own
their allocated UTF-8 buffer and use the existing exact-once string cleanup.
The eighth positive fixture and forced first string-allocation failure match
Stage0 C byte-for-byte before strict native, allocation-accounting and
ASan/UBSan execution. The compiler source matches across 855 normalized
typed-IR functions. P177b is complete; P177c-P177d remain open.

P177c1 adds the Stage0 pointer ABI for `con` and `mut` parameters, borrowed
parameter dereferencing, single-evaluation borrowed direct-call arguments and
qualified receiver methods. The ninth positive fixture covers receiver calls,
string field borrows and mutable array/slice element borrows. Four runtime
rejections and an explicit function-value boundary keep the next slice visible.
All paths pass the applicable differential, strict native, accounting and
sanitizer checks, and the compiler source matches across 857 normalized
typed-IR functions. Function values, closures, remaining intrinsics and the
broader project surface remain in P177c.

P177c2 retains callable signatures in the typed IR and uses them for function
type mangling, the environment/drop/call C ABI and exact-once callable cleanup.
Named function values validate their declaration signature and use generated
thunks; indirect calls evaluate the callee once and preserve owned, `con` and
`mut` argument evaluation and reverse cleanup order. The tenth positive fixture
passes byte parity, strict native, accounting and sanitizer checks, while a
closure remains the single explicit boundary. The compiler source matches
Stage0 across 880 normalized typed-IR functions. Intrinsic function values,
closures/captures and the broader project surface remain in P177c.

P177c3 completes the compiler-used scalar expression surface with checked
integer negation, division and remainder, including overflow and zero guards.
Logical short-circuit emission now consumes the canonical typed-IR kind names.
The scalar fixture and a division-by-zero rejection bring the backend gate to
ten positive, five runtime-rejection and one closure-boundary path. Compiler
source parity is 882 normalized functions, and self C generation of the
compiler project now reaches the `StringsByteLen` intrinsic boundary.

P177c4 gives intrinsic calls the ordinary call argument and cleanup ABI, then
adds demand-driven runtime helpers for the six string operations used by the
compiler: byte length/access, UTF-8-safe slicing, byte-offset find, join and
integer parsing. Concrete `Result`, `Option`, `Error` and `[]string` C types come
from typed-IR metadata. A linked project fixture covers successful and
domain-error results, and an empty join fixture covers forced helper allocation
failure. Eleven positive, six runtime-rejection and one closure-boundary path
pass deterministic C parity, strict native, accounting and sanitizer checks.
Compiler source parity is 897 normalized functions, and self C generation now
reaches `IoWriteStderr`.

P177c5 completes the platform surface used by the compiler with owned process
arguments, stdout/stderr writes, text-file reads and bounded process exit. The
runtime preserves allocation-failure diagnostics and exact cleanup for partial
argument and file-read construction. Owned direct slice fields use an internal
take-and-reset IR operation, while read, assignment and append contexts retain
their distinct borrow or consuming behavior. Twelve positive, nine runtime
rejection and one explicit closure-boundary path pass the B3 fixture gate, and
the complete compiler source matches Stage0 across 908 normalized typed-IR
functions.

P177d closes B3. Stage1 deterministically emits the complete compiler project
as warning-clean strict C11, with source discovery and native C compilation
remaining explicit host-harness operations. Closures and intrinsic function
values remain valid language features but are not used by the current compiler
source set, so their backend support is outside the B3 bootstrap-critical
surface. The canonical repository and supported-platform CI gates protect the
closed boundary. B4 owns Stage1-to-Stage2 fixed-point and conformance behavior.

## B4 Current Slice

P178a introduces an isolated deep gate. Rust Stage0 builds Stage1 from the
declared source set; Stage1 emits strict-C11 Stage2, and Stage2 must emit the
same compiler C byte-for-byte. A sanitizer-instrumented Stage2 must regenerate
the same output. The gate also compares Stage1 and Stage2 across every tracked
compiler-core fixture, the complete parser corpus, compiler project operations
and linked backend project C generation. P178b retains complete project-graph
and native behavior conformance before B4 can close.

P178a is complete. The full gate reaches a 9,780,069-byte compiler-C fixed
point, reproduces it under ASan/UBSan and matches 487 compiler-pair fixtures,
the 168-source parser corpus, four compiler project operations and eight linked
backend projects. P178b-P178c remain active.

P178b is complete. The existing B2 harness exposes a compiler-pair mode, so
Stage1 and Stage2 traverse the same complete package-layout, linker,
standard-project, compiler-project, fixture and parser-corpus inventory. The B4
gate additionally compares fourteen flat and eight linked backend C outputs and
runs twenty-one strict native positive/rejection/allocation pairs. B3 accounting
and Stage0 parity apply transitively to the byte-identical C. P178c remote
supported-platform acceptance remains open.

B1 is complete. The Mallang frontend covers the frozen v1 lexer, parser and
bounded recovery, and the repository corpus matches Rust Stage0 through normal,
strict-accounting and sanitizer execution. B2 is complete: P176a provides the
declaration/type checker, P176b1 adds primitive bodies and typed IR, and P176b2
adds named function values plus direct/indirect calls. P176b3a adds field/index
reads, and P176b3b adds mutable field/index assignment places. P176b4a adds
nested lexical scopes and if-statement return convergence, while P176b4b adds
if-expression branch type convergence. P176c1 adds non-Copy local moves and
direct local `con`/`mut` call borrows, and P176c2 extends them to nested
field/index places with same-call overlap checking. P176c3a conservatively
merges outer move state across statement and expression `if` branches. P176c3b1
checks condition and conditionless loop scope, control depth and persistent
condition/body moves. P176c3b2a adds loop-scoped init bindings, optional
conditions and direct-binding post assignment ownership. P176c3b2b1 reuses the
same assignment-place checks for field/index post targets. P176c3b2b2 checks
range source reads, Copy/index-only bindings, body scope, active-source
assignment and persistent outer moves. P176c4a checks owned/`con`/`mut` direct
local method receivers, method arguments and receiver/argument borrow overlap.
P176c4b extends the same ownership rules to local-rooted field/index receivers
and temporary/computed bases without evaluating receiver inputs twice. P176d1a
checks explicit non-generic struct, fixed-size array and slice literals with
owned element moves. P176d1b1 propagates expected types into those literals
through calls, returns, assignments, nested fields/elements and if-expression
branches. P176d1b2a checks `None`/`Some`/`Ok`/`Err` context, arity, owned
arguments and nested expected payloads. P176d1b2b checks known non-generic user
enum constructors with zero, one or multiple payloads, including expected
payload types, owned modes and move order. P176d1b2c1 checks flat
`Option`/`Result` expression match patterns, exhaustiveness, expected arm types,
binding scopes and branch move joins. P176d1b2c2a extends the same flat pattern
contract to statement match, including return convergence and loop-control
scope. P176d1b2c2b1 extends expression and statement matches to flat
non-generic user enum variants with zero, one or multiple payload bindings,
deterministic exhaustiveness and scrutinee ownership. P176d1b2c2b2 adds nested
built-in/user enum payload patterns, Cartesian multi-payload coverage and finite
recursive-enum coverage. P176d2a1 adds capture-free plain/mutable function
literals, structural callable signatures and indirect mutable-call checks.
P176d2a2 records plain closure captures in first-use order, preserves capture
metadata, copies Copy captures and moves non-Copy captures at creation while
rejecting moved, borrowed non-Copy and active range-source captures. Mutable and
nested capture propagation is complete in P176d2a3, including direct mutation,
`mut` arguments and receivers, nested Copy propagation, borrowed non-Copy
rejection and recursive initializer diagnostics. Complete generic semantics,
deterministic drop insertion and the remaining typed IR remain incomplete.
P176e1 lowers plain, mutable and nested closure definitions, parameters,
ordered capture metadata, closure values and capture expressions into typed IR.
Nine focused IR fixtures now compare the Rust Stage0 oracle and generated
Stage1 byte-for-byte. P176e2a adds deterministic straight-line drops for owned
cleanup parameters and locals, excludes moved roots, and evaluates return
values before remaining drops with stable temporary names. Ten focused IR
fixtures now cover this contract. Branch cleanup, overwrite cleanup and the
remaining typed IR are incomplete. P176e2b1 recursively inserts branch-local
tail and return cleanup with the enclosing `if` span, bringing the focused IR
corpus to eleven fixtures. P176e2b2 merges non-shadowing outer cleanup roots
across nested branches, inserts compensating drops only on continuing paths
that retain a root moved elsewhere, and drops outer roots before branches that
return without moving them. Twelve focused IR fixtures cover this contract.
P176e2b3 gives each cleanup binding a stable name-and-declaration-span identity,
keeps inner shadow moves and drops separate from the same-named outer root, and
preserves the original identity when assignment reactivates a moved root.
Thirteen focused IR fixtures and 207 Mallang compiler project tests cover this
contract. P176e2c1 evaluates direct local cleanup assignment RHS values into a
stable temporary before `Stmt.Overwrite`, keeps the old target alive through
that evaluation, and reactivates a self-consuming assignment under the same
binding identity. Fourteen focused IR fixtures and 208 Mallang compiler project
tests cover the expanded contract. P176e2c2 extends RHS-before-overwrite to
non-self-consuming field and index places without moving their aggregate bases.
Fifteen focused IR fixtures and 209 Mallang compiler project tests cover this
boundary. P176e2c3a models mutable cleanup parameters and captures as externally
owned overwrite roots: replacement RHS values are evaluated first, while the
caller/environment-owned root receives no tail drop. Sixteen focused IR
fixtures and 210 Mallang compiler project tests cover this boundary.
P176e2c3b lowers field-source `append` calls as `Expr.SliceAppend`, preserves
direct and indexed same-field assignments without overwrite, and retains
RHS-first overwrite for a distinct source path. Seventeen focused IR fixtures
and 212 Mallang compiler project tests cover this boundary. P176e2c3c covers
direct slice self-append reactivation, field-source reads and non-Copy item
moves. P176e2c3d lowers read-only `len` as `Expr.ArrayLen`, evaluates return
values before dropping owned sources and preserves field owners. P176e2c3e
normalizes statement-only `print` sources as read-only `Arg.Con` values and
drops their owned direct or field owner at function tail. P176e2c3f lowers
struct literal fields in declaration order as `Field.Value` nodes and consumes
owned field sources exactly once. P176e2c3g lowers fixed-array and slice
literals as `Expr.ArrayLiteral` and consumes owned elements exactly once.
P176e2c3h lowers `None`, `Some`, `Ok` and `Err` as inline
`Expr.VariantConstructor` values and consumes owned payloads exactly once.
P176e2c3i lowers user enum constructors with graph-derived inline or owned
storage and consumes zero, one and multiple payloads exactly once. P176e2c3j
lowers flat Copy `Option` and `Result` expression matches as explicit match-arm
and pattern nodes. P176e2c3k extends that contract to cleanup payloads, moved
bindings, cleanup wildcards and string-read full-expression temporaries.
P176e2c3l lowers flat user-enum expression matches with graph-derived inline or
owned pattern storage, multiple payload bindings and cleanup wildcards.
P176e2c3m recursively lowers nested built-in and user-enum patterns, including
recursive owned enums, while sharing deterministic cleanup wildcard numbering
across each arm. P176e2c3n lowers statement-form match blocks with arm-local
payload cleanup and outer branch move compensation. Twenty-nine focused IR
fixtures and 227 Mallang compiler project tests cover that boundary. P176e2c3o
checks test declaration bodies and lowers `assert(bool)` plus the selected test
body as a synthetic `main`. Twenty-nine focused IR fixtures and 230 Mallang compiler
project tests cover that boundary. P176e2c3p lowers condition and conditionless
`for`, `break` and `continue` with explicit body and cleanup blocks. Thirty
focused IR fixtures and 231 Mallang compiler project tests cover that boundary.
P176e2c3q lowers three-clause `for` init, optional condition and post nodes.
Thirty-one focused IR fixtures and 232 Mallang compiler project tests cover the
expanded boundary. P176e2c3r lowers Copy element array and slice range bindings,
source reads, body and cleanup blocks while preserving owned range-source
cleanup. Thirty-two focused IR fixtures and 233 Mallang compiler project tests
cover that boundary. P176e2c3s inserts owned for-init exit cleanup and loop/range
body-local cleanup at normal tails and before `break` or `continue`. Thirty-three
focused IR fixtures and 234 Mallang compiler project tests cover that boundary.
P176e2c3t lowers index-only non-Copy ranges and indexed `con` element borrows.
Thirty-four focused IR fixtures and 235 Mallang compiler project tests cover the
expanded boundary. P176e2c3u lowers method declarations as
`ReceiverType.method` functions and preserves direct local owned, `con` and
`mut` receivers as the first call argument. Thirty-five focused IR fixtures and
236 Mallang compiler project tests cover that boundary. P176e2c3v lowers
field, index, temporary and computed method receivers with place-aware
full-expression temporaries while preserving moves inside computed owners.
Thirty-six focused IR fixtures and 237 Mallang compiler project tests cover
that boundary. P176e2c3w routes computed array and projected-field `len` sources
through place-aware full-expression temporaries and preserves moves inside their
owners. Thirty-seven focused IR fixtures and 238 Mallang compiler project tests
cover that boundary. P176e2c3x retains inline and projected range temporary
owners through normal loop exit and early control flow while preserving source
moves. Thirty-eight focused IR fixtures and 239 Mallang compiler project tests
cover that boundary. P176e2c3y normalizes expression-form `if` branch moves into
explicit then/else cleanup blocks shared by Stage0 and Stage1. Thirty-nine
focused IR fixtures and 240 Mallang compiler project tests cover the expanded
boundary. P176e2c3z appends expression-form `match` outer-owner compensation
drops after each arm's existing pattern cleanup. Forty focused IR fixtures and
241 Mallang compiler project tests cover the expanded boundary. P176e3a adds
demand-driven generic struct, function and receiver specialization, and P176e3b
extends the same concrete path to generic enum constructors and source pattern
origins. P176e3c validates every generic declaration body once with symbolic
non-Copy, non-printable arguments and restores internal names to source `T` and
`Box[T]` diagnostics. Two hundred thirty-six semantic fixtures, forty-two
typed-IR fixtures and two hundred forty-four Mallang project tests cover this
boundary. P176e4a preserves source IDs through tokens, lexical diagnostics and
parser spans, then merges multiple syntax arenas in deterministic declaration
groups. Two valid/rejection source sets and two hundred forty-six Mallang
project tests fix the Rust `parse_sources_with_diagnostics` boundary. P176e4b1
derives canonical source package identity from deterministic project and path
inputs, then matches Rust package-declaration validation for valid, missing and
mismatched layouts. Three package-layout sets, one hundred sixty-three parser
corpus sources and two hundred forty-eight Mallang project tests cover this
boundary. P176e4b2a groups same-project source packages, validates file-local
import paths and qualifiers, rejects unresolved imports and cycles, and emits
lexical package/import plus dependency-first build order matching Rust. One
valid and seven rejection layout sets, one hundred sixty-three parser corpus
sources and two hundred fifty Mallang project tests cover this boundary.
P176e4b2b collects struct, enum and function declarations per package, groups
methods by receiver, preserves visibility and type parameters, and matches Rust
duplicate declaration diagnostics. One valid and nine rejection layout sets,
one hundred sixty-three parser corpus sources and two hundred fifty-one Mallang
project tests cover this boundary. P176e4b2c synthesizes the six compiler-owned
standard packages at their import spans and preserves all thirty public declaration
kinds and generic parameters in Rust registry order. Unknown `std/*` imports match
the Rust package error span and message. Two valid and ten rejection layout sets,
one hundred sixty-three parser corpus sources and two hundred fifty-three Mallang
project tests cover this boundary. P176e4b3a preserves dependency project source
roots and direct dependency edges in the compiler input. The transitive
`app -> text -> shared` graph normalizes dependency-first, while an `app` import
of undeclared `shared` matches the Rust source, span and message. Three valid and
eleven rejection layout sets, one hundred sixty-three parser corpus sources and
two hundred fifty-five Mallang project tests cover this boundary. Package
visibility and package-qualified declaration/type/body rewriting are complete
in P176e4b3b. Five focused linker tests, six project differential invocations
and the complete eleven-file compiler source link match Rust Stage0; the
integrated parser corpus is one hundred sixty-five sources and the Mallang
project suite is two hundred sixty tests. P176e4c1 adds compiler-owned standard
declaration augmentation, intrinsic generic specialization and semantic
metadata, plus typed intrinsic calls and function values. The valid project's
augment, prepare, check and typed-IR outputs and the unsupported-map-key
rejection match Rust Stage0. Three focused tests, five project differential
invocations, one hundred sixty-seven parser corpus sources and two hundred
  sixty-three Mallang project tests cover this boundary. The quadratic string
  cursor path is removed, fixture/corpus work uses bounded concurrency, and CI
  no longer repeats the canonical core gate per platform artifact. `mlg test`
  lowers selected tests into one deterministic C runner while keeping every case
  in a separate child process. The complete 263-test compiler suite now takes
  about 3.2-3.4 seconds instead of roughly 250 seconds and produces a 9.9 MB
  artifact directory instead of about 1.76 GB of generated C. Representative
  A function-indexed IR comparator and rebuildable diagnostic loop isolate the
  first compiler-source mismatch in about 20 seconds. Assignment reactivation,
  borrowed full-expression arguments, computed places, partial field moves,
  return pre-evaluation and Copy pattern shadowing are fixed by six new fixtures.
  All 675 normalized functions from the twelve-file compiler source now match
  Stage0 typed IR, and the full gate enforces link, prepare, check and IR parity.
  The C backend also gives pattern bindings arm-span identities so an inner Copy
  binding cannot redirect an outer cleanup drop.

  Generated Stage1/profile builds now run concurrently, and fast mode runs the
  complete 263-test suite once instead of rebuilding 24 selected runners. On the
  same local host, the IR-focused gate completed in 26 seconds, fast in 40
  seconds and the full 48-fixture, 167-source gate in 83 seconds. These are
  host-local observations, not thresholds. The canonical repository gate,
  public `main` publication and supported-platform CI acceptance all pass, so
  B2 is complete. B3 subsequently moved the compiler-required C backend and
  linked-project path behind Stage1 and is now complete; B4 fixed-point work is
  next.
