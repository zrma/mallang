# Spec: no-push-freshness-dry-run

Status: complete; historical milestone record

## Objective

- Make the `--no-push` finalizer dry run exercise the same remote freshness
  checks as real publish while still stopping before bookmark movement and push.
- Preserve `--verify-only` as the side-effect-minimal readiness gate that does
  not mutate the jj description or run remote freshness checks.

## Scope

- Split the publish helper's push decision from its remote freshness decision.
- Run freshness preflight and final checks for real publish and `--no-push`.
- Keep `--verify-only` out of description mutation, bookmark movement, push, and
  freshness checks.
- Document the dry-run distinction.

## Checklist

- [x] Add a separate freshness-check flag in `scripts/finalize-and-push.sh`.
- [x] Run freshness checks for `--no-push`.
- [x] Keep `--verify-only` freshness-free.
- [x] Record P93 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-helpers.sh` | helper contract verifies freshness flag wiring |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate remains green |
| C3 | done | `scripts/finalize-and-push.sh --message "test: publish v0 release candidate" --no-push` | full finalization dry run exercises freshness checks without publish |
