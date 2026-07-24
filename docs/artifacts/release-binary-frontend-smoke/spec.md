# Spec: release-binary-frontend-smoke

Status: complete; historical milestone record

## Objective

- Verify every read-only frontend/IR CLI command on the release-built `mlg`
  binary before v0 publication.
- Keep release artifact coverage aligned with the CLI surface listed in the v0
  release-candidate notes.

## Scope

- Extend `scripts/check-release-binary.sh`.
- Run `target/release/mlg lex examples/first.mlg`.
- Run `target/release/mlg parse examples/first.mlg`.
- Run `target/release/mlg ir examples/first.mlg`.
- Assert stable structural output markers rather than the full debug output.

## Checklist

- [x] Add release binary `lex` smoke.
- [x] Add release binary `parse` smoke.
- [x] Add release binary `ir` smoke.
- [x] Record P98 in roadmaps and release notes.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-binary.sh` | release binary frontend/IR smoke |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate includes frontend/IR release smoke |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | non-publishing finalizer dry run remains green |
