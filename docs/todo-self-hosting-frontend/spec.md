# Spec: B1 Self-Hosting Frontend

Status: active; P175a-P175b and P175c1 complete, P175c2 parser body work next

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
- **P175c2**: implement statement and expression parsing, then add bounded
  recovery without changing the frozen grammar
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
- [ ] Mallang frontend AST and complete parser
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

## Excluded

- semantic or ownership checking, which starts in B2
- typed IR and specialization
- C backend porting
- Stage1 compiler claims
- default compiler transition
