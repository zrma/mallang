# B2 Self-Hosting Semantics And Typed IR

Status: in progress; P176a-P176b3 complete

## Objective

Implement the Mallang compiler's semantic, ownership and typed-IR core in
Mallang while preserving the frozen v1 language contract. Rust Stage0 remains
the differential oracle until the complete positive and rejection corpus
agrees.

## Boundaries

- The Mallang checker receives the B1 flat syntax arena through `con Program`.
  It does not duplicate or retain syntax solely to emulate Rust's `Arc<Program>`.
- `CheckedProgram` owns resolved declarations and types. Typed lowering will
  receive both the immutable syntax arena and checked model explicitly.
- Stable differential output is harness-owned. Rust `Debug`, hash-map order, C
  layout and private string tags are not language contracts.
- Type identity records a canonical source name plus private kind, Copy and
  cleanup classification. Source syntax still uses the frozen v1 type grammar.
- The host driver may discover files and invoke the compiler, but it may not
  resolve names, infer types, enforce ownership or construct typed IR.
- A compiler implementation inconvenience is not evidence for a new syntax or
  standard-library feature.

## Work Breakdown

### P176a: Declarations And Resolved Types

- collect non-generic struct, enum, function and method declarations
- reject duplicate or reserved declarations and invalid `main` signatures
- resolve primitive, Option, Result, array, slice, nominal and function types
- retain Copy and cleanup classification in the private resolved type model
- compare normalized signatures and first semantic diagnostics with Stage0
- run generated C through strict warnings, allocation accounting and ASan/UBSan

### P176b: Expressions, Statements And Local Types

- P176b1: check primitive literals, unary/binary operators, bindings,
  assignment and return types; lower that subset into typed IR
- P176b2: type direct calls, arguments and named function values (complete)
- P176b3a: type field/index reads and lower their typed IR (complete)
- P176b3b: type mutable local-rooted field/index assignment places (complete)
- P176b4: enforce nested lexical scopes and branch type convergence

### P176c: Ownership And Places

- model available, moved, immutably borrowed and mutably borrowed local states
- distinguish owned, `con` and `mut` call arguments and method receivers
- validate assignment places, field/index borrows and branch/loop state joins
- preserve cleanup obligations without exposing pointers or first-class borrows

### P176d: Complete Semantic Surface

- arrays, slices, structs, enums, match coverage and recursive ADTs
- closures, captures, function values and indirect calls
- generic validation, specialization and standard intrinsic identity
- package visibility, methods, tests and complete control-flow checking

### P176e: Typed IR And B2 Closure

- lower every checked construct into the Mallang typed IR arena
- insert deterministic drops and full-expression temporaries
- normalize checked declarations, diagnostics and typed IR independently of C
- run the full positive, semantic-rejection and ownership-rejection corpus
- close supported-platform CI, publication and sanitizer evidence for B2

## P176a Differential Contract

Successful sources emit declaration groups in this order: structs, enums,
top-level functions and methods. Each field, variant payload, parameter,
receiver and return type uses the same canonical source spelling as Stage0.
Rejected sources emit `SERR|source|start|end|encoded-message` for the first
semantic diagnostic. Parser and lexer failures retain their B1 schemas.

The focused P176a corpus covers nested built-in types, function types, arrays,
slices, nominal fields, enum payloads, methods, unknown types, duplicate fields,
empty enums and invalid entrypoints. It intentionally excludes generic
specialization and function-body checking until later P176 slices.

## P176a Evidence

- `bootstrap/compiler/src/semantic/semantic.mlg` owns declaration collection,
  resolved type classification and stable checked/diagnostic normalization.
- Ten focused success and rejection fixtures match Rust Stage0 byte-for-byte,
  including declaration conflicts, reserved names, type arity, receiver and
  parameter validation.
- The bootstrap compiler's 13 Mallang project tests pass through Stage0.
- The integrated self-hosting gate validates 157 repository parser sources and
  every semantic fixture through generated Stage1, strict allocation accounting
  and ASan/UBSan execution with empty stderr.

## P176b1 Evidence

- `CheckedProgram` records a flat `TypedNode` arena keyed by B1 syntax node
  index; no first-class borrow or retained syntax reference is introduced.
- Primitive function bodies enforce local uniqueness, mutability, assignment
  type, unary/binary operand, return type and return-completeness rules.
- Eighteen semantic fixtures cover the P176a contract plus primitive positive
  and rejection bodies, and 16 Mallang project tests pass through Stage0.
- `bootstrap/compiler/src/ir/ir.mlg` lowers primitive typed functions,
  parameters, statements and expression trees into a separate flat IR arena.
- The primitive IR fixture matches Stage0 `IrProgram` normalization exactly,
  including node category, kind, type, source span, value and child order.

## P176b2 Evidence

- AST-node function references distinguish top-level direct references from
  local function values without adding recursive fields or first-class borrows
  to `ResolvedType`.
- Direct and indirect calls enforce Stage0 arity, owned/`con`/`mut` mode and
  argument type diagnostics; named function values and aliases retain their
  callable signature.
- Twenty-six semantic fixtures cover declaration/body checks plus direct and
  indirect call success and rejection cases. Nineteen Mallang project tests
  pass through Stage0.
- Two typed-IR fixtures match Stage0 for primitive nodes and `FunctionValue`,
  `Call`, `IndirectCall` and mode-bearing argument nodes. Cleanup-local return
  rewriting and deterministic `Drop` insertion remain explicitly in P176e.
- The integrated generated-Stage1 gate covers 159 repository parser sources,
  strict allocation accounting and ASan/UBSan with empty stderr.

## P176b3a Evidence

- A canonical type-shape registry preserves nested array, slice, Option and
  Result arguments without making `ResolvedType` recursive or retaining syntax.
- Struct field reads and fixed-array/slice index reads agree with Stage0 for
  field existence, base/index type, negative and out-of-bounds literals, and
  the current Copy-element restriction.
- Thirty-four semantic fixtures and three typed-IR fixtures cover the cumulative
  B2 subset. Twenty-two Mallang project tests pass through Stage0.
- `FieldAccess` and `Index` typed nodes preserve Stage0 result types, source
  spans, field names and child evaluation order through normal, strict
  accounting and ASan/UBSan execution.

## P176b3b Evidence

- Mutable local roots are resolved independently from nested field/index place
  types; immutable roots, unknown fields, invalid indexes and RHS mismatches
  retain Stage0's first diagnostic and span.
- `FieldAssign` and `IndexAssign` typed statements preserve base, index and RHS
  evaluation order without introducing cleanup or ownership behavior early.
- Forty-two semantic fixtures and four typed-IR fixtures cover the cumulative
  B2 subset. Twenty-five Mallang project tests pass through Stage0.
- Generated Stage1, strict allocation accounting and ASan/UBSan agree with the
  Rust oracle across the focused fixtures and 159-source parser corpus.

## B2 Completion Criteria

- All frozen v1 positive sources accepted by Stage0 are accepted by the Mallang
  semantic and typed-IR core.
- Semantic and ownership rejection stage, span and message agree with Stage0.
- Normalized typed IR agrees for all accepted sources in deterministic order.
- Generated compiler code remains warning-clean, leak-free under allocation
  accounting and clean under ASan/UBSan on supported platforms.
- Canonical repository, publication and remote CI gates pass.

## Excluded

- C backend generation, which starts in B3
- Stage1-to-Stage2 compiler rebuild claims, which belong to B4
- default compiler transition, which belongs to B5
- source-visible compatibility changes without an independently demonstrated
  compiler blocker
