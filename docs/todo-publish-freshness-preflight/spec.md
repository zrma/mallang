# Spec: publish-freshness-preflight

## Objective

- Fail fast on stale remote state before the finalizer mutates the local jj
  description or runs the expensive v0 RC verification gate.
- Keep the final remote freshness check immediately before bookmark movement so
  remote movement during verification is still caught.

## Scope

- Run a real-publish-only freshness preflight before `describe_with_attribution`.
- Keep the existing final freshness guard before `jj bookmark set`.
- Include phase-specific diagnostics for preflight and final freshness failures.
- Extend release helper contract checks to verify both freshness calls are wired.

## Checklist

- [x] Add preflight freshness check before description mutation.
- [x] Keep final freshness check before bookmark movement.
- [x] Keep `--verify-only` and `--no-push` out of the remote push path.
- [x] Record P92 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-helpers.sh` | helper contract verifies preflight/final freshness wiring |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate remains green |
| C3 | done | `scripts/finalize-and-push.sh --verify-only` | full readiness gate remains non-publishing |
