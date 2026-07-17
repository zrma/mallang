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
  accounting and sanitizer gate. The historical filename remains stable while
  the compiler gate grows through B2.
- `docs/todo-self-hosting-bootstrap/`: closed B0 contract and decisions.
- `docs/todo-self-hosting-frontend/`: closed B1 work breakdown and decisions.
- `docs/todo-self-hosting-semantics/`: active B2 work breakdown and decisions.
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
scope. Nested and user enum patterns, complete ADT/closure/generic semantics and
deterministic typed-IR drop insertion remain incomplete, so no complete
semantic, typed-IR or Stage1 compiler claim is made.
