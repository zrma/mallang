# Spec: release-binary-run-smoke

## Objective

- Verify the release-built CLI `mlg run` path before v0 publication.
- Keep release binary coverage from relying only on `mlg build` followed by
  manual native binary execution.

## Scope

- Extend `scripts/check-release-binary.sh`.
- Run `target/release/mlg run examples/first.mlg`.
- Require stdout to remain `30`.
- Reuse the existing `scripts/verify-v0-rc.sh` release binary gate.

## Checklist

- [x] Add release binary `mlg run` smoke.
- [x] Keep release native `build`/binary smoke intact.
- [x] Record P97 in roadmaps and release notes.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-binary.sh` | release binary `mlg run` smoke |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate includes release binary run smoke |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | non-publishing finalizer dry run remains green |
