# Spec: generated-c-full-warning-clean

## Objective

- Make every successful generated C example in the normal smoke gate compile
  cleanly under strict clang warnings.
- Promote the P83 representative warning-clean smoke to the full default
  `scripts/check.sh` gate.

## Scope

- Treat `scripts/check.sh` normal build labels as the source of truth.
- Compile every corresponding `target/mallang/<label>.c` with
  `clang -std=c11 -Wall -Wextra -Werror`.
- Mark generated drop helpers as maybe-unused because helper emission is
  conservative.
- Mark range source/value temporaries as intentionally used when the source
  program does not read them.

## Checklist

- [x] Add full generated C warning-clean loop to `scripts/check.sh`.
- [x] Mark generated drop helpers with `MLG_UNUSED`.
- [x] Suppress unused range source/value temps without reading uninitialized
  zero-length range values.
- [x] Record P85 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | includes full generated C warning-clean sweep |
| C2 | done | `cargo test --all-targets` | backend output regression coverage remains green |
| C3 | done | `cargo clippy --all-targets -- -D warnings` | Rust lint gate remains green |
