# Spec: release-binary-safety-rejections

## Objective

- Verify the release-built CLI rejects representative v0 safety violations
  before publication.
- Keep release artifact coverage aligned with Mallang's Rust-like safety claim,
  not only successful compilation and execution paths.

## Scope

- Extend `scripts/check-release-binary.sh`.
- Generate temporary negative sources under `target/mallang`.
- Require `target/release/mlg check` to fail for:
  - use-after-move of a move-only `string`;
  - moving a borrowed non-copy parameter out of a function;
  - overlapping `mut` and `con` borrows in the same call.
- Require failure stdout to be empty and stderr to contain stable diagnostics.

## Checklist

- [x] Add release use-after-move failure smoke.
- [x] Add release borrowed non-copy escape failure smoke.
- [x] Add release overlapping borrow failure smoke.
- [x] Record P99 in roadmaps and release notes.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-binary.sh` | release binary safety rejection smoke |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate includes release safety rejection smoke |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | non-publishing finalizer dry run remains green |
