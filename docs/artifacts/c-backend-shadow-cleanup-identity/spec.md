# C Backend Shadow Cleanup Identity

Status: complete; accepted on 2026-07-19

## Objective

- Preserve lexical binding identity when generated C emits cleanup for an outer
  value from inside a match arm or nested scope that shadows the same source
  name.
- Keep source-level shadowing valid without relying on self-hosted compiler
  source naming conventions.

## Evidence

- Pattern bindings use a deterministic C identifier derived from their arm span
  and retain an identity-keyed environment entry alongside the current lexical
  name mapping.
- Cleanup variable spans select the matching pattern identity. A cleanup for an
  outer ordinary binding bypasses an unrelated inner pattern mapping.

## Acceptance

- [x] Add IR/backend and native generated-C regressions for different-typed nested
  shadow bindings with outer cleanup on return and branch exit.
- [x] Carry binding identity through cleanup IR or an equivalent scoped C-name map;
  source names alone must not select cleanup targets.
- [x] Pass warning-clean generated C, allocation accounting and ASan/UBSan local
  gates. Repository publication and supported-platform CI are owned by the B2
  canonical acceptance change.
