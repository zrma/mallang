# C Backend Shadow Cleanup Identity

Status: pending

## Objective

- Preserve lexical binding identity when generated C emits cleanup for an outer
  value from inside a match arm or nested scope that shadows the same source
  name.
- Keep source-level shadowing valid without relying on self-hosted compiler
  source naming conventions.

## Evidence

- A cleanup-bearing outer string and an integer match-arm binding with the same
  source name can currently map to one C identifier. Cleanup emitted inside the
  arm then resolves to the inner C binding and produces an invalid drop call.
- The self-hosted compiler avoids the collision with distinct arm binding names,
  but that workaround is not a language or backend contract.

## Acceptance

- Add IR/backend and native generated-C regressions for different-typed nested
  shadow bindings with outer cleanup on return and branch exit.
- Carry binding identity through cleanup IR or an equivalent scoped C-name map;
  source names alone must not select cleanup targets.
- Pass warning-clean generated C, allocation accounting, ASan/UBSan, repository
  publication gates and supported-platform CI.
