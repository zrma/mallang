# Mallang Self-Hosting

Status: active long-term program; B0-B1 complete, B2 semantics in progress

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
  the complete lexer/parser plus declaration/type checking, primitive body
  checking, direct/indirect calls, field/index places and the incremental
  typed-IR subset with nested if statements and expressions, plus local
  move/borrow state and call-scoped place overlap checking.
- `scripts/check-self-hosting-bootstrap.sh`: current bootstrap gate.
- `scripts/check-self-hosting-lexer.sh`: deterministic Rust/Mallang lexer and
  parser differential plus incremental B2 semantic differential, ownership
  accounting and sanitizer gate. The argument-free command remains the full
  milestone gate; `--fast` keeps complete Stage0/Stage1 differential coverage
  with one exact project test per compiler phase, focused accounting and
  representative sanitizer smoke for inner loops. The historical filename
  remains stable while the compiler gate grows through B2.
- `docs/todo-self-hosting-bootstrap/`: closed B0 contract and decisions.
- `docs/todo-self-hosting-frontend/`: closed B1 work breakdown and decisions.
- `docs/todo-self-hosting-semantics/`: active B2 work breakdown and decisions.
- `docs/todo-self-hosting-loop-performance/`: B2 full/fast gate and optimized C
  execution contract.
- `tests/fixtures/self-hosting/`: focused capabilities required by compiler code.

B1 is complete. The Mallang frontend covers the frozen v1 lexer, parser and
bounded recovery, and the repository corpus matches Rust Stage0 through normal,
strict-accounting and sanitizer execution. B2 is active: P176a provides the
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
body as a synthetic `main`. Thirty focused IR fixtures and 230 Mallang compiler
project tests cover that boundary. P176e2c3p lowers condition and conditionless
`for`, `break` and `continue` with explicit body and cleanup blocks. Thirty-one
focused IR fixtures and 231 Mallang compiler project tests cover that boundary.
P176e2c3q lowers three-clause `for` init, optional condition and post nodes.
Thirty-two focused IR fixtures and 232 Mallang compiler project tests cover the
expanded boundary. Range, cleanup-bearing loops and remaining
full-expression typed IR are incomplete, so
no complete semantic, typed-IR or Stage1 compiler claim is made.
