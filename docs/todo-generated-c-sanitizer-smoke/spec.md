# Spec: generated-c-sanitizer-smoke

Status: complete; historical milestone record

## Objective

- Add a focused generated C memory/lifetime smoke for cleanup-heavy v0 examples.
- Exercise owned slice field cleanup, field take, append-take, and indexed slice
  field append paths under AddressSanitizer and UndefinedBehaviorSanitizer.

## Scope

- Reuse the `.c` files emitted by normal `mlg build` smoke steps.
- Compile selected generated C files with `clang -fsanitize=address,undefined`.
- Assert expected stdout and empty sanitizer stderr.
- Do not require a new Mallang CLI flag for sanitizer compilation in this
  slice.

## Selected Programs

| Program | Why |
| --- | --- |
| `struct-slice-field.mlg` | struct cleanup with owned slice fields |
| `slice-field-take.mlg` | owned slice field take leaves source empty |
| `slice-field-take-append.mlg` | append source takes a slice field and preserves cleanup ownership |
| `indexed-slice-field-append.mlg` | stable indexed slice field append reassignment |

## Checklist

- [x] Add sanitizer compile/run helper to `scripts/check.sh`.
- [x] Recompile selected generated C files with ASan/UBSan.
- [x] Require expected stdout and empty sanitizer stderr.
- [x] Record P82 in roadmap and handoff docs.

## Acceptance

| ID | Status | Command | Notes |
| --- | --- | --- | --- |
| C1 | done | `scripts/check.sh` | includes sanitizer smoke after normal generated C builds |
| C2 | done | `cargo test --all-targets` | regression unit coverage still green |
| C3 | done | `cargo clippy --all-targets -- -D warnings` | lint gate |
