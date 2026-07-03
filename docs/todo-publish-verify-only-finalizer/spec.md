# Spec: publish-verify-only-finalizer

## Objective

- Provide a publish-readiness command that runs the full v0 RC gate without
  changing the jj description, moving bookmarks, or pushing remotes.
- Keep `--no-push` available for the finalization flow that intentionally writes
  the final jj description but stops before bookmark movement and remote push.

## Scope

- Add `scripts/finalize-and-push.sh --verify-only`.
- Document the difference between verify-only and no-push finalization.
- Expose the verify-only command through README, handoff, release notes, and the
  repo manifest.

## Checklist

- [x] Add `--verify-only` to `scripts/finalize-and-push.sh`.
- [x] Reject `--verify-only` with `--message` or `--bookmark`.
- [x] Keep `--no-push` behavior unchanged for final description dry runs.
- [x] Record P89 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-helpers.sh` | release helper syntax and argument-contract gate |
| C2 | done | `scripts/finalize-and-push.sh --verify-only` | full v0 RC gate without publish side effects |
| C3 | done | `scripts/finalize-and-push.sh --verify-only --message "test: publish v0 release candidate"` | rejects message mutation in verify-only mode |
