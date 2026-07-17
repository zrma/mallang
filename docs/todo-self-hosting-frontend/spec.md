# Spec: B1 Self-Hosting Frontend

Status: active; P175a-P175c3 complete, P175d full differential corpus next

## Goal

Implement the frozen v1 lexer and parser in Mallang and prove token, syntax tree
and frontend diagnostic equivalence against the Rust Stage0 oracle.

## Work Breakdown

- **P175a**: add the narrow owned string cursor operations required for a
  linear-time lexer, with runtime, rejection, allocation and sanitizer evidence
- **P175b**: define Mallang source/span/token data and implement the complete
  lexer with deterministic differential output
- **P175c1**: define the flat syntax arena and implement package, import,
  declaration and type parsing with exact Rust-oracle normalization
- **P175c2a**: implement core statements, Pratt expressions, postfix chains,
  literals, calls, assignments and struct/array construction
- **P175c2b1**: implement control-flow statements, test assertions and match
  statement patterns
- **P175c2b2**: implement function literals, if/match expressions and complete
  recursive pattern coverage
- **P175c3**: add bounded statement/top-level recovery without changing the
  frozen grammar
- **P175d**: run the positive, rejection and crash corpus through both frontends
  and close B1 only when normalized tokens, ASTs and diagnostics agree

## Compatibility

`strings.byteAt` and `strings.slice` are backward-compatible standard-library
additions. They expose no pointer, borrowed return, mutable string view or host
handle. Their byte offsets align with existing `byteLen`, `find` and source span
semantics.

No lexer/parser convenience syntax is approved by B1. Any further public
surface change requires a concrete compiler-source blocker and independent 1.x
compatibility evidence.

## Acceptance

- [x] safe byte read and UTF-8-boundary-checked owned slice operations
- [x] strict C, ASan/UBSan, allocation accounting and failure injection evidence
- [x] Mallang source/span/token model and stable byte-oriented normalization
- [x] complete Mallang lexer and Rust token/diagnostic differential corpus
- [x] Mallang frontend AST and complete success-path parser
- [ ] normalized AST and diagnostic differential corpus
- [ ] B1 canonical, publication and supported-platform CI gates

## P175b Evidence

- `bootstrap/compiler/src/frontend/token/token.mlg` owns byte spans, token
  payloads and the `T|...` / `E|...` differential representation.
- `bootstrap/compiler/src/frontend/lexer/lexer.mlg` implements the frozen v1
  token and keyword set with zero-based UTF-8 byte offsets.
- `tools/bootstrap-frontend-oracle.rs` adapts the Rust Stage0 lexer to the same
  representation without exposing Rust `Debug` output as a contract.
- `scripts/check-self-hosting-lexer.sh` verifies Stage0-generated C identity,
  project tests, positive and rejection corpus equivalence, strict C,
  ASan/UBSan and zero live compiler-owned allocations.
- The cleanup regressions under `tests/fixtures/self-hosting/` protect owned
  slice append reassignment through `match`, including cleanup-bearing values.

## P175c1 Evidence

- `bootstrap/compiler/src/frontend/source/source.mlg` and
  `bootstrap/compiler/src/frontend/ast/ast.mlg` define byte spans and a flat
  node arena with stable preorder normalization. The arena allows repeated
  syntax traversal through `con` without weakening move-only enum rules.
- `bootstrap/compiler/src/frontend/parser/parser.mlg` parses package and import
  clauses, struct and enum declarations, generic type parameters, functions,
  methods, tests and the frozen v1 type grammar. Function bodies are accepted
  only when empty in this slice.
- `bootstrap/compiler/fixtures/parser/` covers normalized success and rejection
  output, and `tools/bootstrap-frontend-oracle.rs` maps the equivalent Rust AST
  subset into the same harness-owned representation.
- Token predicate helpers compare private token fields in place, avoiding an
  owned copy solely for parser lookahead.
- The P175c1 work exposed and fixed Stage0 cleanup of owned temporary strings in
  equality expressions. `string-equality-temporaries.mlg` now proves that the
  comparison happens before reverse-order cleanup under strict allocation
  accounting and ASan/UBSan.

## P175c2a Evidence

- The Mallang parser now builds normalized nodes for let, assignment, return
  and expression statements; unary/binary precedence; calls and argument modes;
  field/index/type-apply chains; pipelines; and struct/array literals.
- `core-expressions.mlg` exercises the core body grammar against the Rust AST
  oracle, including generic struct construction and pipeline desugaring.
- The Rust oracle now owns the complete frozen statement/expression/pattern node
  vocabulary so later slices extend coverage without changing prior output.
- Nested cleanup passes now recognize an already pre-evaluated compiler return
  temporary. `match-arm-return-temp.mlg` protects match payload cleanup plus an
  outer owned local under strict C, allocation accounting and ASan/UBSan.

## P175c2b1 Evidence

- Statement parsing now covers if/else-if, infinite/conditional/C-style/range
  loops, break/continue, block match arms and test-only assertions.
- Match patterns use a pending flat arena and are materialized with the complete
  arm span, preserving exact Rust oracle normalization without recursive owned
  syntax values.
- `control-flow.mlg` differentially covers loop header variants, built-in and
  qualified multi-payload patterns, nested blocks and assertions under strict
  C, zero-allocation accounting and ASan/UBSan.

## P175c2b2 Evidence

- Expression parsing now covers plain/mutable function literals, optional
  return types, if/else-if expressions and expression match arms.
- Recursive built-in and qualified variant patterns reuse the pending pattern
  arena and materialize every nested node with the enclosing arm span required
  by the Rust normalization contract.
- `control-expressions.mlg` differentially covers nested `Some(Ok(...))`,
  generic qualified variants, both closure mutabilities and nested if branches
  under deterministic C, strict accounting and sanitizer execution.

## P175c3 Evidence

- The Mallang parser now separates global diagnostics from the current parse
  attempt, records one primary error per declaration or statement, suppresses
  exact duplicates and caps each source at 32 parse errors.
- Top-level recovery tracks parenthesis, brace and bracket depth; block recovery
  keeps nested function literals local while abandoning an unclosed block only
  at an unambiguous named declaration boundary.
- Recovery fixtures cover multiple declarations and statements, nested
  closures, missing block ends, receiver methods and the diagnostic cap. Their
  normalized errors match Rust Stage0 under strict C, allocation accounting and
  ASan/UBSan.

## Excluded

- semantic or ownership checking, which starts in B2
- typed IR and specialization
- C backend porting
- Stage1 compiler claims
- default compiler transition
