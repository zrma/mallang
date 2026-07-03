# Spec: finalizer-option-value-diagnostics

## Objective

- Make missing finalizer option values fail with explicit diagnostics instead of
  shell `shift` failures.
- Keep invalid values distinct from invalid message format checks.

## Scope

- Validate `--message` has a non-empty value before shifting.
- Validate `--bookmark` has a non-empty value before shifting.
- Reject accidental next-option tokens as missing values for options that require
  values.
- Add release helper contract coverage for missing values, empty values, and
  next-option tokens accidentally passed as values.

## Checklist

- [x] Add explicit value checks for `--message`.
- [x] Add explicit value checks for `--bookmark`.
- [x] Add contract tests for missing, empty, and next-option message/bookmark values.
- [x] Record P94 in roadmap docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-release-helpers.sh` | helper contract covers missing option values |
| C2 | done | `scripts/verify-v0-rc.sh --skip-deep-sanitizers` | v0 RC gate remains green |
| C3 | done | `scripts/finalize-and-push.sh --message` | exits 2 with explicit diagnostic |
