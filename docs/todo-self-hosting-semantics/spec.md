# B2 Self-Hosting Semantics And Typed IR

Status: in progress; P176a-P176d1b2c1 complete

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
- P176b4a: enforce nested lexical scopes and if-statement return convergence
  (complete)
- P176b4b: type if expressions and enforce branch type convergence (complete)

### P176c: Ownership And Places

- P176c1: model local available/moved state, borrowed parameters and direct
  owned/`con`/`mut` call arguments (complete)
- P176c2: validate field/index borrow places and same-call overlap (complete)
- P176c3a: merge move state across statement and expression branches
  (complete)
- P176c3b1: check condition and conditionless loops, loop control and
  condition/body persistent move state (complete)
- P176c3b2a: check three-clause init, optional condition and direct binding post
  persistent move state (complete)
- P176c3b2b1: check field/index post targets through shared assignment places
  (complete)
- P176c3b2b2: check range loop bindings and persistent move state (complete)
- P176c4a: distinguish owned, `con` and `mut` direct local method receivers,
  including argument mode and receiver/argument overlap (complete)
- P176c4b: extend method receiver ownership to field, index and temporary bases
  (complete)
- validate assignment places, field/index borrows and branch/loop state joins
- preserve cleanup obligations without exposing pointers or first-class borrows

### P176d: Complete Semantic Surface

- P176d1a: check explicit non-generic struct, fixed-size array and slice literals,
  including field completeness, element counts, types and owned element moves
  (complete)
- P176d1b1: propagate explicit struct, array and slice literal expected types
  through calls, returns, assignments, fields, elements and branch expressions
  while preserving Stage0 diagnostic priority (complete)
- P176d1b2a: propagate expected types through `None`, `Some`, `Ok` and `Err`,
  including nested literal payloads and constructor context diagnostics
  (complete)
- P176d1b2b: check zero, one and multiple-payload user enum constructors and
  propagate expected payload types (complete)
- P176d1b2c1: check flat `Option`/`Result` expression match patterns,
  exhaustiveness, arm bindings, expected types and branch move joins (complete)
- P176d1b2c2: extend match semantics to nested patterns, user enums and
  statement-form control flow
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

## P176b4a Evidence

- Locals carry lexical depth, nearest-binding lookup permits nested shadowing,
  and duplicate checks remain scoped to the current block.
- If branches start from independent outer-local snapshots; branch locals do
  not leak, and a statement is return-complete only when both branches return.
- If typed IR uses deterministic condition, `Block.Then` and `Block.Else`
  children, including an empty else block when Rust IR loses source presence.
- Forty-seven semantic fixtures, five typed-IR fixtures and twenty-eight
  Mallang project tests cover the cumulative B2 subset through Stage0,
  generated Stage1, strict accounting and ASan/UBSan.

## P176b4b Evidence

- If expressions check a bool condition and independent branches before
  requiring one non-unit result type, including recursively nested else-if
  expressions.
- Condition, branch mismatch and unit-branch diagnostics preserve Stage0's
  first span and message without introducing expected-type or ownership rules
  before their representative constructs are supported.
- Typed IR preserves deterministic condition, then-expression and
  else-expression child order, with ownership cleanup explicitly excluded from
  this slice.
- Fifty-one semantic fixtures, six typed-IR fixtures and thirty-one Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan.

## P176c1 Evidence

- Locals record whether they originate from borrowed parameters and whether an
  owned non-Copy use has moved them; Copy values remain reusable.
- Direct local `con` and `mut` call arguments are call-scoped reads rather than
  first-class borrow values. Mutable borrows require a mutable root and all
  borrows reject moved roots.
- Moving a borrowed non-Copy parameter and using or borrowing a moved local
  preserve Stage0's first diagnostic message and source span.
- Fifty-seven semantic fixtures, six typed-IR fixtures and thirty-five Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan. Field/index borrow places and control-flow state joins remain
  explicit follow-up work.

## P176c2 Evidence

- Borrow places use a compiler-private root plus field/index segment path;
  nested fields and array/slice elements remain call-scoped syntax, not values.
