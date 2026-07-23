# Spec: generated-c-deep-sanitizer

Status: complete; historical milestone record

## Objective

- Add a reusable deep generated C sanitizer sweep before v0 publication.
- Cover every successful example binary produced by `scripts/check.sh`, not only
  the focused cleanup-heavy sanitizer subset.

## Scope

- Add `scripts/check-generated-c-sanitizers.sh`.
- Derive the successful generated C labels from `scripts/check.sh` so the deep
  sweep follows the normal example smoke source of truth.
- Recompile each generated C file with AddressSanitizer and
  UndefinedBehaviorSanitizer.
- Run each sanitizer binary and compare stdout with the normal native binary.
- Keep this as an explicit heavy gate instead of adding it to default
  `scripts/check.sh`.

## Checklist

- [x] Add reusable deep generated C sanitizer script.
- [x] Support `--assume-generated` for fast local reruns after `scripts/check.sh`.
- [x] Compare sanitizer output against normal native output.
- [x] Document the command in the repo manifest and roadmap.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check-generated-c-sanitizers.sh --assume-generated` | validates all current normal generated C examples under ASan/UBSan |
| C2 | done | `scripts/check.sh` | normal v0 smoke gate remains green |
| C3 | done | `cargo test --all-targets` | full Rust regression suite remains green |
