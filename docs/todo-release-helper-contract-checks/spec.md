# Spec: release-helper-contract-checks

## Objective

- Keep the publish helper argument contract under automated local verification.
- Catch accidental release helper regressions before the heavier v0 RC gate moves
  into compiler and generated C checks.

## Scope

- Add a lightweight `scripts/check-release-helpers.sh` command.
- Validate shell syntax for local project scripts.
- Validate `scripts/finalize-and-push.sh` help output and rejected argument
  combinations that must not mutate jj state or remotes.
- Validate that real fetch/push invocations stay routed through the Git-version
  fallback wrapper.
- Wire the release helper check into `scripts/verify-v0-rc.sh`.

## Checklist

- [x] Add `scripts/check-release-helpers.sh`.
- [x] Check finalize helper help output.
- [x] Check finalize helper fetch/push wrapper wiring.
- [x] Check invalid `--verify-only` combinations.
- [x] Check invalid publish message and unknown option handling.
- [x] Run release helper checks from `scripts/verify-v0-rc.sh`.
- [x] Record P90 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-helpers.sh` | lightweight release helper contract gate, including per-script shell syntax |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate includes helper checks |
| C3 | done | `bash -n scripts/check-release-helpers.sh` | direct shell syntax gate for the contract checker |
