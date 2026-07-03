# Spec: v0-release-candidate-audit

## Objective

- Record the current v0 release-candidate evidence in a durable agent-readable
  form.
- Keep every checked-in example source connected to `scripts/check.sh` so new
  examples cannot silently drift outside the native smoke gate.
- Separate completed v0 scope from intentionally deferred post-v0 language
  features.

## Scope

- Add a smoke-coverage guard for `examples/*.mlg` in `scripts/check.sh`.
- Record the audit result in roadmap and handoff docs.
- Do not add new language syntax or expand deferred v0 boundaries in this slice.

## Evidence Snapshot

| Area | Status | Evidence |
| --- | --- | --- |
| Naming and CLI | done | `mlg check`, `mlg ir`, `mlg build`, `mlg run`, `mlg --version`, `mlg --help` |
| Syntax frontend | done | `ROADMAP.md` Milestone 1 and parser tests |
| Static semantics | done | `ROADMAP.md` Milestone 2 and semantic tests |
| Ownership and borrowing | done for v0 | `con`/`mut` call modes, same-call overlap checks, move/use-after-move regressions |
| Functional value style | done for v0 | `if` expressions, `match`, `Option`, `Result`, pipeline call sugar |
| Native backend | release-candidate | C backend covers all checked-in examples and runtime failure smoke |
| Deferred boundaries | explicit | first-class references, statement-spanning borrows, general non-slice partial moves, interfaces, closures, modules |

## Checklist

- [x] Confirm all `examples/*.mlg` are referenced by `scripts/check.sh`.
- [x] Add an automatic missing-example smoke coverage guard.
- [x] Mark v0 release-candidate audit in `ROADMAP.md`.
- [x] Mark P81 in `docs/ROADMAP.md`.
- [x] Update `docs/HANDOFF.md` next-action guidance.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | full local gate plus example coverage guard |
| C2 | done | `cargo test --all-targets` | unit and binary tests |
| C3 | done | `cargo clippy --all-targets -- -D warnings` | lint gate |
