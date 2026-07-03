# Spec: generated-c-warning-clean

## Objective

- Keep representative generated C output clean under `clang -std=c11 -Wall
  -Wextra -Werror`.
- Remove avoidable warning sources from the C backend before v0 publication.

## Scope

- Mark always-emitted runtime helpers as intentionally maybe-unused in generated
  C.
- Emit `(void)param;` at function entry so source-level unused parameters do not
  become C warnings.
- Emit `for` post labels only when a loop body can continue to the post block.
- Add a warning-clean generated C smoke to `scripts/check.sh`.

## Selected Programs

| Program | Warning class covered |
| --- | --- |
| `adt.mlg` | unused runtime helpers in programs with no index/runtime guard use |
| `arrays.mlg` | source-level unused owned parameter lowering |
| `array-for-post.mlg` | post block without unnecessary label |
| `slice-field-take.mlg` | cleanup-heavy generated C under strict warnings |

## Checklist

- [x] Add `MLG_UNUSED` helper annotation macro in generated C.
- [x] Emit unused parameter casts for generated function parameters.
- [x] Avoid unused `for` post labels when no outer `continue` targets them.
- [x] Preserve `continue` to post lowering when a post loop body contains
  `continue`.
- [x] Add strict generated C warning smoke.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `cargo test --all-targets` | backend warning-hygiene regressions |
| C2 | done | `cargo clippy --all-targets -- -D warnings` | Rust lint gate |
| C3 | done | `scripts/check.sh` | warning-clean generated C smoke included |