- Root move and mutability checks preserve Stage0's binding, field and indexed
  place diagnostics. Repeated shared borrows and disjoint fields are accepted.
- Same-call overlap rejects shared/exclusive and exclusive/exclusive prefix
  paths with deterministic `root.field` and `root[?].field` diagnostics.
- Sixty-two semantic fixtures, six typed-IR fixtures and thirty-nine Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan. Control-flow ownership joins remain P176c3 work.

## P176c3a Evidence

- Statement and expression `if` branches start from independent local-state
  snapshots and conservatively merge an outer binding as moved when either
  branch moves it.
- Missing statement `else` branches retain the pre-branch state. Branch-local
  bindings remain excluded from the outer merge, and call-scoped `con` borrows
  do not persist past their call.
- Use-after-branch-move diagnostics agree with Stage0 for statement and
  expression forms, including the original source span and message.
- Sixty-five semantic fixtures, six typed-IR fixtures and forty-two Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan. Loop-persistent move-state joins remain P176c3b work.

## P176c3b1 Evidence

- Condition and conditionless `for` statements check body-local scope and
  preserve outer locals through call-scoped borrows. Conditions require `bool`.
- A newly moved non-Copy binding that persists across iterations is rejected in
  the condition or body with Stage0's diagnostic. Values declared inside the
  body remain iteration-local and may move.
- `break` and `continue` track nested loop depth and are rejected outside a
  loop. A loop remains non-return-complete even when conditionless.
- Seventy-one semantic fixtures, six typed-IR fixtures and forty-eight Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan. Three-clause init/post and range-loop ownership remain P176c3b2.

## P176c3b2a Evidence

- Three-clause loops collect an optional loop-scoped init binding, optional
  `bool` condition and direct mutable-binding post assignment in source order.
- Init bindings do not leak. Move-only outer or init bindings cannot move from
  the condition, body or post repeatable paths; Copy state and mutable post
  updates remain valid.
- Immutable post targets, leaked init bindings and persistent init/post moves
  preserve Stage0's diagnostic message and source span.
- Seventy-six semantic fixtures, six typed-IR fixtures and fifty-three Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan. Field/index post targets and range loops remain P176c3b2b.

## P176c3b2b1 Evidence

- Statement and for-post field/index assignments share one local-rooted place
  checker for mutability, nested field resolution, index validation and value
  type convergence.
- Mutable struct fields and array/slice elements are accepted as repeatable
  post targets. Immutable roots preserve Stage0's field/index diagnostics and
  source spans.
- Eighty semantic fixtures, six typed-IR fixtures and fifty-six Mallang project
  tests pass through Stage0, generated Stage1, strict accounting and ASan/UBSan.
  Range-loop binding and ownership checks remain P176c3b2b2.

## P176c3b2b2 Evidence

- Range sources are checked once as reads and remain reusable after the loop.
  Index bindings are `int`; value bindings require Copy elements, while
  index-only iteration accepts non-Copy array and slice elements.
- Range bindings remain body-local. Duplicate or built-in binding names,
  non-collection sources and assignment to an active direct range source
  preserve Stage0's first diagnostic message and source span.
- Outer non-Copy moves from a repeatable range body are rejected, while
  iteration-local state is discarded at the loop boundary and outer move state
  is merged conservatively.
- Eighty-nine semantic fixtures, six typed-IR fixtures and sixty-four Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan. Direct local receiver ownership remains P176c4a.

## P176c4a Evidence

- Direct local method calls probe a struct receiver before resolving the method.
  Owned receivers move non-Copy locals, `con` receivers retain availability and
  `mut` receivers require a mutable root.
- Method arguments preserve function-call arity, mode and type diagnostics.
  Receiver borrows participate in the same prefix-place overlap check as
  `con`/`mut` arguments, allowing shared/shared access and rejecting exclusive
  overlap.
- Moved receiver probes, unknown methods and non-struct receivers preserve
  Stage0's first diagnostic message and source span.
- One hundred semantic fixtures, six typed-IR fixtures and seventy-four Mallang
  project tests pass through Stage0, generated Stage1, strict accounting and
  ASan/UBSan at the P176c4a boundary.

## P176c4b Evidence

