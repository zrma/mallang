# Spec: release-binary-cli-error-smoke

Status: complete; historical milestone record

## Objective

- Verify release-built CLI failure UX before v0 publication.
- Keep release artifact checks aligned with the debug CLI error stream smoke in
  `scripts/check.sh`.

## Scope

- Extend `scripts/check-release-binary.sh`.
- Run `target/release/mlg` without arguments and require stderr usage.
- Run `target/release/mlg nope` and require an unknown subcommand diagnostic.
- Require failure stdout to be empty.
- Share the same failure helper with release safety rejection smokes.

## Checklist

- [x] Add release no-args failure smoke.
- [x] Add release unknown-subcommand failure smoke.
- [x] Ensure release CLI failure stdout remains empty.
- [x] Record P100 in roadmaps and release notes.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-binary.sh` | release binary CLI error smoke |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate includes release CLI error smoke |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | non-publishing finalizer dry run remains green |
