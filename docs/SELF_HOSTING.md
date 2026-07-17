# Mallang Self-Hosting

Status: active long-term program; B0 complete, B1 frontend next

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
- `scripts/check-self-hosting-bootstrap.sh`: current bootstrap gate.
- `docs/todo-self-hosting-bootstrap/`: active stage contract and decisions.

B1 will introduce the compiler's Mallang source root only after the frontend
data model and differential output format are fixed.