- Local-rooted field and array/slice index receivers resolve through the same
  prefix-aware stable-place model as call arguments. Moved roots, immutable
  `mut` roots and receiver/argument overlap preserve Stage0's diagnostic stage,
  message and span.
- Temporary, computed-field and computed-index receivers use a cloned local
  state for method lookup, then evaluate the real full expression exactly once.
  Owned field receivers preserve Stage0's direct, indexed and computed
  non-Copy move restrictions.
- One hundred thirteen semantic fixtures, six typed-IR fixtures and eighty-six
  Mallang project tests pass through Stage0, generated Stage1, strict
  allocation accounting and ASan/UBSan. Complete ADT, closure, generic and
  typed-IR coverage remains P176d-P176e.

## P176d1a Evidence

- Explicit struct literals reject duplicate, unknown and missing fields before
  recording their resolved struct type. Field expressions are checked in source
  order as owned values, preserving move state and Stage0 field diagnostics.
- Fixed-size array literals enforce their declared length; array and slice
  literals check owned elements in source order and preserve zero-based element
  mismatch diagnostics.
- One hundred twenty-one semantic fixtures, six typed-IR fixtures and
  ninety-four Mallang project tests pass through Stage0, generated Stage1,
  strict allocation accounting and ASan/UBSan. Expected-type propagation and
  constructors remain P176d1b2.

## P176d1b1 Evidence

- Calls, returns and mutable local, field and indexed assignments pass their
  destination type into explicit struct, fixed-array and slice literals.
  Struct fields and array/slice elements recursively pass their declared type.
- If-expression branches preserve the outer expected type, so a mismatched
  literal is rejected at the literal boundary before the general branch-type
  convergence diagnostic.
- One hundred thirty-one semantic fixtures, six typed-IR fixtures and one
  hundred four Mallang project tests pass through Stage0 and generated Stage1.
  Strict allocation accounting, ASan/UBSan and the canonical repository gate
  cover the same corpus. Built-in constructors continue in P176d1b2a; user
  enum constructors and the remaining match contexts remain P176d1b2b-P176d1b2c.

## P176d1b2a Evidence

- `None` requires an expected `Option[T]`; `Ok` and `Err` require an expected
  `Result[T, E]`. `Some` may infer its payload without context and uses an
  expected Option payload when one is available.
- Constructor arity and owned-argument mode are checked before context.
  Expected payload types flow recursively into struct and array literals while
  primitive payload mismatches preserve the enclosing result-type diagnostic.
- One hundred forty-one semantic fixtures, six typed-IR fixtures and one
  hundred fourteen Mallang project tests pass through Stage0 and generated
  Stage1. User enum constructors remain P176d1b2b, and match expression
  propagation remains P176d1b2c.

## P176d1b2b Evidence

- A known non-generic enum path is distinguished from ordinary field or method
  selection before checking zero-payload selection or callable payload
  variants. Stage0's constructor normalization takes priority over a same-name
  local binding.
- Expected enum type, variant existence, callability, payload arity, owned
  argument mode and positional type diagnostics preserve Stage0 order and
  spans. Payloads are checked left-to-right, including nested literal expected
  types and non-Copy move reuse.
- One hundred fifty-two semantic fixtures, six typed-IR fixtures and one
  hundred twenty-five Mallang project tests pass through Stage0 and generated
  Stage1. Match expression expected-type propagation remains P176d1b2c.

## P176d1b2c1 Evidence

- Flat `Option` and `Result` expression matches validate scrutinee types,
  built-in patterns, wildcard coverage, duplicate or unreachable arms and
  deterministic exhaustiveness diagnostics before checking arm expressions.
- Arm payload bindings use isolated lexical scopes. Expected types propagate
  through every arm, inference retries from the first independently typable
  arm, the scrutinee is consumed once and outer moves merge across all arms.
- Twelve focused success and rejection fixtures match Rust Stage0
  byte-for-byte. One hundred sixty-four semantic fixtures, six typed-IR
  fixtures and one hundred thirty-seven Mallang project tests pass through
  Stage0 and generated Stage1. Nested patterns, user enum patterns and
  statement-form match remain P176d1b2c2.

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
