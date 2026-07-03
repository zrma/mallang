# Spec: release-binary-build-arg-errors

## Objective

- Verify release-built `mlg build` rejects malformed build arguments before v0
  publication.
- Keep the native backend's primary user-facing command covered for both
  success and failure paths.

## Scope

- Extend `scripts/check-release-binary.sh`.
- Run `target/release/mlg build examples/first.mlg -o` and require the missing
  output diagnostic.
- Run `target/release/mlg build examples/first.mlg --wat` and require the
  unknown build argument diagnostic.
- Require failure stdout to be empty.

## Checklist

- [x] Add release `build -o` missing value smoke.
- [x] Add release unknown build argument smoke.
- [x] Ensure release build argument failure stdout remains empty.
- [x] Record P101 in roadmaps and release notes.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-binary.sh` | release binary build argument error smoke |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate includes release build argument error smoke |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | non-publishing finalizer dry run remains green |
